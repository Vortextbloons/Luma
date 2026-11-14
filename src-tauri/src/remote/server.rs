use crate::commands::youtube;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    view: String,
    fullscreen: bool,
    window: Option<String>,
    window_fullscreen: Option<bool>,
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

fn should_wake(visible: bool, external_session: bool) -> bool {
    !visible && !external_session
}

pub(crate) async fn wake(app: &tauri::AppHandle) -> Result<(), String> {
    let visible = ["main", "youtube", "stremio"].iter().any(|label|
        app.get_webview_window(label).is_some_and(|window| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)));
    if should_wake(visible, crate::commands::session::active(app)) {
        let window = app.get_webview_window("main").ok_or("Luma window is unavailable")?;
        crate::background::present(window, None).await?;
    }
    Ok(())
}

pub(crate) fn stop(app: &tauri::AppHandle) -> Result<(), String> {
    crate::remote::pointer::stop_all(app);
    crate::remote::pointer::show_cursor(app);
    for label in ["main", "youtube", "stremio"] {
        if let Some(window) = app.get_webview_window(label) {
            if label != "main" { let _ = window.eval("document.querySelectorAll('video,audio').forEach(media => media.pause())"); }
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    let _ = crate::tv::restore_audio_sync();
    Ok(())
}

#[cfg(windows)]
pub(crate) fn type_text(text: &str) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE, KEYEVENTF_KEYUP};
    let inputs: Vec<INPUT> = text.encode_utf16().flat_map(|unit| [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP].map(|flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wScan: unit, dwFlags: flags, ..Default::default() } },
    })).collect();
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() { return Err("Windows could not type the text. Focus a text field on the PC and try again.".into()); }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn type_text(_: &str) -> Result<(), String> { Err("Remote typing requires Windows.".into()) }

pub(crate) fn navigate(app: &tauri::AppHandle, command: NavigationCommand) {
    match command {
        NavigationCommand::Home => {
            crate::remote::pointer::stop_all(app);
            crate::remote::pointer::hide_cursor(app);
        }
        NavigationCommand::Up
        | NavigationCommand::Down
        | NavigationCommand::Left
        | NavigationCommand::Right
        | NavigationCommand::Activate
        | NavigationCommand::Back => crate::remote::pointer::hide_cursor(app),
    }
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
                window: None,
                window_fullscreen: None,
            }),
        }
    }
}

fn active_window(app: &tauri::AppHandle) -> Option<(&'static str, tauri::WebviewWindow)> {
    let windows: Vec<_> = [("main", "Luma"), ("youtube", "YouTube"), ("stremio", "Stremio")]
        .into_iter()
        .filter_map(|(label, name)| {
            let window = app.get_webview_window(label)?;
            if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
                return None;
            }
            Some((name, window))
        })
        .collect();
    windows
        .iter()
        .find(|(_, window)| window.is_focused().unwrap_or(false))
        .or_else(|| windows.first())
        .cloned()
}

pub(crate) fn snapshot(app: &tauri::AppHandle) -> Result<AppState, String> {
    app.state::<Remote>()
        .state
        .lock()
        .map(|state| state.clone())
        .map_err(|e| e.to_string())
}

pub(crate) fn snapshot_live(app: &tauri::AppHandle) -> Result<AppState, String> {
    let window = active_window(app).map(|(name, window)| (name, window.is_fullscreen().ok()));
    let remote = app.state::<Remote>();
    let mut state = remote.state.lock().map_err(|e| e.to_string())?;
    if let Some((name, fullscreen)) = window {
        state.window = Some(name.into());
        state.window_fullscreen = fullscreen;
    } else {
        state.window = None;
        state.window_fullscreen = None;
    }
    Ok(state.clone())
}

pub(crate) fn fullscreen(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(result) = crate::commands::session::fullscreen(app) {
        return result;
    }
    let (_, window) = active_window(app).ok_or("Open Luma or an app before using PC fullscreen.")?;
    let value = !window.is_fullscreen().map_err(|error| error.to_string())?;
    window
        .set_fullscreen(value)
        .map_err(|error| error.to_string())
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
    let mut state = remote.state.lock().map_err(|e| e.to_string())?;
    state.view = view;
    state.fullscreen = fullscreen;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{frontend_receives, show_main, should_wake, NavigationCommand};

    #[test]
    fn external_session_prevents_waking_the_launcher() {
        assert!(!should_wake(false, true));
        assert!(should_wake(false, false));
        assert!(!should_wake(true, false));
    }

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
    fn pc_window_state_uses_the_field_names_the_remotes_read() {
        let state = super::AppState {
            view: "Home".into(),
            fullscreen: false,
            window: Some("YouTube".into()),
            window_fullscreen: Some(true),
        };
        let value = serde_json::to_value(state).unwrap();
        assert_eq!(value["window"], "YouTube");
        assert_eq!(value["windowFullscreen"], true);
    }

    #[test]
    fn home_brings_back_a_hidden_luma_but_not_a_handled_app() {
        assert!(show_main(false, false, NavigationCommand::Home));
        assert!(!show_main(true, false, NavigationCommand::Home));
        assert!(!show_main(false, true, NavigationCommand::Home));
        assert!(!show_main(false, false, NavigationCommand::Left));
    }
}
