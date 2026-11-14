use crate::commands::youtube;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    view: String,
    fullscreen: bool,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NavigationCommand {
    Up,
    Down,
    Left,
    Right,
    Activate,
    Back,
    Home,
}

impl NavigationCommand {
    fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Activate => "activate",
            Self::Back => "back",
            Self::Home => "home",
        }
    }
}

pub(crate) fn navigate(app: &tauri::AppHandle, command: NavigationCommand) {
    if crate::commands::session::route(app, command.as_str()) {
        return;
    }
    let stremio = app
        .get_webview_window("stremio")
        .filter(|window| window.is_visible().unwrap_or(false));
    if stremio.is_some() {
        crate::commands::stremio::forward(app, command.as_str());
        if matches!(command, NavigationCommand::Home) {
            let _ = app.emit("remote-navigation", command);
        }
    } else if app
        .get_webview_window("youtube")
        .filter(|window| window.is_visible().unwrap_or(false))
        .is_some()
    {
        youtube::forward(app, command.as_str());
        if matches!(command, NavigationCommand::Home) {
            let _ = app.emit("remote-navigation", command);
        }
    } else {
        let _ = app.emit("remote-navigation", command);
    }
}

pub struct Remote {
    state: Mutex<AppState>,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            state: Mutex::new(AppState {
                view: "Home".into(),
                fullscreen: false,
            }),
        }
    }
}

pub(crate) fn snapshot(app: &tauri::AppHandle) -> Result<AppState, String> {
    app.state::<Remote>()
        .state
        .lock()
        .map(|state| state.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_app_state(
    remote: tauri::State<Remote>,
    view: String,
    fullscreen: bool,
) -> Result<(), String> {
    if !["Home", "Watch", "Music", "Apps", "Settings", "Details"].contains(&view.as_str()) {
        return Err("Invalid view.".into());
    }
    *remote.state.lock().map_err(|e| e.to_string())? = AppState { view, fullscreen };
    Ok(())
}
