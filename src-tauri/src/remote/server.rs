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
    let handled = crate::commands::session::route(app, command.as_str());
    let embedded = if handled {
        false
    } else if app
        .get_webview_window("stremio")
        .filter(|window| window.is_visible().unwrap_or(false))
        .is_some()
    {
        crate::commands::stremio::forward(app, command.as_str());
        true
    } else if app
        .get_webview_window("youtube")
        .filter(|window| window.is_visible().unwrap_or(false))
        .is_some()
    {
        youtube::forward(app, command.as_str());
        true
    } else {
        false
    };
    if frontend_receives(handled, embedded, command) {
        if show_main(handled, embedded, command) {
            crate::background::show(app);
        }
        let _ = app.emit("remote-navigation", command);
    }
}

fn frontend_receives(handled: bool, embedded: bool, command: NavigationCommand) -> bool {
    matches!(command, NavigationCommand::Home) || (!handled && !embedded)
}

fn show_main(handled: bool, embedded: bool, command: NavigationCommand) -> bool {
    matches!(command, NavigationCommand::Home) && !handled && !embedded
}

pub(crate) fn open_music(app: &tauri::AppHandle) -> Result<(), String> {
    crate::commands::session::release(app)?;
    for label in ["stremio", "youtube"] {
        if app
            .get_webview_window(label)
            .is_some_and(|window| window.is_visible().unwrap_or(false))
        {
            if label == "stremio" {
                crate::commands::stremio::close(app);
            } else {
                youtube::close(app);
            }
        }
    }
    crate::background::show(app);
    app.emit_to("main", "open-music", ())
        .map_err(|error| error.to_string())?;
    app.state::<Remote>()
        .state
        .lock()
        .map_err(|error| error.to_string())?
        .view = "Music".into();
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{frontend_receives, show_main, NavigationCommand};

    #[test]
    fn home_always_resets_the_luma_frontend() {
        assert!(frontend_receives(true, false, NavigationCommand::Home));
        assert!(frontend_receives(false, true, NavigationCommand::Home));
        assert!(frontend_receives(false, false, NavigationCommand::Home));
    }

    #[test]
    fn routed_commands_are_not_duplicated_on_the_frontend() {
        assert!(!frontend_receives(true, false, NavigationCommand::Left));
        assert!(!frontend_receives(false, true, NavigationCommand::Activate));
        assert!(frontend_receives(false, false, NavigationCommand::Left));
    }

    #[test]
    fn home_brings_back_a_hidden_luma_but_not_a_handled_app() {
        assert!(show_main(false, false, NavigationCommand::Home));
        assert!(!show_main(true, false, NavigationCommand::Home));
        assert!(!show_main(false, true, NavigationCommand::Home));
        assert!(!show_main(false, false, NavigationCommand::Left));
    }
}
