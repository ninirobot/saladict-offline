//! 全局鼠标拖选钩子（WH_MOUSE_LL）。
//!
//! 约束：钩子回调必须极快，否则会拖慢整机鼠标。这里只记录坐标、判断位移，
//! 真正的取词丢给独立工作线程，并做 80ms 延迟等待操作系统把选区稳定下来。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::AppHandle;

pub struct HookCtx {
    pub paused: AtomicBool,
    pub app: AppHandle,
    down: Mutex<Option<Down>>,
}

struct Down {
    x: i32,
    y: i32,
    at: Instant,
}

static CTX: OnceLock<Arc<HookCtx>> = OnceLock::new();
static WAKE: OnceLock<mpsc::Sender<()>> = OnceLock::new();

pub fn set_paused(paused: bool) {
    if let Some(ctx) = CTX.get() {
        ctx.paused.store(paused, Ordering::Relaxed);
    }
}

pub fn is_paused() -> bool {
    CTX.get()
        .map(|c| c.paused.load(Ordering::Relaxed))
        .unwrap_or(false)
}

pub fn install(app: AppHandle) {
    let ctx = Arc::new(HookCtx {
        paused: AtomicBool::new(false),
        app: app.clone(),
        down: Mutex::new(None),
    });
    if CTX.set(ctx).is_err() {
        return;
    }

    let (tx, rx) = mpsc::channel::<()>();
    if WAKE.set(tx).is_err() {
        return;
    }

    // 取词工作线程：把 UI 线程与钩子回调彻底隔离
    let app_worker = app.clone();
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            // 合并短时间内的重复触发（双击会产生两次）
            while rx.recv_timeout(Duration::from_millis(120)).is_ok() {}
            std::thread::sleep(Duration::from_millis(80));
            crate::state::capture_and_show(&app_worker, "drag");
        }
    });

    std::thread::spawn(move || unsafe {
        use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
            MSG, WH_MOUSE_LL,
        };

        let module = GetModuleHandleW(None).ok();
        let hinstance = module.map(|m| HINSTANCE(m.0));
        let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hinstance, 0) {
            Ok(hook) => hook,
            Err(err) => {
                log::error!("install mouse hook failed: {err}");
                return;
            }
        };
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(hook);
        let _ = (std::mem::size_of::<WPARAM>(), std::mem::size_of::<LPARAM>(), std::mem::size_of::<LRESULT>());
    });
}

fn schedule() {
    if let Some(tx) = WAKE.get() {
        let _ = tx.send(());
    }
}

#[cfg(windows)]
unsafe extern "system" fn mouse_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, MSLLHOOKSTRUCT, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    };

    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    let Some(ctx) = CTX.get() else {
        return CallNextHookEx(None, code, wparam, lparam);
    };
    if ctx.paused.load(Ordering::Relaxed) {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let msg = wparam.0 as u32;
    let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);

    match msg {
        WM_LBUTTONDOWN => {
            if let Ok(mut guard) = ctx.down.lock() {
                *guard = Some(Down {
                    x: info.pt.x,
                    y: info.pt.y,
                    at: Instant::now(),
                });
            }
            crate::win::popup::hide_if_outside(&ctx.app, info.pt.x, info.pt.y);
        }
        WM_LBUTTONUP => {
            let start = ctx.down.lock().ok().and_then(|mut g| g.take());
            if let Some(start) = start {
                let dx = (info.pt.x - start.x) as f64;
                let dy = (info.pt.y - start.y) as f64;
                if dx * dx + dy * dy > 16.0
                    && start.at.elapsed() < Duration::from_secs(4)
                    && !foreground_is_self()
                {
                    schedule();
                }
            }
        }
        WM_LBUTTONDBLCLK if !foreground_is_self() => schedule(),
        _ => {}
    }

    CallNextHookEx(None, code, wparam, lparam)
}

/// 点击落在自己的浮窗/主窗口里时不应再次触发查询
#[cfg(windows)]
fn foreground_is_self() -> bool {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn foreground_is_self() -> bool {
    false
}
