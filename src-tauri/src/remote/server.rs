use crate::commands::youtube;
use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        DefaultBodyLimit, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tokio::sync::{watch, Semaphore};

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    running: bool,
    lan: bool,
    address: String,
    token: String,
    origin: String,
    error: Option<String>,
}

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
    if app.get_webview_window("youtube").is_some() {
        youtube::forward(app, command.as_str());
        if matches!(command, NavigationCommand::Home) {
            let _ = app.emit("remote-navigation", command);
        }
    } else {
        let _ = app.emit("remote-navigation", command);
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    command: NavigationCommand,
}

struct Running {
    stop: watch::Sender<bool>,
    task: tauri::async_runtime::JoinHandle<()>,
}

pub struct Remote {
    status: Mutex<RemoteStatus>,
    state: Mutex<AppState>,
    running: tokio::sync::Mutex<Option<Running>>,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            status: Mutex::new(RemoteStatus::default()),
            state: Mutex::new(AppState {
                view: "Home".into(),
                fullscreen: false,
            }),
            running: tokio::sync::Mutex::new(None),
        }
    }
}

#[derive(Clone)]
struct ServerState {
    app: tauri::AppHandle,
    token: String,
    origin: String,
    stop: watch::Receiver<bool>,
    clients: Arc<Semaphore>,
}

fn authorized(
    headers: &HeaderMap,
    token: &str,
    origin: &str,
    websocket: bool,
) -> Result<(), StatusCode> {
    if (websocket || headers.contains_key("origin"))
        && headers.get("origin").and_then(|v| v.to_str().ok()) != Some(origin)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let protocol = if websocket {
        headers
            .get("sec-websocket-protocol")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.split(',')
                    .map(str::trim)
                    .find_map(|v| v.strip_prefix("luma."))
            })
    } else {
        None
    };
    let candidate = bearer.or(protocol).ok_or(StatusCode::UNAUTHORIZED)?;
    let equal = candidate.len() == token.len()
        && candidate
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0;
    if !equal {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

fn check_origin(origin: &str) -> Result<String, String> {
    let parsed: axum::http::Uri = origin
        .parse()
        .map_err(|_| "Enter an exact http:// or https:// origin.".to_string())?;
    if !matches!(parsed.scheme_str(), Some("http" | "https"))
        || parsed.authority().is_none()
        || parsed.authority().is_some_and(|a| a.as_str().contains('@'))
        || parsed.path_and_query().is_some_and(|p| p.as_str() != "/")
    {
        return Err("Use an exact HTTP(S) origin without a path, query, or credentials.".into());
    }
    Ok(format!(
        "{}://{}",
        parsed.scheme_str().unwrap(),
        parsed.authority().unwrap()
    ))
}

async fn state(
    State(server): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<AppState>, StatusCode> {
    authorized(&headers, &server.token, &server.origin, false)?;
    let remote = server.app.state::<Remote>();
    let state = remote
        .state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .clone();
    Ok(Json(state))
}

async fn command(
    State(server): State<ServerState>,
    headers: HeaderMap,
    Json(command): Json<Command>,
) -> Result<StatusCode, StatusCode> {
    authorized(&headers, &server.token, &server.origin, false)?;
    navigate(&server.app, command.command);
    Ok(StatusCode::ACCEPTED)
}

async fn websocket(
    State(server): State<ServerState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(status) = authorized(&headers, &server.token, &server.origin, true) {
        return status.into_response();
    }
    let Ok(permit) = server.clients.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    let protocol = format!("luma.{}", server.token);
    ws.protocols([protocol]).max_message_size(512).max_frame_size(512).read_buffer_size(1024).on_upgrade(move |mut socket| async move {
        let _permit = permit;
        let mut stop = server.stop.clone();
        let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
        loop {
            tokio::select! {
                _ = stop.changed() => { let _ = socket.send(Message::Close(None)).await; break; }
                message = socket.recv() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if last.elapsed() < std::time::Duration::from_millis(30) { continue; }
                            last = std::time::Instant::now();
                            let response = match serde_json::from_str::<Command>(&text) {
                                Ok(command) => {
                                    navigate(&server.app, command.command);
                                    let state = server.app.state::<Remote>().state.lock().ok().map(|s| s.clone());
                                    serde_json::json!({"accepted": true, "state": state}).to_string()
                                },
                                Err(_) => "{\"error\":\"Invalid navigation command\"}".into(),
                            };
                            if socket.send(Message::Text(response.into())).await.is_err() { break; }
                        }
                        Some(Ok(Message::Ping(data))) => { if socket.send(Message::Pong(data)).await.is_err() { break; } }
                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                        _ => {}
                    }
                }
            }
        }
    })
}

pub async fn start(
    app: tauri::AppHandle,
    lan: bool,
    origin: String,
) -> Result<RemoteStatus, String> {
    let origin = check_origin(&origin)?;
    let remote = app.state::<Remote>();
    let mut running = remote.running.lock().await;
    if let Some(mut old) = running.take() {
        let _ = old.stop.send(true);
        if tokio::time::timeout(std::time::Duration::from_secs(2), &mut old.task)
            .await
            .is_err()
        {
            old.task.abort();
        }
    }
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|e| format!("Cannot generate remote credential: {e}"))?;
    let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let address = if lan {
        "0.0.0.0:9876"
    } else {
        "127.0.0.1:9876"
    };
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            let status = RemoteStatus {
                error: Some(format!("Cannot listen on {address}: {error}")),
                ..Default::default()
            };
            *remote.status.lock().map_err(|e| e.to_string())? = status.clone();
            let _ = app.emit("remote-status", &status);
            return Ok(status);
        }
    };
    let (stop, mut stopped) = watch::channel(false);
    let server = ServerState {
        app: app.clone(),
        token: token.clone(),
        origin: origin.clone(),
        stop: stop.subscribe(),
        clients: Arc::new(Semaphore::new(4)),
    };
    let router = Router::new()
        .route("/state", get(state))
        .route("/command", post(command))
        .route("/ws", get(websocket))
        .layer(DefaultBodyLimit::max(512))
        .with_state(server);
    let handle = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let result = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = stopped.changed().await;
            })
            .await;
        if let Err(error) = result {
            if let Ok(mut status) = handle.state::<Remote>().status.lock() {
                status.running = false;
                status.error = Some(error.to_string());
            }
        }
    });
    *running = Some(Running { stop, task });
    let status = RemoteStatus {
        running: true,
        lan,
        address: address.into(),
        token,
        origin,
        error: None,
    };
    *remote.status.lock().map_err(|e| e.to_string())? = status.clone();
    let _ = app.emit("remote-status", &status);
    Ok(status)
}

pub async fn initialize(app: tauri::AppHandle) {
    if let Err(error) = start(app.clone(), false, "http://localhost:9876".into()).await {
        let status = RemoteStatus {
            error: Some(error),
            ..Default::default()
        };
        if let Ok(mut current) = app.state::<Remote>().status.lock() {
            *current = status.clone();
        }
        let _ = app.emit("remote-status", status);
    }
}

#[tauri::command]
pub fn remote_status(remote: tauri::State<Remote>) -> Result<RemoteStatus, String> {
    Ok(remote.status.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub async fn configure_remote(
    app: tauri::AppHandle,
    lan: bool,
    origin: String,
) -> Result<RemoteStatus, String> {
    start(app, lan, origin).await
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

pub fn stop(app: &tauri::AppHandle) {
    if let Ok(running) = app.state::<Remote>().running.try_lock() {
        if let Some(server) = running.as_ref() {
            let _ = server.stop.send(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentication_and_origin_are_both_required_for_websocket() {
        let mut headers = HeaderMap::new();
        assert_eq!(
            authorized(&headers, "secret", "https://frame.local", true),
            Err(StatusCode::FORBIDDEN)
        );
        headers.insert("origin", "https://frame.local".parse().unwrap());
        assert_eq!(
            authorized(&headers, "secret", "https://frame.local", true),
            Err(StatusCode::UNAUTHORIZED)
        );
        headers.insert("authorization", "Bearer wrong".parse().unwrap());
        assert!(authorized(&headers, "secret", "https://frame.local", true).is_err());
        headers.insert("authorization", "Bearer secret".parse().unwrap());
        assert!(authorized(&headers, "secret", "https://frame.local", true).is_ok());
        headers.insert("origin", "https://evil.local".parse().unwrap());
        assert!(authorized(&headers, "secret", "https://frame.local", true).is_err());
    }
    #[test]
    fn websocket_subprotocol_and_http_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", "https://frame.local".parse().unwrap());
        headers.insert("sec-websocket-protocol", "luma.secret".parse().unwrap());
        assert!(authorized(&headers, "secret", "https://frame.local", true).is_ok());
        assert!(authorized(&headers, "secret", "https://frame.local", false).is_err());
        headers.remove("origin");
        headers.insert("authorization", "Bearer secret".parse().unwrap());
        assert!(authorized(&headers, "secret", "https://frame.local", false).is_ok());
    }
    #[test]
    fn rejects_arbitrary_commands_and_untrusted_origins() {
        assert!(
            serde_json::from_str::<Command>(r#"{"command":"launch","path":"cmd.exe"}"#).is_err()
        );
        assert!(serde_json::from_str::<Command>(r#"{"command":"home","path":"cmd.exe"}"#).is_err());
        for origin in [
            "null",
            "*",
            "file:///frame",
            "https://frame/path",
            "https://user@frame",
            "https://frame?token=x",
        ] {
            assert!(check_origin(origin).is_err(), "{origin}");
        }
        assert_eq!(
            check_origin("https://frame.local").unwrap(),
            "https://frame.local"
        );
    }
}
