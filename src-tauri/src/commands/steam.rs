use tauri::AppHandle;

pub(crate) fn launch(app: &AppHandle) -> Result<(), String> {
    if crate::commands::session::focus_steam(app) {
        return Ok(());
    }
    if crate::commands::apps::detect("Steam").is_none() {
        return Err("Steam was not found. Install Steam, then try again.".into());
    }
    open("steam://open/bigpicture")?;
    crate::commands::session::track_steam(app);
    Ok(())
}

pub(crate) fn close() {
    let _ = open("steam://close/bigpicture");
}

#[cfg(windows)]
fn open(target: &str) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let operation = wide("open");
    let target = wide(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Windows could not open Steam Big Picture.".into());
    }
    Ok(())
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn open(_: &str) -> Result<(), String> {
    Err("Steam Big Picture requires Windows.".into())
}
