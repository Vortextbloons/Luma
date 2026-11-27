#[cfg(windows)]
fn process_ids() -> Vec<u32> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    let mut ids = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return ids;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = Process32FirstW(snapshot, &mut entry).is_ok();
        while found {
            let name = String::from_utf16_lossy(&entry.szExeFile);
            let name = name.trim_end_matches('\0');
            if name.eq_ignore_ascii_case("Spotify.exe")
                || name.eq_ignore_ascii_case("SpotifyLauncher.exe")
            {
                ids.push(entry.th32ProcessID);
            }
            found = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    ids
}

#[cfg(windows)]
pub fn running() -> bool {
    !process_ids().is_empty()
}

#[cfg(windows)]
pub fn hide_main_window() -> bool {
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{HWND, LPARAM},
            UI::WindowsAndMessaging::{
                EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId,
                IsWindowVisible, ShowWindow, SW_HIDE,
            },
        },
    };
    struct Target {
        ids: Vec<u32>,
        hidden: bool,
    }
    unsafe extern "system" fn hide(window: HWND, target: LPARAM) -> BOOL {
        let target = &mut *(target.0 as *mut Target);
        let mut id = 0;
        GetWindowThreadProcessId(window, Some(&mut id));
        if !target.ids.contains(&id) || !IsWindowVisible(window).as_bool() {
            return BOOL(1);
        }
        let mut class = [0u16; 64];
        let class_length = GetClassNameW(window, &mut class);
        if class_length <= 0
            || !String::from_utf16_lossy(&class[..class_length as usize])
                .starts_with("Chrome_WidgetWin")
        {
            return BOOL(1);
        }
        let mut title = [0u16; 256];
        if GetWindowTextW(window, &mut title) <= 0 {
            return BOOL(1);
        }
        let _ = ShowWindow(window, SW_HIDE);
        target.hidden = true;
        BOOL(1)
    }
    let mut target = Target {
        ids: process_ids(),
        hidden: false,
    };
    unsafe {
        let _ = EnumWindows(Some(hide), LPARAM(&mut target as *mut Target as isize));
    }
    target.hidden
}

#[cfg(not(windows))]
pub fn running() -> bool {
    false
}

#[cfg(not(windows))]
pub fn hide_main_window() -> bool {
    false
}

fn configured_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    if let Some(apps) = app.try_state::<crate::commands::apps::Apps>() {
        if let Ok(paths) = apps.0.lock() {
            if let Some(path) = paths.get("Spotify").filter(|path| !path.is_empty()) {
                let path = std::path::PathBuf::from(path);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    crate::commands::apps::detect("Spotify")
}

pub fn installed(app: &tauri::AppHandle) -> bool {
    configured_path(app).is_some()
}

pub fn launch_background(app: &tauri::AppHandle) -> Result<(), String> {
    let executable =
        configured_path(app).ok_or("Spotify is not installed or its path is unknown.")?;
    launch_path_background(&executable)
}

#[cfg(windows)]
pub fn launch_path_background(executable: &std::path::Path) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWMINNOACTIVE},
    };
    let directory = executable.parent().map(|path| path.to_path_buf());
    let operation = wide("open");
    let file = wide(&executable.to_string_lossy());
    let directory = directory.map(|path| wide(&path.to_string_lossy()));
    let parameters = wide("--autostart --minimized");
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
pub fn launch_path_background(_: &std::path::Path) -> Result<(), String> {
    Err("Starting Spotify in the background requires Windows.".into())
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
