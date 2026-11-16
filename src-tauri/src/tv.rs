use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    display: String,
    audio: String,
}

#[derive(Default)]
pub struct Tv(
    pub Mutex<Preferences>,
    Mutex<Option<String>>,
    Mutex<Option<String>>,
    Mutex<Option<u32>>,
);

fn selected_display(app: &tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<Tv>();
    let current = state.2.lock().map_err(|e| e.to_string())?.clone();
    if let Some(current) = current {
        return Ok(current);
    }
    let display = state.0.lock().map_err(|e| e.to_string())?.display.clone();
    Ok(display)
}

#[tauri::command]
pub fn switch_display(app: tauri::AppHandle) -> Result<String, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    if monitors.len() < 2 {
        return Err("Connect another display before switching screens.".into());
    }
    let selected = selected_display(&app)?;
    let current = app
        .get_webview_window("main")
        .and_then(|window| window.current_monitor().ok().flatten())
        .or_else(|| {
            crate::viewing::selected(&app)
                .and_then(|window| window.current_monitor().ok().flatten())
        });
    let index = current
        .as_ref()
        .and_then(|current| {
            monitors
                .iter()
                .position(|monitor| monitor.position() == current.position())
        })
        .or_else(|| {
            monitors
                .iter()
                .position(|monitor| monitor.name() == Some(&selected))
        })
        .unwrap_or(0);
    let target = &monitors[(index + 1) % monitors.len()];
    let name = target
        .name()
        .cloned()
        .ok_or("This display has no identifier.")?;
    *app.state::<Tv>().2.lock().map_err(|e| e.to_string())? = Some(name.clone());
    for window in app
        .webview_windows()
        .values()
        .filter(|window| ["main", "youtube", "stremio", "sports"].contains(&window.label()))
    {
        place(window)?;
    }
    crate::commands::session::place_current(&app)?;
    crate::viewing::arrange(&app)?;
    Ok(name)
}

#[cfg(windows)]
pub fn shortcut(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        use windows::Win32::UI::{
            Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
            },
            WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY},
        };
        unsafe {
            *app.state::<Tv>()
                .3
                .lock()
                .unwrap_or_else(|error| error.into_inner()) =
                Some(windows::Win32::System::Threading::GetCurrentThreadId());
            if RegisterHotKey(None, 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, b'M' as u32).is_err() {
                *app.state::<Tv>()
                    .3
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = None;
                report_audio(&app, Some("Ctrl+Alt+M is in use by another app. Use Switch display in Luma or the remote.".into()));
                return;
            }
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                if message.message == WM_HOTKEY {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Err(error) = switch_display(handle.clone()) {
                            use tauri::Emitter;
                            let _ = handle.emit("tv-setup-error", error);
                        }
                    });
                }
            }
            let _ = UnregisterHotKey(None, 1);
            *app.state::<Tv>()
                .3
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = None;
        }
    });
}

#[cfg(not(windows))]
pub fn shortcut(_: &tauri::AppHandle) {}

pub fn stop_shortcut(app: &tauri::AppHandle) {
    #[cfg(windows)]
    if let Ok(thread) = app.state::<Tv>().3.lock() {
        if let Some(thread) = *thread {
            use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            let _ = unsafe {
                PostThreadMessageW(
                    thread,
                    WM_QUIT,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                )
            };
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    id: String,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    preferences: Preferences,
    displays: Vec<Display>,
    audio: Vec<audio::Output>,
}

pub fn load(app: &tauri::AppHandle) -> Result<(), String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("tv.json");
    match std::fs::read(path) {
        Ok(bytes) => {
            *app.state::<Tv>().0.lock().map_err(|e| e.to_string())? = serde_json::from_slice(&bytes)
                .map_err(|e| format!("TV settings could not be read: {e}"))?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.to_string()),
    }
    Ok(())
}

#[tauri::command]
pub async fn tv_setup(app: tauri::AppHandle) -> Result<Setup, String> {
    let displays = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|monitor| {
            Some(Display {
                id: monitor.name()?.clone(),
                width: monitor.size().width,
                height: monitor.size().height,
            })
        })
        .collect();
    let preferences = app
        .state::<Tv>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let audio = tauri::async_runtime::spawn_blocking(audio::outputs)
        .await
        .map_err(|e| e.to_string())??;
    Ok(Setup {
        preferences,
        displays,
        audio,
    })
}

pub fn place(window: &tauri::WebviewWindow) -> Result<(), String> {
    let selected = selected_display(window.app_handle())?;
    if selected.is_empty() {
        return Ok(());
    }
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let target = monitors
        .iter()
        .find(|monitor| monitor.name() == Some(&selected))
        .or_else(|| monitors.first())
        .ok_or("No display is available.")?;
    if window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .as_ref()
        .is_some_and(|current| current.name() == target.name())
    {
        return Ok(());
    }
    let fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    if fullscreen {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
    }
    window
        .set_position(*target.position())
        .map_err(|e| e.to_string())?;
    if fullscreen {
        window.set_fullscreen(true).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn bounds(app: &tauri::AppHandle) -> Option<(i32, i32, u32, u32)> {
    let selected = selected_display(app).ok()?;
    if selected.is_empty() {
        return None;
    }
    let monitors = app.available_monitors().ok()?;
    let target = monitors
        .iter()
        .find(|monitor| monitor.name() == Some(&selected))
        .or_else(|| monitors.first())?;
    Some((
        target.position().x,
        target.position().y,
        target.size().width,
        target.size().height,
    ))
}

#[tauri::command]
pub async fn save_tv_setup(app: tauri::AppHandle, preferences: Preferences) -> Result<(), String> {
    if !preferences.display.is_empty()
        && !app
            .available_monitors()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|monitor| monitor.name() == Some(&preferences.display))
    {
        return Err("That screen is disconnected. Turn on the TV and refresh devices.".into());
    }
    let directory = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&preferences).map_err(|e| e.to_string())?;
    let audio = preferences.audio.clone();
    tauri::async_runtime::spawn_blocking(move || audio::select(&audio))
        .await
        .map_err(|e| e.to_string())??;
    if let Err(error) = std::fs::write(directory.join("tv.json"), bytes) {
        let _ = apply_audio(&app).await;
        return Err(format!("TV setup could not be saved: {error}"));
    }
    report_audio(&app, None);
    *app.state::<Tv>().0.lock().map_err(|e| e.to_string())? = preferences;
    *app.state::<Tv>().2.lock().map_err(|e| e.to_string())? = None;
    for window in app.webview_windows().values() {
        place(window)?;
    }
    crate::commands::session::place_current(&app)?;
    crate::viewing::arrange(&app)?;
    Ok(())
}

pub async fn apply_audio(app: &tauri::AppHandle) -> Result<(), String> {
    let selected = app
        .state::<Tv>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .audio
        .clone();
    tauri::async_runtime::spawn_blocking(move || audio::select(&selected))
        .await
        .map_err(|e| e.to_string())?
}

pub fn restore_audio_sync() -> Result<(), String> {
    audio::restore()
}

pub async fn restore_audio() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(audio::restore)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn restore_tv_audio() -> Result<(), String> {
    restore_audio().await
}

pub fn report_audio(app: &tauri::AppHandle, error: Option<String>) {
    use tauri::Emitter;
    if let Ok(mut warning) = app.state::<Tv>().1.lock() {
        *warning = error.clone();
    }
    if let Some(error) = error {
        let _ = app.emit("tv-setup-error", error);
    }
}

#[tauri::command]
pub fn tv_setup_warning(app: tauri::AppHandle) -> Option<String> {
    app.state::<Tv>()
        .1
        .lock()
        .ok()
        .and_then(|warning| warning.clone())
}

#[cfg(windows)]
mod audio;

#[cfg(not(windows))]
mod audio {
    #[derive(serde::Serialize)]
    pub struct Output {
        id: String,
        name: String,
    }
    pub fn outputs() -> Result<Vec<Output>, String> {
        Ok(Vec::new())
    }
    pub fn select(id: &str) -> Result<(), String> {
        if id.is_empty() {
            Ok(())
        } else {
            Err("TV audio requires Windows.".into())
        }
    }
    pub fn restore() -> Result<(), String> {
        Ok(())
    }
}
