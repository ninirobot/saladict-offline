//! 取词：UIA 优先，失败时用 Ctrl+C 兜底，并还原用户原有剪贴板
//! （只备份/还原纯文本，不动图片等其它格式）。

use std::time::Duration;

use super::uia;

/// 读取当前选中文本
pub fn selected_text() -> Option<String> {
    if let Some(text) = uia::selected_text() {
        let text = text.trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    clipboard_fallback()
}

#[cfg(windows)]
fn clipboard_fallback() -> Option<String> {
    use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

    let backup = read_clipboard_text();
    let before = unsafe { GetClipboardSequenceNumber() };
    send_ctrl_c();

    let mut text = None;
    for _ in 0..14 {
        std::thread::sleep(Duration::from_millis(14));
        let now = unsafe { GetClipboardSequenceNumber() };
        if now != before {
            text = read_clipboard_text();
            break;
        }
    }

    if let Some(backup) = backup {
        // 还原用户原来的剪贴板内容；失败就算了，不能因此丢掉查询结果
        write_clipboard_text(&backup);
    }
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

#[cfg(not(windows))]
fn clipboard_fallback() -> Option<String> {
    None
}

/// 直接读剪贴板（托盘「剪贴板取词」用）
pub fn clipboard_text() -> Option<String> {
    #[cfg(windows)]
    {
        read_clipboard_text()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn read_clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{GetClipboardData, IsClipboardFormatAvailable, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    unsafe {
        if !IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok() {
            return None;
        }
        if OpenClipboard(None).is_err() {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT.0 as u32);
        let mut out = None;
        if let Ok(handle) = handle {
            let hglobal = HGLOBAL(handle.0 as _);
            let ptr = GlobalLock(hglobal) as *const u16;
            if !ptr.is_null() {
                let mut len = 0usize;
                // 上限 4MB，避免异常数据把界面拖死
                while len < 2 * 1024 * 1024 && *ptr.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(ptr, len);
                out = Some(String::from_utf16_lossy(slice));
                let _ = GlobalUnlock(hglobal);
            }
        }
        let _ = windows::Win32::System::DataExchange::CloseClipboard();
        out
    }
}

#[cfg(windows)]
fn write_clipboard_text(text: &str) -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    unsafe {
        if OpenClipboard(None).is_err() {
            return false;
        }
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = wide.len() * 2;
        let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, bytes) else {
            let _ = windows::Win32::System::DataExchange::CloseClipboard();
            return false;
        };
        let ptr = GlobalLock(hmem) as *mut u16;
        if !ptr.is_null() {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            let _ = GlobalUnlock(hmem);
        }
        let _ = EmptyClipboard();
        let ok = SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(hmem.0 as _))).is_ok();
        let _ = windows::Win32::System::DataExchange::CloseClipboard();
        ok
    }
}

/// 模拟一次 Ctrl+C；若用户正按着 Ctrl（拖选时常见），只发 C
#[cfg(windows)]
fn send_ctrl_c() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, VK_C, VK_CONTROL,
    };

    fn key(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    unsafe {
        let ctrl_down = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
        let mut inputs: Vec<INPUT> = Vec::with_capacity(4);
        if !ctrl_down {
            inputs.push(key(VK_CONTROL, false));
        }
        inputs.push(key(VK_C, false));
        inputs.push(key(VK_C, true));
        if !ctrl_down {
            inputs.push(key(VK_CONTROL, true));
        }
        if !inputs.is_empty() {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}
