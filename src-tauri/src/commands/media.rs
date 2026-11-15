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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Timeline {
    position: f64,
    duration: f64,
}

static TIMELINES: std::sync::Mutex<Option<(String, std::time::Instant, Timeline)>> = std::sync::Mutex::new(None);

#[tauri::command]
pub fn report_timeline(window: tauri::WebviewWindow, timeline: Option<Timeline>) {
    if !["youtube", "stremio"].contains(&window.label()) { return; }
    if let Ok(mut current) = TIMELINES.lock() {
        *current = timeline.map(|timeline| (window.label().into(), std::time::Instant::now(), timeline));
    }
}

pub async fn timeline(app: &tauri::AppHandle, position: Option<f64>) -> Result<serde_json::Value, String> {
    let script = format!("({})({})", include_str!("../../resources/timeline.js"), serde_json::to_string(&position).map_err(|e| e.to_string())?);
    for label in ["youtube", "stremio"] {
        if let Some(window) = app.get_webview_window(label).filter(|w| w.is_visible().unwrap_or(false)) {
            window.eval(format!("window.__TAURI_INTERNALS__.invoke('report_timeline', {{timeline: {script}}}).catch(() => {{}});" )).map_err(|e| e.to_string())?;
            return Ok(TIMELINES.lock().ok().and_then(|current| current.as_ref().filter(|(name, at, _)| name == label && at.elapsed().as_secs() < 3).map(|(_, _, value)| serde_json::to_value(value).unwrap_or_default())).unwrap_or_default());
        }
    }
    crate::commands::services::timeline(app, &script).await
}
