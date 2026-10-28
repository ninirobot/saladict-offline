use std::sync::atomic::Ordering;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::state::AppState;

pub const LABEL: &str = "popup";

use crate::state::PopupRequest;

/// 浮窗懒创建：首次划词才建 WebView，之后只做 show/hide + 重新定位
pub fn ensure(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(win) = app.get_webview_window(LABEL) {
        return Ok(win);
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("沙拉划词")
        .inner_size(388.0, 420.0)
        .min_inner_size(320.0, 160.0)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()
}

pub fn show_for(app: &AppHandle, word: &str, source: &str, pin: bool) {
    let Ok(win) = ensure(app) else {
        return;
    };
    let state = app.state::<AppState>();
    let width = state.config_snapshot().ui.popup_width.clamp(300, 640) as f64;
    let pinned = pin || state.pinned.load(Ordering::Relaxed);
    state.pinned.store(pinned, Ordering::Relaxed);
    place(app, &win, width, 420.0);
    let _ = win.show();

    let seq = state.popup_seq.fetch_add(1, Ordering::Relaxed) + 1;
    let payload = PopupRequest {
        word: word.to_string(),
        source: source.to_string(),
        pinned,
        seq,
    };
    if let Ok(mut slot) = state.pending_popup.lock() {
        *slot = Some(payload.clone());
    }
    let _ = app.emit_to(LABEL, "popup-show", payload);
}

/// 浮窗高度自适应：前端渲染完成后回报内容高度（CSS 像素）
pub fn resize(app: &AppHandle, height: f64) {
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let scale = cursor_scale(app);
    let width = app.state::<AppState>().config_snapshot().ui.popup_width;
    let h = (height.clamp(120.0, 620.0) * scale).round() as u32;
    let w = (width as f64 * scale).round() as u32;
    let _ = win.set_size(PhysicalSize::new(w, h));

    // 变高之后重新夹取位置，避免内容变长后掉出屏幕
    if let Ok(pos) = win.outer_position() {
        let (nx, ny) = clamp_to_work_area(pos.x, pos.y, w as i32, h as i32);
        let _ = win.set_position(PhysicalPosition::new(nx, ny));
    }
}

/// 鼠标在浮窗外按下时收起浮窗（无边框窗口不一定能拿到失焦事件）
pub fn hide_if_outside(app: &AppHandle, x: i32, y: i32) {
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let state = app.state::<AppState>();
    if state.pinned.load(Ordering::Relaxed) {
        return;
    }
    if !win.is_visible().unwrap_or(false) {
        return;
    }
    if let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) {
        let inside = x >= pos.x
            && y >= pos.y
            && x < pos.x + size.width as i32
            && y < pos.y + size.height as i32;
        if !inside {
            let _ = win.hide();
        }
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.hide();
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn place(app: &AppHandle, win: &WebviewWindow, width_logical: f64, height_logical: f64) {
    let scale = cursor_scale(app);
    let w = (width_logical * scale).round() as i32;
    let h = (height_logical * scale).round() as i32;
    let (cx, cy) = cursor_pos();
    let gap_x = (14.0 * scale).round() as i32;
    let gap_y = (18.0 * scale).round() as i32;

    let mut x = cx + gap_x;
    let mut y = cy + gap_y;
    let (work_x, work_y, work_w, work_h) = work_area(cx, cy);
    if x + w > work_x + work_w {
        x = cx - w - gap_x;
    }
    if y + h > work_y + work_h {
        y = cy - h - gap_y;
    }
    let (x, y) = clamp_to_work_area(x, y, w, h);
    log::info!(
        "popup place: cursor=({cx},{cy}) scale={scale} size={w}x{h} pos=({x},{y}) work=({work_x},{work_y},{work_w},{work_h})"
    );

    let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// 把窗口夹进当前显示器工作区（物理像素）
fn clamp_to_work_area(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let (work_x, work_y, work_w, work_h) = work_area(x + w / 2, y + h / 2);
    let min_x = work_x + 4;
    let min_y = work_y + 4;
    let max_x = work_x + work_w - w - 4;
    let max_y = work_y + work_h - h - 4;
    (
        if max_x > min_x { x.clamp(min_x, max_x) } else { min_x },
        if max_y > min_y { y.clamp(min_y, max_y) } else { min_y },
    )
}

/// 光标所在显示器的缩放比：创建窗口时 win.scale_factor() 可能还没跟上，
/// 因此直接按光标位置匹配显示器。
fn cursor_scale(app: &AppHandle) -> f64 {
    let (cx, cy) = cursor_pos();
    if let Ok(monitors) = app.available_monitors() {
        for monitor in monitors {
            let pos = monitor.position();
            let size = monitor.size();
            if cx >= pos.x
                && cy >= pos.y
                && cx < pos.x + size.width as i32
                && cy < pos.y + size.height as i32
            {
                return monitor.scale_factor();
            }
        }
    }
    1.0
}

#[cfg(windows)]
fn cursor_pos() -> (i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            (pt.x, pt.y)
        } else {
            (0, 0)
        }
    }
}

#[cfg(not(windows))]
fn cursor_pos() -> (i32, i32) {
    (0, 0)
}

/// 当前显示器工作区（物理像素，已排除任务栏）
#[cfg(windows)]
fn work_area(x: i32, y: i32) -> (i32, i32, i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let r = info.rcWork;
            (r.left, r.top, r.right - r.left, r.bottom - r.top)
        } else {
            (0, 0, 1920, 1080)
        }
    }
}

#[cfg(not(windows))]
fn work_area(_x: i32, _y: i32) -> (i32, i32, i32, i32) {
    (0, 0, 1920, 1080)
}
