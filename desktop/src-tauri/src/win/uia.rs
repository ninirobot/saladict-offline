//! UI Automation 取词：优先走非侵入式的 TextPattern，
//! 这样不用碰剪贴板，也不会干扰用户正在复制的其它内容。

#[cfg(windows)]
pub fn selected_text() -> Option<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, IUIAutomationElement};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;

        let mut candidates: Vec<IUIAutomationElement> = Vec::with_capacity(2);
        if let Ok(el) = automation.GetFocusedElement() {
            candidates.push(el);
        }
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            if let Ok(el) = automation.ElementFromHandle(hwnd) {
                candidates.push(el);
            }
        }

        for element in candidates {
            if let Some(text) = text_of(&element) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }
}

#[cfg(windows)]
fn text_of(element: &windows::Win32::UI::Accessibility::IUIAutomationElement) -> Option<String> {
    use windows::Win32::UI::Accessibility::{IUIAutomationTextPattern, UIA_TextPatternId};

    unsafe {
        let pattern: IUIAutomationTextPattern =
            element.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
        let ranges = pattern.GetSelection().ok()?;
        let count = ranges.Length().ok()?;
        if count <= 0 {
            return None;
        }
        let mut out = String::new();
        for index in 0..count {
            if let Ok(range) = ranges.GetElement(index) {
                if let Ok(text) = range.GetText(-1) {
                    out.push_str(&text.to_string());
                }
            }
        }
        if out.trim().is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

#[cfg(not(windows))]
pub fn selected_text() -> Option<String> {
    None
}
