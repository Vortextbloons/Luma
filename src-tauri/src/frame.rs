use crate::{
    commands::{apps, media, youtube},
    remote::server::{self, NavigationCommand},
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

const PHONE_COOKIE: &str = "luma_phone";
const PHONE_LOCAL_URL: &str = "http://localhost:9877/";
const PHONE: &str = include_str!("phone.html");
const PHONE_CSP: &str = "default-src 'none'; connect-src 'self'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    enabled: bool,
    token: String,
    #[serde(default)]
    phone_token: String,
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
    phone_pairing: Mutex<Option<Pairing>>,
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
    phone_paired: bool,
    phone_url: String,
    phone_code: Option<String>,
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
fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(';').find_map(|part| {
                let (key, value) = part.trim().split_once('=')?;
                (key == name).then_some(value)
            })
        })
}
fn same_origin(headers: &HeaderMap) -> Result<(), StatusCode> {
    if headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value != "same-origin" && value != "none")
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| url::Url::parse(value).ok())
        .ok_or(StatusCode::FORBIDDEN)?;
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| url::Url::parse(&format!("http://{value}")).ok())
        .ok_or(StatusCode::FORBIDDEN)?;
    if origin.scheme() != "http"
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.host_str() != host.host_str()
        || origin.port_or_known_default() != host.port_or_known_default()
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
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
fn phone_authorized(app: &tauri::AppHandle, headers: &HeaderMap) -> Result<(), StatusCode> {
    let frame = app.state::<Frame>();
    let saved = frame
        .saved
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let candidate = cookie(headers, PHONE_COOKIE).unwrap_or("");
    if !saved.enabled || saved.phone_token.is_empty() || !same(candidate, &saved.phone_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}
fn remote_authorized(app: &tauri::AppHandle, headers: &HeaderMap) -> Result<(), StatusCode> {
    if phone_authorized(app, headers).is_ok() {
        return Ok(());
    }
    authorized(app, headers)
}
fn rotate_display(saved: &Saved) -> Result<Saved, String> {
    Ok(Saved {
        enabled: saved.enabled,
        token: random_token()?,
        phone_token: saved.phone_token.clone(),
    })
}
fn rotate_phone(saved: &Saved) -> Result<Saved, String> {
    Ok(Saved {
        enabled: saved.enabled,
        token: saved.token.clone(),
        phone_token: random_token()?,
    })
}
fn new_pairing() -> Result<Pairing, String> {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(Pairing {
        code: format!("{:06}", u32::from_le_bytes(bytes) % 1_000_000),
        expires: Instant::now() + Duration::from_secs(300),
        attempts: 0,
    })
}
fn active_code(pairing: &Option<Pairing>) -> Option<String> {
    pairing
        .as_ref()
        .filter(|p| p.expires > Instant::now() && p.attempts < 10)
        .map(|p| p.code.clone())
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
    let mut saved = frame
        .saved
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !saved.enabled {
        return Err(StatusCode::FORBIDDEN);
    }
    {
        let mut pairing = frame
            .pairing
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let pending = pairing.as_mut().ok_or(StatusCode::FORBIDDEN)?;
        pending.verify(&request.code, Instant::now())?;
    }
    let replacement = rotate_display(&saved).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    save(&app, &replacement).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *saved = replacement;
    *frame
        .pairing
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? = None;
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
async fn perform(app: &tauri::AppHandle, command: &str) -> Option<Result<(), String>> {
    if let Ok(nav) = serde_json::from_value::<NavigationCommand>(serde_json::json!(command)) {
        server::navigate(app, nav);
        return Some(Ok(()));
    }
    let result = match command {
        "toggle" | "previous" | "next" => media::control(app, command).await,
        "volumeup" | "volumedown" | "mute" => volume(command),
        "youtube" => youtube::open_youtube(app.clone(), true).await,
        _ => match apps::app_command(command) {
            Some(name) => {
                apps::launch_app(name.into(), app.state::<apps::Apps>(), app.clone()).await
            }
            None => return None,
        },
    };
    Some(result)
}
async fn command(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<Command>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    authorized(&app, &headers)?;
    let result = perform(&app, &request.command)
        .await
        .ok_or(StatusCode::BAD_REQUEST)?;
    Ok(Json(match result {
        Ok(()) => serde_json::json!({"accepted": true}),
        Err(error) => serde_json::json!({"error":error}),
    }))
}
async fn phone_page() -> Response {
    Response::builder()
        .header("content-type", "text/html; charset=utf-8")
        .header("cache-control", "no-store")
        .header("content-security-policy", PHONE_CSP)
        .header("x-content-type-options", "nosniff")
        .header("x-frame-options", "DENY")
        .header("referrer-policy", "no-referrer")
        .body(Body::from(PHONE))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
async fn phone_pair(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<PairRequest>,
) -> Result<Response, StatusCode> {
    same_origin(&headers)?;
    let frame = app.state::<Frame>();
    let mut saved = frame
        .saved
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !saved.enabled {
        return Err(StatusCode::FORBIDDEN);
    }
    {
        let mut pairing = frame
            .phone_pairing
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let pending = pairing.as_mut().ok_or(StatusCode::FORBIDDEN)?;
        pending.verify(&request.code, Instant::now())?;
    }
    let replacement = rotate_phone(&saved).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    save(&app, &replacement).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *saved = replacement;
    *frame
        .phone_pairing
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? = None;
    let cookie = format!(
        "{PHONE_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=31536000",
        saved.phone_token
    );
    let mut response = Json(serde_json::json!({"pcName": pc_name()})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}
async fn phone_state(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    phone_authorized(&app, &headers)?;
    let state = server::snapshot(&app).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        serde_json::json!({"pcName": pc_name(), "state": state}),
    ))
}
async fn apps_list(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    remote_authorized(&app, &headers)?;
    let apps: Vec<serde_json::Value> = apps::catalog()
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "command": entry.command,
                "name": entry.name,
                "icon": format!("/apps/{}/icon", entry.command),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "apps": apps })))
}
async fn app_icon(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    axum::extract::Path(command): axum::extract::Path<String>,
) -> Result<Response, StatusCode> {
    remote_authorized(&app, &headers)?;
    let entry = apps::catalog()
        .into_iter()
        .find(|entry| entry.command == command)
        .ok_or(StatusCode::NOT_FOUND)?;
    Response::builder()
        .header("content-type", "image/png")
        .header("cache-control", "private, max-age=86400")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(entry.icon))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
async fn phone_command(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<Command>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    same_origin(&headers)?;
    phone_authorized(&app, &headers)?;
    let result = perform(&app, &request.command)
        .await
        .ok_or(StatusCode::BAD_REQUEST)?;
    let state = server::snapshot(&app).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(match result {
        Ok(()) => serde_json::json!({"accepted": true, "state": state}),
        Err(error) => serde_json::json!({"error": error, "state": state}),
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
        .route("/", get(phone_page))
        .route("/pair", post(pair))
        .route("/state", get(state))
        .route("/command", post(command))
        .route("/phone/pair", post(phone_pair))
        .route("/phone/state", get(phone_state))
        .route("/phone/command", post(phone_command))
        .route("/apps", get(apps_list))
        .route("/apps/{command}/icon", get(app_icon))
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
    let (enabled, paired, phone_paired) = {
        let saved = frame.saved.lock().map_err(|e| e.to_string())?;
        (
            saved.enabled,
            !saved.token.is_empty(),
            !saved.phone_token.is_empty(),
        )
    };
    let pairing = frame.pairing.lock().map_err(|e| e.to_string())?;
    let phone_pairing = frame.phone_pairing.lock().map_err(|e| e.to_string())?;
    let addresses = local_addresses();
    Ok(Status {
        enabled: enabled && frame.task.lock().map_err(|e| e.to_string())?.is_some(),
        paired,
        phone_url: addresses
            .first()
            .map(|address| format!("http://{address}:9877"))
            .unwrap_or_else(|| "http://localhost:9877".into()),
        addresses,
        pc_name: pc_name(),
        code: active_code(&pairing),
        phone_paired,
        phone_code: active_code(&phone_pairing),
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
        saved.enabled = enabled;
        save(&app, &saved)?;
    }
    if !enabled {
        if let Some(task) = frame.task.lock().map_err(|e| e.to_string())?.take() {
            task.abort();
        }
        *frame.pairing.lock().map_err(|e| e.to_string())? = None;
        *frame.phone_pairing.lock().map_err(|e| e.to_string())? = None;
    }
    frame_status(app)
}
#[tauri::command]
pub fn frame_pairing(app: tauri::AppHandle) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    if !frame.saved.lock().map_err(|e| e.to_string())?.enabled {
        return Err("Enable local remote access first.".into());
    }
    *frame.pairing.lock().map_err(|e| e.to_string())? = Some(new_pairing()?);
    frame_status(app)
}
#[tauri::command]
pub fn phone_pairing(app: tauri::AppHandle) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    if !frame.saved.lock().map_err(|e| e.to_string())?.enabled {
        return Err("Enable local remote access first.".into());
    }
    *frame.phone_pairing.lock().map_err(|e| e.to_string())? = Some(new_pairing()?);
    frame_status(app)
}

#[tauri::command]
pub fn phone_remote_qr(app: tauri::AppHandle) -> Result<String, String> {
    let status = frame_status(app)?;
    if !status.enabled || status.addresses.is_empty() {
        return Err("Connect this PC to Wi-Fi and enable remote control first.".into());
    }
    let code = qrcode::QrCode::new(status.phone_url.as_bytes()).map_err(|error| error.to_string())?;
    let svg = code.render::<qrcode::render::svg::Color>().build();
    Ok(format!("data:image/svg+xml,{}", percent_encoding::utf8_percent_encode(&svg, percent_encoding::NON_ALPHANUMERIC)))
}
#[tauri::command]
pub fn open_phone_remote(app: tauri::AppHandle) -> Result<(), String> {
    let frame = app.state::<Frame>();
    let running = frame.task.lock().map_err(|e| e.to_string())?.is_some();
    let enabled = frame.saved.lock().map_err(|e| e.to_string())?.enabled;
    if !running || !enabled {
        return Err("Enable local remote access first.".into());
    }
    app.opener()
        .open_url(PHONE_LOCAL_URL, None::<&str>)
        .map_err(|error| format!("Could not open the phone remote: {error}"))
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
    #[test]
    fn legacy_display_box_file_loads_without_a_phone_credential() {
        let saved: Saved =
            serde_json::from_str(r#"{"enabled":true,"token":"saved"}"#).unwrap();
        assert!(saved.enabled);
        assert_eq!(saved.token, "saved");
        assert!(saved.phone_token.is_empty());
    }
    #[test]
    fn pairing_one_device_does_not_replace_the_other_credential() {
        let saved = Saved {
            enabled: true,
            token: "display".into(),
            phone_token: "phone".into(),
        };
        let display = rotate_display(&saved).unwrap();
        assert_eq!(display.phone_token, "phone");
        assert_eq!(display.token.len(), 64);
        let phone = rotate_phone(&saved).unwrap();
        assert_eq!(phone.token, "display");
        assert_eq!(phone.phone_token.len(), 64);
    }
    #[test]
    fn phone_requests_must_come_from_the_served_origin() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "192.168.1.5:9877".parse().unwrap());
        headers.insert("origin", "http://192.168.1.5:9877".parse().unwrap());
        assert!(same_origin(&headers).is_ok());
        headers.insert("host", "localhost:9877".parse().unwrap());
        headers.insert("origin", "http://localhost:9877".parse().unwrap());
        assert!(same_origin(&headers).is_ok());
        headers.insert("origin", "http://127.0.0.1:9877".parse().unwrap());
        assert!(same_origin(&headers).is_err());
        headers.insert("origin", "http://evil.example".parse().unwrap());
        assert!(same_origin(&headers).is_err());
        headers.insert("origin", "http://localhost:9877".parse().unwrap());
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());
        assert!(same_origin(&headers).is_err());
        headers.remove("sec-fetch-site");
        headers.remove("origin");
        assert!(same_origin(&headers).is_err());
    }
    #[test]
    fn phone_cookie_is_read_from_the_cookie_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            "other=1; luma_phone=abc123; last=2".parse().unwrap(),
        );
        assert_eq!(cookie(&headers, PHONE_COOKIE), Some("abc123"));
        assert_eq!(cookie(&headers, "missing"), None);
    }
    #[test]
    fn phone_page_is_served_with_browser_security_headers() {
        tauri::async_runtime::block_on(async {
            let response = phone_page().await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers()["content-type"],
                "text/html; charset=utf-8"
            );
            assert!(response.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'"));
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert!(String::from_utf8_lossy(&body).contains("Pair this phone"));
        });
    }
}
