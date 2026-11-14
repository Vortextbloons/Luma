#[cfg(windows)]
pub fn running() -> bool {
    use windows::{
        Win32::{
            Foundation::CloseHandle,
            System::Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
        },
    };
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return false;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = Process32FirstW(snapshot, &mut entry).is_ok();
        while found {
            let name = String::from_utf16_lossy(&entry.szExeFile);
            if name
                .trim_end_matches('\0')
                .eq_ignore_ascii_case("Spotify.exe")
            {
                let _ = CloseHandle(snapshot);
                return true;
            }
            found = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        false
    }
}

#[cfg(not(windows))]
pub fn running() -> bool {
    false
}

pub fn installed() -> bool {
    crate::commands::apps::detect("Spotify").is_some()
}

#[cfg(windows)]
pub fn launch_minimized() -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWMINNOACTIVE},
    };
    let executable =
        crate::commands::apps::detect("Spotify").ok_or("Spotify is not installed or its path is unknown.")?;
    let directory = executable.parent().map(|path| path.to_path_buf());
    let operation = wide("open");
    let file = wide(&executable.to_string_lossy());
    let directory = directory.map(|path| wide(&path.to_string_lossy()));
    let parameters = wide("");
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            directory
                .as_ref()
                .map(|value| PCWSTR(value.as_ptr()))
                .unwrap_or(PCWSTR::null()),
            SW_SHOWMINNOACTIVE,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Windows could not start Spotify.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn launch_minimized() -> Result<(), String> {
    Err("Starting Spotify in the background requires Windows.".into())
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
