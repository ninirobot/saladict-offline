//! 全局快捷键（RegisterHotKey）。
//!
//! 独立线程持有热键 + 消息泵，配置变更时用 PostThreadMessage 唤醒线程重新绑定，
//! 这样按键响应是即时的，不需要轮询。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

const WM_APP_REBIND: u32 = 0x8000 + 1;
const HOTKEY_ID: i32 = 0x5A11;

const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const MOD_WIN: u32 = 0x0008;
const MOD_NOREPEAT: u32 = 0x4000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Spec {
    pub mods: u32,
    pub vk: u32,
}

static TID: AtomicU32 = AtomicU32::new(0);
static SPEC: OnceLock<Mutex<Option<Spec>>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();

/// 依据当前配置重新绑定（或在关闭时注销）热键
pub fn sync(app: &AppHandle) {
    let cfg = app.state::<AppState>().config_snapshot();
    let spec = if cfg.selection.hotkey {
        parse_combo(&cfg.selection.hotkey_combo)
    } else {
        None
    };
    if let Ok(mut lock) = SPEC.get_or_init(|| Mutex::new(None)).lock() {
        *lock = spec;
    }

    if STARTED.set(()).is_err() {
        let tid = TID.load(Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    tid,
                    WM_APP_REBIND,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        }
        return;
    }

    let _ = APP.set(app.clone());
    std::thread::spawn(thread_main);
}

fn current_spec() -> Option<Spec> {
    SPEC.get().and_then(|m| m.lock().ok().and_then(|g| *g))
}

#[cfg(windows)]
fn thread_main() {
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG};

    unsafe {
        TID.store(GetCurrentThreadId(), Ordering::Relaxed);
    }
    let mut applied: Option<Option<Spec>> = None;
    apply(&mut applied);

    let mut msg = MSG::default();
    loop {
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if ret.0 <= 0 {
            break;
        }
        if msg.message == WM_APP_REBIND {
            apply(&mut applied);
            continue;
        }
        if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_HOTKEY {
            log::info!("hotkey fired");
            if let Some(app) = APP.get() {
                crate::state::capture_and_show(app, "hotkey");
            }
        }
    }
}

#[cfg(not(windows))]
fn thread_main() {}

#[cfg(windows)]
fn apply(applied: &mut Option<Option<Spec>>) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
    };

    let want = current_spec();
    if applied.as_ref() == Some(&want) {
        return;
    }
    unsafe {
        let _ = UnregisterHotKey(None, HOTKEY_ID);
        if let Some(spec) = want {
            match RegisterHotKey(None, HOTKEY_ID, HOT_KEY_MODIFIERS(spec.mods), spec.vk) {
                Ok(()) => log::info!("hotkey bound: {:#x}/{:#x}", spec.mods, spec.vk),
                Err(err) => {
                    log::warn!("hotkey bind failed: {err}");
                    if let Some(app) = APP.get() {
                        let _ = app.emit("hotkey-error", format!("快捷键被占用：{err}"));
                    }
                }
            }
        }
    }
    *applied = Some(want);
}

#[cfg(not(windows))]
fn apply(_applied: &mut Option<Option<Spec>>) {}

/// "Ctrl+Alt+D" → mods + 虚拟键码
pub fn parse_combo(combo: &str) -> Option<Spec> {
    let mut mods = MOD_NOREPEAT;
    let mut vk: Option<u32> = None;
    for part in combo.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt" => mods |= MOD_ALT,
            "shift" => mods |= MOD_SHIFT,
            "win" | "super" | "meta" => mods |= MOD_WIN,
            other => vk = parse_vk(other),
        }
    }
    let vk = vk?;
    if mods & (MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_WIN) == 0 {
        return None;
    }
    Some(Spec { mods, vk })
}

fn parse_vk(token: &str) -> Option<u32> {
    if token.len() == 1 {
        let ch = token.chars().next()?.to_ascii_uppercase();
        if ch.is_ascii_alphanumeric() {
            return Some(ch as u32);
        }
    }
    if let Some(rest) = token.strip_prefix('f') {
        if let Ok(num) = rest.parse::<u32>() {
            if (1..=24).contains(&num) {
                return Some(0x70 + num - 1);
            }
        }
    }
    let vk = match token {
        "space" => 0x20,
        "enter" | "return" => 0x0D,
        "tab" => 0x09,
        "esc" | "escape" => 0x1B,
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "`" | "grave" => 0xC0,
        "insert" => 0x2D,
        _ => return None,
    };
    Some(vk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_combo() {
        let spec = parse_combo("Ctrl+Alt+D").expect("should parse");
        assert_eq!(spec.vk, 'D' as u32);
        assert!(spec.mods & MOD_CONTROL != 0);
        assert!(spec.mods & MOD_ALT != 0);
    }

    #[test]
    fn rejects_modifier_only() {
        assert!(parse_combo("Ctrl+Alt").is_none());
        assert!(parse_combo("").is_none());
        assert!(parse_combo("D").is_none());
    }

    #[test]
    fn parses_function_keys() {
        assert_eq!(parse_combo("Ctrl+F5").unwrap().vk, 0x74);
    }
}
