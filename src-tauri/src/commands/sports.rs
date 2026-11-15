use std::sync::OnceLock;
use tauri::{webview::NewWindowResponse, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

fn event_url(value: &str) -> Result<tauri::Url, String> {
    let url: tauri::Url = value.parse().map_err(|_| "Invalid sports event.".to_string())?;
    if url.scheme() != "https" || url.host_str() != Some("isportsurge.ws")
        || url.port().is_some() || !url.username().is_empty() || url.password().is_some()
        || !(url.path().starts_with("/watch/") || url.path().starts_with("/event/"))
    {
        return Err("Choose an event from the sports listings.".into());
    }
    Ok(url)
}

#[tauri::command]
pub async fn sports_listing() -> Result<String, String> {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build()
        .map_err(|error| error.to_string())?
        .get("https://isportsurge.ws/index13").send().await
        .map_err(|_| "Sports listings could not load. Check your connection.".to_string())?
        .error_for_status().map_err(|_| "The sports source is unavailable.".to_string())?
        .text().await.map_err(|_| "Sports listings could not be read.".to_string())
}

#[tauri::command]
pub async fn sports_open(app: AppHandle, url: String, source_index: usize, title: String, score: String) -> Result<(), String> {
    static WINDOW: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    let _guard = WINDOW.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    let url = event_url(&url)?;
    if let Some(window) = app.get_webview_window("sports") {
        window.destroy().map_err(|error| error.to_string())?;
    }
    crate::background::clear_unloading("sports");
    let main = app.get_webview_window("main").ok_or("Luma window is unavailable")?;
    let size = main.inner_size().map_err(|error| error.to_string())?;
    let position = main.outer_position().map_err(|error| error.to_string())?;
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let navigation = app.clone();
    let script = format!("window.__lumaSportsSource = {source_index}; window.__lumaSportsTitle = {}; window.__lumaSportsScore = {};\n{}", serde_json::to_string(&title).map_err(|error| error.to_string())?, serde_json::to_string(&score).map_err(|error| error.to_string())?, include_str!("../../resources/sports.js"));
    let window = WebviewWindowBuilder::new(&app, "sports", WebviewUrl::External(url))
        .title("Luma Sports")
        .additional_browser_args(crate::BROWSER_ARGS)
        .inner_size(size.width as f64 / scale, size.height as f64 / scale)
        .position(position.x as f64 / scale, position.y as f64 / scale)
        .min_inner_size(720.0, 405.0)
        .fullscreen(true)
        .initialization_script_for_all_frames(&script)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_navigation(move |url| {
            if url.scheme() == "https" && url.host_str() == Some("isportsurge.ws") && url.path() == "/__luma_close__" {
                let handle = navigation.clone();
                tauri::async_runtime::spawn(async move { close(&handle); });
                return false;
            }
            url.scheme() == "https" && url.host_str() == Some("isportsurge.ws")
        })
        .build().map_err(|error| format!("The sports stream could not open: {error}"))?;
    let restore = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            close(&restore);
        }
    });
    if let Err(error) = crate::background::present(window, Some(true)).await {
        close(&app);
        return Err(error);
    }
    Ok(())
}

pub fn close(app: &AppHandle) {
    crate::background::unload(app, "sports");
    crate::background::show(app);
    let _ = app.emit_to("main", "sports-closed", ());
}

pub fn forward(app: &AppHandle, command: &str) {
    if command == "home" {
        close(app);
    } else if let Some(window) = app.get_webview_window("sports") {
        if let Ok(command) = serde_json::to_string(command) {
            let _ = window.eval(format!("window.__lumaSportsNavigate?.({command});"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::event_url;

    #[test]
    fn only_source_event_pages_can_be_opened() {
        assert!(event_url("https://isportsurge.ws/watch/nba/game/123").is_ok());
        assert!(event_url("https://isportsurge.ws/event/nfl/redzone").is_ok());
        for url in ["http://isportsurge.ws/watch/a", "https://isportsurge.ws.evil.com/watch/a", "https://isportsurge.ws/index13", "https://user@isportsurge.ws/watch/a", "https://isportsurge.ws:444/watch/a"] {
            assert!(event_url(url).is_err());
        }
    }
}

fn league_path(league: &str) -> Result<&'static str, String> {
    match league {
        "NBA" => Ok("basketball/nba"),
        "NFL" => Ok("football/nfl"),
        "MLB" => Ok("baseball/mlb"),
        "NHL" => Ok("hockey/nhl"),
        _ => Err("Scores are unavailable for this league.".into()),
    }
}

async fn data_request(path: &str) -> Result<serde_json::Value, String> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    let client = CLIENT.get_or_init(|| reqwest::Client::builder().timeout(std::time::Duration::from_secs(12)).build().unwrap_or_default());
    client.get(format!("https://site.api.espn.com/apis/site/v2/sports/{path}"))
        .send().await.map_err(|_| "Game information is unavailable.".to_string())?
        .error_for_status().map_err(|_| "Game information is unavailable.".to_string())?
        .json().await.map_err(|_| "Game information could not be read.".to_string())
}

#[tauri::command]
pub async fn sports_scoreboard(league: String) -> Result<serde_json::Value, String> {
    data_request(&format!("{}/scoreboard?dates={}", league_path(&league)?, chrono::Local::now().format("%Y%m%d"))).await
}

#[tauri::command]
pub async fn sports_artwork(league: String, event_id: String) -> Result<Option<String>, String> {
    if event_id.is_empty() || !event_id.chars().all(|character| character.is_ascii_digit()) {
        return Err("Invalid game identifier.".into());
    }
    let data = data_request(&format!("{}/summary?event={event_id}", league_path(&league)?)).await?;
    Ok(data.pointer("/gameInfo/venue/images/0/href").and_then(serde_json::Value::as_str)
        .filter(|url| url.starts_with("https://")).map(str::to_string))
}

#[tauri::command]
pub fn sports_update_score(app: AppHandle, score: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("sports") {
        let score = serde_json::to_string(&score).map_err(|error| error.to_string())?;
        window.eval(format!("window.__lumaSportsScoreUpdate?.({score});")).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub async fn control(app: &AppHandle, action: &str) -> Result<(), String> {
    if !matches!(action, "toggle" | "previous" | "next") { return Err("Unknown playback command.".into()); }
    let window = app.get_webview_window("sports").ok_or("The sports player is closed.")?;
    let action = serde_json::to_string(action).map_err(|error| error.to_string())?;
    window.eval(format!("window.__lumaSportsControl?.({action});")).map_err(|error| error.to_string())
}

pub async fn timeline(app: &AppHandle, position: Option<f64>) -> Result<serde_json::Value, String> {
    let window = app.get_webview_window("sports").ok_or("The sports player is closed.")?;
    let position = serde_json::to_string(&position).map_err(|error| error.to_string())?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    window.eval_with_callback(format!("window.__lumaSportsTimeline?.({position}) ?? null"), move |value| {
        if let Ok(mut sender) = sender.lock() { if let Some(sender) = sender.take() { let _ = sender.send(value); } }
    }).map_err(|error| error.to_string())?;
    let value = tokio::time::timeout(std::time::Duration::from_secs(2), receiver).await
        .map_err(|_| "The sports player did not respond.".to_string())?.map_err(|error| error.to_string())?;
    serde_json::from_str(&value).map_err(|error| error.to_string())
}
