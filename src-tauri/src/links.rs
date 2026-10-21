pub fn checked_url(input: &str) -> Result<tauri::Url, String> {
    let url = tauri::Url::parse(input).map_err(|_| "Indirizzo non valido")?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Si possono aprire solo indirizzi HTTP/HTTPS senza credenziali.".into());
    }
    Ok(url)
}

pub fn open(input: &str) -> Result<(), String> {
    let url = checked_url(input)?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let target: Vec<u16> = std::ffi::OsStr::new(url.as_str())
            .encode_wide()
            .chain(Some(0))
            .collect();
        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        // SAFETY: Both UTF-16 strings are NUL-terminated and outlive this call.
        // Only a validated http(s) URL is passed as lpFile, with no parameters.
        let result = unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result as isize <= 32 {
            return Err("Impossibile aprire il browser di sistema.".into());
        }
    }
    #[cfg(not(windows))]
    {
        let executable = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(executable)
            .arg(url.as_str())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_links_exclude_commands_and_credentials() {
        assert!(checked_url("https://example.com/path?q=hello").is_ok());
        for url in [
            "javascript:alert(1)",
            "file:///C:/secret",
            "cmd://run",
            "https://user:pass@example.com",
            "data:text/html,hello",
        ] {
            assert!(checked_url(url).is_err());
        }
    }
}
