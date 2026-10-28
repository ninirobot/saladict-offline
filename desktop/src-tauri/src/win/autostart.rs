//! 开机自启：写 HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! 只写一个值，不引入额外插件。

#[cfg(windows)]
pub fn set(enabled: bool) -> Result<(), String> {
    use windows::core::{w, HSTRING};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_SZ,
    };

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let value = HSTRING::from(exe.to_string_lossy().to_string());

    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            KEY_SET_VALUE,
            &mut key,
        )
        .ok()
        .map_err(|e| e.to_string())?;

        let result = if enabled {
            let bytes = std::slice::from_raw_parts(
                value.as_ptr() as *const u8,
                value.len() * std::mem::size_of::<u16>(),
            );
            RegSetValueExW(key, w!("SaladictLite"), None, REG_SZ, Some(bytes))
                .ok()
                .map_err(|e| e.to_string())
        } else {
            let _ = RegDeleteValueW(key, w!("SaladictLite"));
            Ok(())
        };
        let _ = RegCloseKey(key);
        result
    }
}

#[cfg(not(windows))]
pub fn set(_enabled: bool) -> Result<(), String> {
    Err("仅支持 Windows".into())
}
