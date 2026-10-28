//! 系统集成：文件选择对话框 + 用默认浏览器打开链接。
//! 直接调 Win32，省掉 dialog / opener 两个插件。

/// 弹出「选择 mdx 文件」对话框
#[cfg(windows)]
pub fn pick_mdx() -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    let mut buffer = [0u16; 1024];
    let filter: Vec<u16> = "MDict 词典 (*.mdx)\0*.mdx\0所有文件 (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();

    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: windows::core::PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
        ..Default::default()
    };

    unsafe {
        if GetOpenFileNameW(&mut dialog).as_bool() {
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            return Some(String::from_utf16_lossy(&buffer[..len]));
        }
    }
    None
}

#[cfg(not(windows))]
pub fn pick_mdx() -> Option<String> {
    None
}

#[cfg(windows)]
pub fn open_url(url: &str) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return false;
    }
    let target = HSTRING::from(url);
    unsafe {
        let handle = ShellExecuteW(
            None,
            windows::core::w!("open"),
            &target,
            None,
            None,
            SW_SHOWNORMAL,
        );
        handle.0 as usize > 32
    }
}

#[cfg(not(windows))]
pub fn open_url(_url: &str) -> bool {
    false
}
