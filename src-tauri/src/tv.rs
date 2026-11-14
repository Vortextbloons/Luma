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
pub struct Tv(pub Mutex<Preferences>, Mutex<Option<String>>);

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
    let path = app.path().app_config_dir().map_err(|e| e.to_string())?.join("tv.json");
    match std::fs::read(path) {
        Ok(bytes) => *app.state::<Tv>().0.lock().map_err(|e| e.to_string())? = serde_json::from_slice(&bytes).map_err(|e| format!("TV settings could not be read: {e}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.to_string()),
    }
    Ok(())
}

#[tauri::command]
pub async fn tv_setup(app: tauri::AppHandle) -> Result<Setup, String> {
    let displays = app.available_monitors().map_err(|e| e.to_string())?.into_iter().filter_map(|monitor| {
        Some(Display { id: monitor.name()?.clone(), width: monitor.size().width, height: monitor.size().height })
    }).collect();
    let preferences = app.state::<Tv>().0.lock().map_err(|e| e.to_string())?.clone();
    let audio = tauri::async_runtime::spawn_blocking(audio::outputs).await.map_err(|e| e.to_string())??;
    Ok(Setup { preferences, displays, audio })
}

pub fn place(window: &tauri::WebviewWindow) -> Result<(), String> {
    let selected = window.app_handle().state::<Tv>().0.lock().map_err(|e| e.to_string())?.display.clone();
    if selected.is_empty() { return Ok(()); }
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let target = monitors.iter().find(|monitor| monitor.name() == Some(&selected))
        .or_else(|| monitors.first()).ok_or("No display is available.")?;
    if window.current_monitor().map_err(|e| e.to_string())?.as_ref().is_some_and(|current| current.name() == target.name()) { return Ok(()); }
    let fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    if fullscreen { window.set_fullscreen(false).map_err(|e| e.to_string())?; }
    window.set_position(*target.position()).map_err(|e| e.to_string())?;
    if fullscreen { window.set_fullscreen(true).map_err(|e| e.to_string())?; }
    Ok(())
}

pub fn bounds(app: &tauri::AppHandle) -> Option<(i32, i32, u32, u32)> {
    let selected = app.state::<Tv>().0.lock().ok()?.display.clone();
    if selected.is_empty() { return None; }
    let monitors = app.available_monitors().ok()?;
    let target = monitors.iter().find(|monitor| monitor.name() == Some(&selected)).or_else(|| monitors.first())?;
    Some((target.position().x, target.position().y, target.size().width, target.size().height))
}

#[tauri::command]
pub async fn save_tv_setup(app: tauri::AppHandle, preferences: Preferences) -> Result<(), String> {
    if !preferences.display.is_empty() && !app.available_monitors().map_err(|e| e.to_string())?.iter().any(|monitor| monitor.name() == Some(&preferences.display)) {
        return Err("That screen is disconnected. Turn on the TV and refresh devices.".into());
    }
    let directory = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&preferences).map_err(|e| e.to_string())?;
    let audio = preferences.audio.clone();
    tauri::async_runtime::spawn_blocking(move || audio::select(&audio)).await.map_err(|e| e.to_string())??;
    if let Err(error) = std::fs::write(directory.join("tv.json"), bytes) {
        let _ = apply_audio(&app).await;
        return Err(format!("TV setup could not be saved: {error}"));
    }
    report_audio(&app, None);
    *app.state::<Tv>().0.lock().map_err(|e| e.to_string())? = preferences;
    for window in app.webview_windows().values() { place(window)?; }
    Ok(())
}

pub async fn apply_audio(app: &tauri::AppHandle) -> Result<(), String> {
    let selected = app.state::<Tv>().0.lock().map_err(|e| e.to_string())?.audio.clone();
    tauri::async_runtime::spawn_blocking(move || audio::select(&selected)).await.map_err(|e| e.to_string())?
}

pub fn restore_audio_sync() -> Result<(), String> { audio::restore() }

pub async fn restore_audio() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(audio::restore).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn restore_tv_audio() -> Result<(), String> { restore_audio().await }

pub fn report_audio(app: &tauri::AppHandle, error: Option<String>) {
    use tauri::Emitter;
    if let Ok(mut warning) = app.state::<Tv>().1.lock() { *warning = error.clone(); }
    if let Some(error) = error { let _ = app.emit("tv-setup-error", error); }
}

#[tauri::command]
pub fn tv_setup_warning(app: tauri::AppHandle) -> Option<String> {
    app.state::<Tv>().1.lock().ok().and_then(|warning| warning.clone())
}

#[cfg(windows)]
mod audio;

#[cfg(not(windows))]
mod audio {
    #[derive(serde::Serialize)]
    pub struct Output { id: String, name: String }
    pub fn outputs() -> Result<Vec<Output>, String> { Ok(Vec::new()) }
    pub fn select(id: &str) -> Result<(), String> {
        if id.is_empty() { Ok(()) } else { Err("TV audio requires Windows.".into()) }
    }
    pub fn restore() -> Result<(), String> { Ok(()) }
}
