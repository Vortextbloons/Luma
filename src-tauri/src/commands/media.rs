use crate::integrations::spotify::{self, auth::Auth};
use tauri::Manager;

pub fn stop(app: &tauri::AppHandle) {
    spotify::shutdown(app);
}

pub async fn control(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    if !matches!(action, "toggle" | "previous" | "next") {
        return Err("Unknown playback command.".into());
    }
    for label in ["youtube", "stremio"] {
        if let Some(window) = app.get_webview_window(label).filter(|window| window.is_visible().unwrap_or(false)) {
            let action = serde_json::to_string(action).map_err(|error| error.to_string())?;
            let script = include_str!("../../resources/playback.js");
            return window.eval(format!("({script})({action});")).map_err(|error| error.to_string());
        }
    }
    let auth = app.state::<Auth>();
    spotify::control(app, &auth, action, None).await
}
