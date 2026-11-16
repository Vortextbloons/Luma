use std::sync::OnceLock;
use tauri::{
    webview::NewWindowResponse, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub(crate) fn shortcut_league(command: &str) -> Option<&'static str> {
    match command {
        "sportsnba" => Some("NBA"),
        "sportsnfl" => Some("NFL"),
        "sportsmlb" => Some("MLB"),
        "sportsnhl" => Some("NHL"),
        _ => None,
    }
}

pub(crate) fn shortcuts() -> Vec<super::apps::CatalogEntry> {
    [
        (
            "sportsnba",
            "NBA",
            include_bytes!("../../resources/icons/sports-nba.png").as_slice(),
        ),
        (
            "sportsnfl",
            "NFL",
            include_bytes!("../../resources/icons/sports-nfl.png").as_slice(),
        ),
        (
            "sportsmlb",
            "MLB",
            include_bytes!("../../resources/icons/sports-mlb.png").as_slice(),
        ),
        (
            "sportsnhl",
            "NHL",
            include_bytes!("../../resources/icons/sports-nhl.png").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(command, name, icon)| super::apps::CatalogEntry {
        command: command.into(),
        name,
        icon,
    })
    .collect()
}

fn event_url(value: &str) -> Result<tauri::Url, String> {
    let url: tauri::Url = value
        .parse()
        .map_err(|_| "Invalid sports event.".to_string())?;
    if url.scheme() != "https"
        || !match url.host_str() {
            Some("isportsurge.ws") => url.path().starts_with("/watch/") || url.path().starts_with("/event/"),
            Some("golygoals.com") => matches!(url.path(), "/streameast.html" | "/streame.html") && url.query_pairs().any(|(key, value)| matches!(key.as_ref(), "id" | "key") && !value.is_empty()),
            Some("streamec.st") => url.path().starts_with("/embed/ch") && url.path().ends_with(".php"),
            _ => false,
        }
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Choose an event from the sports listings.".into());
    }
    Ok(url)
}

#[tauri::command]
pub async fn sports_listing() -> Result<String, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?
        .get("https://isportsurge.ws/index13")
        .send()
        .await
        .map_err(|_| "Sports listings could not load. Check your connection.".to_string())?
        .error_for_status()
        .map_err(|_| "The sports source is unavailable.".to_string())?
        .text()
        .await
        .map_err(|_| "Sports listings could not be read.".to_string())
}

#[tauri::command]
pub async fn sports_streameast_listing(app: AppHandle, force: Option<bool>) -> Result<serde_json::Value, String> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    static CACHE: OnceLock<std::sync::Mutex<Option<(std::time::Instant, serde_json::Value)>>> = OnceLock::new();
    let _guard = LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await;
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(None));
    if let Some((time, data)) = cache.lock().map_err(|error| error.to_string())?.as_ref() {
        if !force.unwrap_or(false) && time.elapsed() < std::time::Duration::from_secs(60) { return Ok(data.clone()); }
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    let window = WebviewWindowBuilder::new(&app, "sports-catalog", WebviewUrl::External("https://gostreameast.one/".parse().map_err(|_| "Invalid source URL")?))
        .visible(false).focused(false).skip_taskbar(true)
        .initialization_script(include_str!("../../resources/sports-catalog.js"))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_navigation(|url| url.scheme() == "https" && url.host_str() == Some("gostreameast.one"))
        .on_document_title_changed(move |_, title| {
            if let Some(data) = title.strip_prefix("LumaSportsCatalog:") {
                if let Ok(mut sender) = sender.lock() {
                    if let Some(sender) = sender.take() { let _ = sender.send(serde_json::from_str::<serde_json::Value>(data)); }
                }
            }
        }).build().map_err(|error| error.to_string())?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(12), receiver).await;
    let _ = window.destroy();
    let data = result.map_err(|_| "StreamEast took too long to load.".to_string())?
        .map_err(|error| error.to_string())?.map_err(|error| error.to_string())?;
    if !data.get("matches").is_some_and(serde_json::Value::is_array) { return Err("StreamEast listings could not load.".into()); }
    *cache.lock().map_err(|error| error.to_string())? = Some((std::time::Instant::now(), data.clone()));
    Ok(data)
}

#[tauri::command]
pub async fn sports_open(
    app: AppHandle,
    url: String,
    source_index: usize,
    title: String,
    score: String,
    sources: Option<Vec<String>>,
) -> Result<(), String> {
    static WINDOW: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    let _guard = WINDOW
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    let url = event_url(&url)?;
    let sources = sources.unwrap_or_else(|| vec![url.to_string()]);
    if sources.is_empty() || sources.len() > 16 { return Err("Invalid sports source list".into()); }
    for source in &sources { event_url(source)?; }
    if let Some(window) = app.get_webview_window("sports") {
        window.destroy().map_err(|error| error.to_string())?;
    }
    crate::background::clear_unloading("sports");
    let main = app
        .get_webview_window("main")
        .ok_or("Luma window is unavailable")?;
    let size = main.inner_size().map_err(|error| error.to_string())?;
    let position = main.outer_position().map_err(|error| error.to_string())?;
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let navigation = app.clone();
    let script = format!("window.__lumaSportsSource = {source_index}; window.__lumaSportsTitle = {}; window.__lumaSportsScore = {};\n{}", serde_json::to_string(&title).map_err(|error| error.to_string())?, serde_json::to_string(&score).map_err(|error| error.to_string())?, include_str!("../../resources/sports.js"));
    let script = format!("window.__lumaSportsUrls = {};\n{script}", serde_json::to_string(&sources).map_err(|error| error.to_string())?);
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
            if url.scheme() == "https"
                && url.host_str() == Some("isportsurge.ws")
                && url.path() == "/__luma_close__"
            {
                let handle = navigation.clone();
                tauri::async_runtime::spawn(async move {
                    close(&handle);
                });
                return false;
            }
            event_url(url.as_str()).is_ok()
        })
        .build()
        .map_err(|error| format!("The sports stream could not open: {error}"))?;
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
    fn remote_shortcuts_have_icons_and_route_to_supported_sports_views() {
        let catalog = crate::commands::apps::catalog();
        for entry in super::shortcuts() {
            assert_eq!(catalog.iter().filter(|app| app.command == entry.command).count(), 1);
            assert_eq!(crate::commands::apps::app_command(&entry.command), Some(entry.name));
            let order = crate::remote::server::normalize_app_order(vec![entry.name.into(), "Netflix".into()]);
            assert_eq!(order.first().map(String::as_str), Some(entry.name));
            let league =
                super::shortcut_league(&entry.command).expect("shortcut command must be handled");
            assert!(league == "All" || super::league_path(league).is_ok());
            assert!(entry.icon.starts_with(b"\x89PNG"));
        }
        assert!(super::shortcut_league("sportsunknown").is_none());
    }

    #[test]
    fn only_source_event_pages_can_be_opened() {
        assert!(event_url("https://isportsurge.ws/watch/nba/game/123").is_ok());
        assert!(event_url("https://isportsurge.ws/event/nfl/redzone").is_ok());
        assert!(event_url("https://golygoals.com/streameast.html?id=60157").is_ok());
        assert!(event_url("https://streamec.st/embed/ch73.php").is_ok());
        assert!(event_url("https://golygoals.com/advert").is_err());
        for url in [
            "http://isportsurge.ws/watch/a",
            "https://isportsurge.ws.evil.com/watch/a",
            "https://isportsurge.ws/index13",
            "https://user@isportsurge.ws/watch/a",
            "https://isportsurge.ws:444/watch/a",
        ] {
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
    let client = CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .unwrap_or_default()
    });
    client
        .get(format!(
            "https://site.api.espn.com/apis/site/v2/sports/{path}"
        ))
        .send()
        .await
        .map_err(|_| "Game information is unavailable.".to_string())?
        .error_for_status()
        .map_err(|_| "Game information is unavailable.".to_string())?
        .json()
        .await
        .map_err(|_| "Game information could not be read.".to_string())
}

#[tauri::command]
pub async fn sports_scoreboard(league: String) -> Result<serde_json::Value, String> {
    data_request(&format!(
        "{}/scoreboard?dates={}",
        league_path(&league)?,
        chrono::Local::now().format("%Y%m%d")
    ))
    .await
}

#[tauri::command]
pub async fn sports_teams(league: String) -> Result<serde_json::Value, String> {
    data_request(&format!("{}/teams?limit=100", league_path(&league)?)).await
}

#[tauri::command]
pub async fn sports_artwork(league: String, event_id: String) -> Result<Option<String>, String> {
    if event_id.is_empty() || !event_id.chars().all(|character| character.is_ascii_digit()) {
        return Err("Invalid game identifier.".into());
    }
    let data = data_request(&format!(
        "{}/summary?event={event_id}",
        league_path(&league)?
    ))
    .await?;
    Ok(data
        .pointer("/gameInfo/venue/images/0/href")
        .and_then(serde_json::Value::as_str)
        .filter(|url| url.starts_with("https://"))
        .map(str::to_string))
}

#[tauri::command]
pub fn sports_update_score(app: AppHandle, score: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("sports") {
        let score = serde_json::to_string(&score).map_err(|error| error.to_string())?;
        window
            .eval(format!("window.__lumaSportsScoreUpdate?.({score});"))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub async fn control(app: &AppHandle, action: &str) -> Result<(), String> {
    if !matches!(action, "toggle" | "previous" | "next") {
        return Err("Unknown playback command.".into());
    }
    let window = app
        .get_webview_window("sports")
        .ok_or("The sports player is closed.")?;
    let action = serde_json::to_string(action).map_err(|error| error.to_string())?;
    window
        .eval(format!("window.__lumaSportsControl?.({action});"))
        .map_err(|error| error.to_string())
}

pub async fn timeline(app: &AppHandle, position: Option<f64>) -> Result<serde_json::Value, String> {
    let window = app
        .get_webview_window("sports")
        .ok_or("The sports player is closed.")?;
    let position = serde_json::to_string(&position).map_err(|error| error.to_string())?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = std::sync::Mutex::new(Some(sender));
    window
        .eval_with_callback(
            format!("window.__lumaSportsTimeline?.({position}) ?? null"),
            move |value| {
                if let Ok(mut sender) = sender.lock() {
                    if let Some(sender) = sender.take() {
                        let _ = sender.send(value);
                    }
                }
            },
        )
        .map_err(|error| error.to_string())?;
    let value = tokio::time::timeout(std::time::Duration::from_secs(2), receiver)
        .await
        .map_err(|_| "The sports player did not respond.".to_string())?
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&value).map_err(|error| error.to_string())
}
