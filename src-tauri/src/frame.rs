use crate::{
    commands::{apps, media, youtube},
    remote::{
        credentials,
        server::{self, NavigationCommand},
    },
};
use axum::{
    body::Body,
    extract::{ws::{Message, WebSocketUpgrade}, DefaultBodyLimit, State},
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
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

const PHONE_COOKIE: &str = "luma_phone";
const PHONE_COOKIE_MAX_AGE: u32 = 400 * 24 * 60 * 60;
const PHONE_LOCAL_URL: &str = "http://localhost:9877/";
const PHONE: &str = include_str!("phone.html");
const MANIFEST: &str = include_str!("web/manifest.webmanifest");
const WEB_ICON_192: &[u8] = include_bytes!("web/luma-192.png");
const WEB_ICON_512: &[u8] = include_bytes!("web/luma-512.png");
const WEB_ICON_MASKABLE: &[u8] = include_bytes!("web/luma-maskable-512.png");
const WEB_ICON_APPLE: &[u8] = include_bytes!("web/apple-touch-icon.png");
const PHONE_CSP: &str = "default-src 'none'; connect-src 'self' ws:; manifest-src 'self'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
const DISPLAY_CREDENTIAL: &str = "display";
const PHONE_CREDENTIAL: &str = "phone";

#[derive(Default, Deserialize)]
struct Saved {
    enabled: bool,
    #[serde(default)]
    token: String,
    #[serde(default)]
    phone_token: String,
}

#[derive(Serialize)]
struct Disk {
    enabled: bool,
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
        if same(code, &self.code) {
            Ok(())
        } else {
            self.attempts += 1;
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
    persist(DISPLAY_CREDENTIAL, &saved.token)?;
    persist(PHONE_CREDENTIAL, &saved.phone_token)?;
    std::fs::write(
        path(app)?,
        serde_json::to_vec(&Disk {
            enabled: saved.enabled,
        })
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn persist(kind: &str, token: &str) -> Result<(), String> {
    if token.is_empty() {
        credentials::delete(kind)
    } else {
        credentials::save(kind, token)
    }
}
fn random_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut difference = a.len() ^ b.len();
    for index in 0..a.len().max(b.len()) {
        difference |= usize::from(a.get(index).unwrap_or(&0) ^ b.get(index).unwrap_or(&0));
    }
    difference == 0
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
        phone_token: if saved.phone_token.is_empty() {
            random_token()?
        } else {
            saved.phone_token.clone()
        },
    })
}
fn clear_device(saved: &mut Saved, kind: &str) -> Result<(), String> {
    match kind {
        "display" => saved.token.clear(),
        "phone" => saved.phone_token.clear(),
        _ => return Err("Unknown device.".into()),
    }
    Ok(())
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
) -> Result<Response, StatusCode> {
    authorized(&app, &headers)?;
    let state = server::snapshot_live(&app).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut response = Json(serde_json::json!({"pcName": pc_name(), "connected": true, "state": state})).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    Ok(response)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    command: String,
}
async fn perform(app: &tauri::AppHandle, command: &str) -> Option<Result<(), String>> {
    if !["stop", "click", "fullscreen"].contains(&command) {
        if let Err(error) = server::wake(app).await { return Some(Err(error)); }
    }
    if crate::remote::pointer::hides_cursor(command) {
        crate::remote::pointer::hide_cursor(app);
    }
    if command == "fullscreen" {
        return Some(server::fullscreen(app));
    }
    if let Ok(nav) = serde_json::from_value::<NavigationCommand>(serde_json::json!(command)) {
        server::navigate(app, nav);
        return Some(Ok(()));
    }
    let result = match command {
        "stop" => server::stop(app),
        "click" => crate::remote::pointer::click(app),
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
    volume_feedback(&app, &request.command);
    let state = if request.command == "fullscreen" {
        server::snapshot_live(&app)
    } else {
        server::snapshot(&app)
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(match result {
        Ok(()) => serde_json::json!({"accepted": true, "state": state}),
        Err(error) => serde_json::json!({"error":error, "state": state}),
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
async fn web_manifest() -> Response {
    Response::builder()
        .header("content-type", "application/manifest+json")
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(MANIFEST))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
async fn web_icon(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Response, StatusCode> {
    let bytes: &'static [u8] = match name.as_str() {
        "luma-192.png" => WEB_ICON_192,
        "luma-512.png" => WEB_ICON_512,
        "luma-maskable-512.png" => WEB_ICON_MASKABLE,
        "apple-touch-icon.png" => WEB_ICON_APPLE,
        _ => return Err(StatusCode::NOT_FOUND),
    };
    Response::builder()
        .header("content-type", "image/png")
        .header("cache-control", "private, max-age=86400")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
    let cookie = format!(
        "{PHONE_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={PHONE_COOKIE_MAX_AGE}",
        saved.phone_token
    );
    let mut response = Json(serde_json::json!({"pcName": pc_name()})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}
async fn phone_disconnect(headers: HeaderMap) -> Result<Response, StatusCode> {
    same_origin(&headers)?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("luma_phone=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(response)
}
async fn phone_playback(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    payload: Option<Json<serde_json::Value>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if payload.is_some() { same_origin(&headers)?; }
    phone_authorized(&app, &headers)?;
    let position = match payload {
        Some(Json(value)) => Some(value.get("position").and_then(serde_json::Value::as_f64).filter(|value| value.is_finite() && *value >= 0.0).ok_or(StatusCode::BAD_REQUEST)?),
        None => None,
    };
    match media::timeline(&app, position).await {
        Ok(timeline) => Ok(Json(serde_json::json!({ "timeline": timeline }))),
        Err(error) => Ok(Json(serde_json::json!({ "error": error }))),
    }
}
async fn phone_state(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    phone_authorized(&app, &headers)?;
    let state = server::snapshot_live(&app).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        serde_json::json!({"pcName": pc_name(), "state": state}),
    ))
}
async fn apps_list(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    remote_authorized(&app, &headers)?;
    let order = server::snapshot(&app)
        .map(|state| state.app_order().to_vec())
        .unwrap_or_default();
    let apps: Vec<serde_json::Value> = apps_in_order(&order)
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "command": entry.command,
                "name": entry.name,
                "icon": format!("/apps/{}/icon", entry.command),
            })
        })
        .collect();
    let mut response = Json(serde_json::json!({ "apps": apps })).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    Ok(response)
}

fn apps_in_order(order: &[String]) -> Vec<apps::CatalogEntry> {
    let rank: std::collections::HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut catalog = apps::catalog();
    catalog.sort_by_key(|entry| rank.get(entry.name).copied().unwrap_or(usize::MAX));
    catalog
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutRequest {
    order: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingRequest {
    key: String,
    value: String,
}

async fn phone_setting(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<SettingRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    same_origin(&headers)?;
    phone_authorized(&app, &headers)?;
    server::change_setting(&app, request.key, request.value).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(serde_json::json!({"accepted": true})))
}

async fn phone_layout(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<LayoutRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    same_origin(&headers)?;
    phone_authorized(&app, &headers)?;
    if request.order.len() > 32 || request.order.iter().any(|name| name.len() > 64) {
        return Err(StatusCode::BAD_REQUEST);
    }
    server::set_app_order(&app, request.order)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let state = server::snapshot(&app).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"accepted": true, "state": state})))
}

async fn app_icon(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    axum::extract::Path(command): axum::extract::Path<String>,
) -> Result<Response, StatusCode> {    remote_authorized(&app, &headers)?;
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
async fn phone_sound(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    axum::extract::Path((preset, role)): axum::extract::Path<(String, String)>,
) -> Result<Response, StatusCode> {
    phone_authorized(&app, &headers)?;
    let bytes: &'static [u8] = match (preset.as_str(), role.as_str()) {
        ("real", "move") => include_bytes!("../../src/assets/sounds/real/move.wav"),
        ("real", "press") => include_bytes!("../../src/assets/sounds/real/press.wav"),
        ("real", "back") => include_bytes!("../../src/assets/sounds/real/back.wav"),
        ("real", "volume") => include_bytes!("../../src/assets/sounds/real/volume.wav"),
        ("real", "open") => include_bytes!("../../src/assets/sounds/real/open.wav"),
        ("real", "toggle") => include_bytes!("../../src/assets/sounds/real/toggle.wav"),
        ("google", "move") => include_bytes!("../../src/assets/sounds/google/move.wav"),
        ("google", "press") => include_bytes!("../../src/assets/sounds/google/press.wav"),
        ("google", "back") => include_bytes!("../../src/assets/sounds/google/back.wav"),
        ("google", "volume") => include_bytes!("../../src/assets/sounds/google/volume.wav"),
        ("google", "open") => include_bytes!("../../src/assets/sounds/google/open.wav"),
        ("google", "toggle") => include_bytes!("../../src/assets/sounds/google/toggle.wav"),
        ("roku", "move") => include_bytes!("../../src/assets/sounds/roku/move.wav"),
        ("roku", "press") => include_bytes!("../../src/assets/sounds/roku/press.wav"),
        ("roku", "back") => include_bytes!("../../src/assets/sounds/roku/back.wav"),
        ("roku", "volume") => include_bytes!("../../src/assets/sounds/roku/volume.wav"),
        ("roku", "open") => include_bytes!("../../src/assets/sounds/roku/open.wav"),
        ("roku", "toggle") => include_bytes!("../../src/assets/sounds/roku/toggle.wav"),
        _ => return Err(StatusCode::NOT_FOUND),
    };
    Response::builder()
        .header("content-type", "audio/wav")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(bytes))
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
    volume_feedback(&app, &request.command);
    let state = if request.command == "fullscreen" {
        server::snapshot_live(&app)
    } else {
        server::snapshot(&app)
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(match result {
        Ok(()) => serde_json::json!({"accepted": true, "state": state}),
        Err(error) => serde_json::json!({"error": error, "state": state}),
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PointerRequest {
    action: String,
    owner: String,
    #[serde(default)]
    x: f32,
    #[serde(default)]
    y: f32,
}
async fn pointer(
    State(app): State<tauri::AppHandle>,
    headers: HeaderMap,
    Json(request): Json<PointerRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if phone_authorized(&app, &headers).is_ok() { same_origin(&headers)?; }
    else { authorized(&app, &headers)?; }
    if request.owner.is_empty() || request.owner.len() > 64 || request.owner.chars().any(char::is_control) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let value = |axis: f32| if axis.is_finite() { axis.clamp(-1.0, 1.0) } else { 0.0 };
    let (x, y) = (value(request.x), value(request.y));
    let released = match request.action.as_str() {
        "start" => {
            crate::remote::pointer::start(&app, &request.owner, x, y);
            false
        }
        "move" => crate::remote::pointer::update(&app, &request.owner, x, y),
        "stop" => {
            crate::remote::pointer::stop(&app, &request.owner);
            false
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    Ok(Json(serde_json::json!({ "released": released })))
}
async fn phone_socket(
    State(app): State<tauri::AppHandle>, headers: HeaderMap, upgrade: WebSocketUpgrade,
) -> Result<Response, StatusCode> {
    same_origin(&headers)?;
    phone_authorized(&app, &headers)?;
    Ok(upgrade.max_message_size(4096).max_frame_size(4096).write_buffer_size(0).on_upgrade(move |mut socket| async move {
        let mut last_state: Option<String> = None;
        let mut appearance = app.state::<server::Remote>().subscribe_appearance();
        let mut app_order = app.state::<server::Remote>().subscribe_app_order();
        loop {
            tokio::select! {
                message = socket.recv() => {
                    let Some(Ok(message)) = message else { break; };
                    if phone_authorized(&app, &headers).is_err() { break; }
                    let Message::Text(text) = message else { continue; };
                    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else { break; };
                    let Some(id) = value.get("id").and_then(|id| id.as_u64()) else { break; };
                    let command = value.get("command").and_then(|command| command.as_str());
                    let response = match command {
                        Some(command) => {
                            let result = perform(&app, command).await.unwrap_or_else(|| Err("Unknown remote command.".into()));
                            volume_feedback(&app, command);
                            let state = if command == "fullscreen" {
                                server::snapshot_live(&app)
                            } else {
                                server::snapshot(&app)
                            };
                            last_state = state.as_ref().ok().and_then(|state| serde_json::to_string(state).ok());
                            serde_json::json!({ "id": id, "error": result.err(), "state": state.ok() })
                        }
                        None => match server::snapshot_live(&app) {
                            Ok(state) => {
                                let serialized = serde_json::to_string(&state).ok();
                                if serialized.is_some() && serialized == last_state {
                                    serde_json::json!({ "id": id })
                                } else {
                                    last_state = serialized;
                                    serde_json::json!({ "id": id, "state": state })
                                }
                            }
                            Err(_) => serde_json::json!({ "id": id }),
                        },
                    };
                    if socket.send(Message::Text(response.to_string().into())).await.is_err() { break; }
                }
                changed = appearance.changed() => {
                    if changed.is_err() { break; }
                    let value = appearance.borrow_and_update().clone();
                    if let Some(appearance) = value {
                        let response = serde_json::json!({ "appearance": appearance });
                        if socket.send(Message::Text(response.to_string().into())).await.is_err() { break; }
                    }
                }
                changed = app_order.changed() => {
                    if changed.is_err() { break; }
                    let order = app_order.borrow_and_update().clone();
                    let response = serde_json::json!({ "appOrder": order });
                    if socket.send(Message::Text(response.to_string().into())).await.is_err() { break; }
                }
            }
        }
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextRequest { text: String }

async fn remote_text(
    State(app): State<tauri::AppHandle>, headers: HeaderMap, Json(request): Json<TextRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if phone_authorized(&app, &headers).is_ok() { same_origin(&headers)?; }
    else { authorized(&app, &headers)?; }
    if request.text.is_empty() || request.text.len() > 1024 || request.text.chars().any(char::is_control) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let result = match server::wake(&app).await {
        Ok(()) => {
            if let Some(window) = app.get_webview_window("youtube").filter(|window| window.is_visible().unwrap_or(false)) {
                let text = serde_json::to_string(&request.text).map_err(|_| StatusCode::BAD_REQUEST)?;
                window.eval(format!("window.__lumaTypeSearch?.({text});")).map_err(|error| error.to_string())
            } else { server::type_text(&request.text) }
        },
        Err(error) => Err(error),
    };
    Ok(Json(match result {
        Ok(()) => serde_json::json!({"accepted": true}),
        Err(error) => serde_json::json!({"error": error}),
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

fn volume_feedback(app: &tauri::AppHandle, command: &str) {
    if matches!(command, "volumeup" | "volumedown" | "mute") {
        let _ = app.emit_to("main", "remote-volume", ());
    }
}

async fn secure_headers(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    headers
        .entry("x-content-type-options")
        .or_insert(HeaderValue::from_static("nosniff"));
    response
}
async fn start(app: &tauri::AppHandle) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9877")
        .await
        .map_err(|e| format!("Cannot listen on port 9877: {e}"))?;
    let router = Router::new()
        .route("/", get(phone_page))
        .route("/manifest.webmanifest", get(web_manifest))
        .route("/icons/{name}", get(web_icon))
        .route("/pair", post(pair))
        .route("/state", get(state))
        .route("/command", post(command))
        .route("/phone/pair", post(phone_pair))
        .route("/phone/disconnect", post(phone_disconnect))
        .route("/phone/state", get(phone_state))
        .route("/phone/playback", get(phone_playback).post(phone_playback))
        .route("/phone/layout", post(phone_layout))
        .route("/phone/settings", post(phone_setting))
        .route("/phone/ws", get(phone_socket))
        .route("/phone/command", post(phone_command))
        .route("/phone/sound/{preset}/{role}", get(phone_sound))
        .route("/pointer", post(pointer))
        .route("/text", post(remote_text))
        .route("/apps", get(apps_list))
        .route("/apps/{command}/icon", get(app_icon))
        .layer(DefaultBodyLimit::max(4096))
        .layer(axum::middleware::from_fn(secure_headers))
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
    let disk: Saved = if file.exists() {
        serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        Saved::default()
    };
    let saved = Saved {
        enabled: disk.enabled,
        token: credentials::read(DISPLAY_CREDENTIAL)?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| disk.token.clone()),
        phone_token: credentials::read(PHONE_CREDENTIAL)?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| disk.phone_token.clone()),
    };
    if !disk.token.is_empty() || !disk.phone_token.is_empty() {
        save(&app, &saved)?;
    }
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
    crate::network::local_addresses().unwrap_or_default()
}
#[tauri::command]
pub async fn configure_frame(app: tauri::AppHandle, enabled: bool) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    {
        let mut saved = frame.saved.lock().map_err(|e| e.to_string())?;
        let previous = saved.enabled;
        saved.enabled = enabled;
        if let Err(error) = save(&app, &saved) {
            saved.enabled = previous;
            return Err(error);
        }
    }
    if enabled {
        let running = frame.task.lock().map_err(|e| e.to_string())?.is_some();
        if !running {
            if let Err(error) = start(&app).await {
                if let Ok(mut saved) = frame.saved.lock() {
                    saved.enabled = false;
                    let _ = save(&app, &saved);
                }
                return Err(error);
            }
        }
    } else {
        crate::remote::pointer::stop_all(&app);
        crate::remote::pointer::show_cursor(&app);
        if let Some(task) = frame.task.lock().map_err(|e| e.to_string())?.take() {
            task.abort();
        }
        *frame.pairing.lock().map_err(|e| e.to_string())? = None;
        *frame.phone_pairing.lock().map_err(|e| e.to_string())? = None;
    }
    frame_status(app)
}
#[tauri::command]
pub fn forget_device(app: tauri::AppHandle, kind: String) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    {
        let mut saved = frame.saved.lock().map_err(|e| e.to_string())?;
        clear_device(&mut saved, &kind)?;
        save(&app, &saved)?;
    }
    crate::remote::pointer::stop_all(&app);
    crate::remote::pointer::show_cursor(&app);
    if kind == "display" {
        *frame.pairing.lock().map_err(|e| e.to_string())? = None;
    } else {
        *frame.phone_pairing.lock().map_err(|e| e.to_string())? = None;
    }
    frame_status(app)
}

#[tauri::command]
pub fn forget_remotes(app: tauri::AppHandle) -> Result<Status, String> {
    let frame = app.state::<Frame>();
    {
        let mut saved = frame.saved.lock().map_err(|e| e.to_string())?;
        saved.token.clear();
        saved.phone_token.clear();
        save(&app, &saved)?;
    }
    crate::remote::pointer::stop_all(&app);
    crate::remote::pointer::show_cursor(&app);
    *frame.pairing.lock().map_err(|e| e.to_string())? = None;
    *frame.phone_pairing.lock().map_err(|e| e.to_string())? = None;
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

fn phone_remote_url(url: &str, code: Option<&str>) -> String {
    match code.filter(|code| !code.is_empty()) {
        Some(code) => format!("{url}#code={code}"),
        None => url.to_string(),
    }
}

#[tauri::command]
pub fn phone_remote_qr(app: tauri::AppHandle) -> Result<String, String> {
    let status = frame_status(app)?;
    if !status.enabled || status.addresses.is_empty() {
        return Err("Connect this PC to Wi-Fi and enable remote control first.".into());
    }
    let url = phone_remote_url(&status.phone_url, status.phone_code.as_deref());
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|error| error.to_string())?;
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
        .map_err(|error| format!("Could not open the web remote: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_comparison_rejects_prefixes_and_different_lengths() {
        assert!(same("abcdef", "abcdef"));
        assert!(!same("abcdef", "abcde"));
        assert!(!same("abcdef", "abcdeg"));
        assert!(!same("", "a"));
    }
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
    fn app_catalog_follows_the_synced_order() {
        let names: Vec<&str> = apps_in_order(&["Netflix".into(), "YouTube".into()])
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names[0], "Netflix");
        assert_eq!(names[1], "YouTube");
        assert_eq!(names.len(), apps::catalog().len());
        assert_eq!(names.last().copied(), Some("Custom"));
    }

    #[test]
    fn forgetting_one_device_preserves_the_other_credential() {
        let mut saved = Saved {
            enabled: true,
            token: "display".into(),
            phone_token: "phone".into(),
        };
        clear_device(&mut saved, "display").unwrap();
        assert!(saved.token.is_empty());
        assert_eq!(saved.phone_token, "phone");
        clear_device(&mut saved, "phone").unwrap();
        assert!(saved.phone_token.is_empty());
        assert!(clear_device(&mut saved, "other").is_err());
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
        assert_eq!(phone.phone_token, "phone");
        let first = rotate_phone(&Saved::default()).unwrap();
        assert_eq!(first.phone_token.len(), 64);
        let second = rotate_phone(&first).unwrap();
        assert_eq!(first.phone_token, second.phone_token);
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
            assert!(response.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("manifest-src 'self'"));
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert!(String::from_utf8_lossy(&body).contains("<title>Luma Remote</title>"));
        });
    }

    #[test]
    fn pairing_url_carries_the_active_code_in_the_fragment() {
        assert_eq!(
            phone_remote_url("http://192.168.1.5:9877", Some("123456")),
            "http://192.168.1.5:9877#code=123456"
        );
        assert_eq!(
            phone_remote_url("http://192.168.1.5:9877", None),
            "http://192.168.1.5:9877"
        );
        assert_eq!(
            phone_remote_url("http://192.168.1.5:9877", Some("")),
            "http://192.168.1.5:9877"
        );
    }

    #[test]
    fn web_manifest_describes_an_installable_remote() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["name"], "Luma Remote");
        assert_eq!(manifest["start_url"], "/");
        assert_eq!(manifest["scope"], "/");
        assert_eq!(manifest["display"], "standalone");
        let icons = manifest["icons"].as_array().unwrap();
        assert!(icons.iter().any(|icon| icon["sizes"] == "192x192"));
        assert!(icons.iter().any(|icon| icon["sizes"] == "512x512"));
        assert!(icons.iter().any(|icon| icon["purpose"] == "maskable"));
    }

    #[test]
    fn web_manifest_and_icons_are_served_as_embedded_assets() {
        tauri::async_runtime::block_on(async {
            let response = web_manifest().await;
            assert_eq!(
                response.headers()["content-type"],
                "application/manifest+json"
            );
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert!(String::from_utf8_lossy(&body).contains("Luma Remote"));

            let icon = web_icon(axum::extract::Path("luma-192.png".into()))
                .await
                .unwrap();
            assert_eq!(icon.headers()["content-type"], "image/png");
            let bytes = axum::body::to_bytes(icon.into_body(), usize::MAX)
                .await
                .unwrap();
            assert!(bytes.len() > 1024);
            assert!(
                web_icon(axum::extract::Path("missing.png".into()))
                    .await
                    .is_err()
            );
        });
    }
}
