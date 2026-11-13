use crate::{
    commands::{apps, media, youtube},
    remote::server::{self, NavigationCommand},
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    enabled: bool,
    token: String,
}
struct Pairing {
    code: String,
    expires: Instant,
    attempts: u8,
}
impl Pairing {
    fn verify(&mut self, code: &str, now: Instant) -> Result<(), StatusCode> {
        if self.expires <= now || self.attempts >= 10 {
            return Err(StatusCode::FORBIDDEN);
        }
        self.attempts += 1;
        if same(code, &self.code) {
            Ok(())
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
#[derive(Default)]
pub struct Frame {
    saved: Mutex<Saved>,
    pairing: Mutex<Option<Pairing>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    enabled: bool,
    paired: bool,
    addresses: Vec<String>,
    pc_name: String,
    code: Option<String>,
}

fn pc_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Luma PC".into())
}
fn path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("display-box.json"))
}
fn save(app: &tauri::AppHandle, saved: &Saved) -> Result<(), String> {
    std::fs::write(
        path(app)?,
        serde_json::to_vec(saved).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn random_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |n, (a, b)| n | (a ^ b)) == 0
}
fn authorized(app: &tauri::AppHandle, headers: &HeaderMap) -> Result<(), StatusCode> {
    if headers.contains_key("origin") {
        return Err(StatusCode::FORBIDDEN);
    }
    let frame = app.state::<Frame>();
    let saved = frame
        .saved
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let candidate = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !saved.enabled || saved.token.is_empty() || !same(candidate, &saved.token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairRequest {
    code: String,
}
async fn pair(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<PairRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if headers.contains_key("origin") {
        return Err(StatusCode::FORBIDDEN);
    }
    let frame = app.state::<Frame>();
    let mut pairing = frame
        .pairing
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pending = pairing.as_mut().ok_or(StatusCode::FORBIDDEN)?;
    pending.verify(&request.code, Instant::now())?;
    let mut saved = frame
        .saved
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let replacement = Saved {
        enabled: saved.enabled,
        token: random_token().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };
    if !replacement.enabled {
        return Err(StatusCode::FORBIDDEN);
    }
    save(&app, &replacement).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *saved = replacement;
    *pairing = None;
    Ok(Json(
        serde_json::json!({"token": saved.token, "pcName": pc_name()}),
    ))
}
async fn state(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    authorized(&app, &headers)?;
    Ok(Json(
        serde_json::json!({"pcName": pc_name(), "connected": true}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    command: String,
}
async fn command(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<Command>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    authorized(&app, &headers)?;
    if let Ok(nav) = serde_json::from_value::<NavigationCommand>(serde_json::json!(request.command))
    {
        server::navigate(&app, nav);
        return Ok(Json(serde_json::json!({"accepted": true})));
    }
    let result = match request.command.as_str() {
        "youtube" => youtube::open_youtube(app, true).await,
        "stremio" => apps::launch_app("Stremio".into(), app.state::<apps::Apps>(), app.clone()),
        "custom" => apps::launch_app("Custom".into(), app.state::<apps::Apps>(), app.clone()),
        "spotify" => apps::launch_app("Spotify".into(), app.state::<apps::Apps>(), app.clone()),
        "toggle" | "previous" | "next" => media::media_control(app, request.command).await,
        "volumeup" | "volumedown" | "mute" => volume(&request.command),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    Ok(Json(match result {
        Ok(()) => serde_json::json!({"accepted": true}),
        Err(error) => serde_json::json!({"error":error}),
    }))
}
#[cfg(windows)]
fn volume(action: &str) -> Result<(), String> {
    #[link(name = "user32")]
    extern "system" {
        fn keybd_event(key: u8, scan: u8, flags: u32, extra: usize);
    }
    let key = match action {
        "volumeup" => 0xAF,
        "volumedown" => 0xAE,
        _ => 0xAD,
    };
    unsafe {
        keybd_event(key, 0, 0, 0);
        keybd_event(key, 0, 2, 0);
    }
    Ok(())
}
#[cfg(not(windows))]
fn volume(_: &str) -> Result<(), String> {
    Err("PC volume requires Windows.".into())
}

async fn start(app: &tauri::AppHandle) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9877")
        .await
        .map_err(|e| format!("Cannot listen on port 9877: {e}"))?;
    let router = Router::new()
        .route("/pair", post(pair))
        .route("/state", get(state))
        .route("/command", post(command))
        .layer(DefaultBodyLimit::max(512))
        .with_state(app.clone());
    let task = tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    *app.state::<Frame>()
        .task
        .lock()
        .map_err(|e| e.to_string())? = Some(task);
    Ok(())
}
pub async fn initialize(app: tauri::AppHandle) -> Result<(), String> {
    let file = path(&app)?;
    let saved: Saved = if file.exists() {
        serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        Saved::default()
    };
    let enabled = saved.enabled;
    *app.state::<Frame>()
        .saved
        .lock()
        .map_err(|e| e.to_string())? = saved;
    if enabled {
        start(&app).await?;
    }
    Ok(())
}
#[tauri::command]
pub fn frame_status(app: tauri::AppHandle) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    let (enabled, paired) = {
        let saved = frame.saved.lock().map_err(|e| e.to_string())?;
        (saved.enabled, !saved.token.is_empty())
    };
    let pairing = frame.pairing.lock().map_err(|e| e.to_string())?;
    let addresses = local_addresses();
    Ok(Status {
        enabled: enabled && frame.task.lock().map_err(|e| e.to_string())?.is_some(),
        paired,
        addresses,
        pc_name: pc_name(),
        code: pairing
            .as_ref()
            .filter(|p| p.expires > Instant::now() && p.attempts < 10)
            .map(|p| p.code.clone()),
    })
}
fn local_addresses() -> Vec<String> {
    let mut addresses = Vec::new();
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("192.0.2.1:80").is_ok() {
            if let Ok(address) = socket.local_addr() {
                addresses.push(address.ip().to_string());
            }
        }
    }
    addresses
}
#[tauri::command]
pub async fn configure_frame(app: tauri::AppHandle, enabled: bool) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    if enabled {
        let running = frame.task.lock().map_err(|e| e.to_string())?.is_some();
        if !running {
            start(&app).await?;
        }
    }
    {
        let mut saved = frame.saved.lock().map_err(|e| e.to_string())?;
        save(
            &app,
            &Saved {
                enabled,
                token: saved.token.clone(),
            },
        )?;
        saved.enabled = enabled;
    }
    if !enabled {
        if let Some(task) = frame.task.lock().map_err(|e| e.to_string())?.take() {
            task.abort();
        }
        *frame.pairing.lock().map_err(|e| e.to_string())? = None;
    }
    frame_status(app)
}
#[tauri::command]
pub fn frame_pairing(app: tauri::AppHandle) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    if !frame.saved.lock().map_err(|e| e.to_string())?.enabled {
        return Err("Enable Display Box first.".into());
    }
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    let code = format!("{:06}", u32::from_le_bytes(bytes) % 1_000_000);
    *frame.pairing.lock().map_err(|e| e.to_string())? = Some(Pairing {
        code,
        expires: Instant::now() + Duration::from_secs(300),
        attempts: 0,
    });
    frame_status(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_expires_and_locks_after_ten_guesses() {
        let now = Instant::now();
        let mut pairing = Pairing {
            code: "012345".into(),
            expires: now + Duration::from_secs(300),
            attempts: 0,
        };
        for _ in 0..10 {
            assert_eq!(pairing.verify("999999", now), Err(StatusCode::UNAUTHORIZED));
        }
        assert_eq!(pairing.verify("012345", now), Err(StatusCode::FORBIDDEN));
        pairing.attempts = 0;
        assert!(pairing.verify("012345", now).is_ok());
        assert_eq!(
            pairing.verify("012345", now + Duration::from_secs(300)),
            Err(StatusCode::FORBIDDEN)
        );
    }
    #[test]
    fn command_cannot_inject_arguments() {
        assert!(serde_json::from_str::<Command>(r#"{"command":"home","path":"cmd.exe"}"#).is_err());
    }
}
