use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

const API_URL: &str = "https://api.strem.io/api";
const LINK_URL: &str = "https://link.stremio.com";
const WEB_URL: &str = "https://web.stremio.com/";
const CREDENTIAL_TARGET: &str = "Luma/Stremio";
const LINK_TIMEOUT: Duration = Duration::from_secs(300);
const SCRIPT: &str = include_str!("../../resources/stremio/inject.js");
const URI_COMPONENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StremioItem {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub poster: String,
    pub progress: f64,
    pub time_offset: u64,
    pub duration: u64,
    pub last_watched: Option<String>,
    pub video_id: Option<String>,
    pub mtime: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StremioLink {
    code: String,
    link: String,
    qr: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StremioStatus {
    connected: bool,
    email: Option<String>,
    last_sync: Option<u64>,
    linking: Option<StremioLink>,
    items: Vec<StremioItem>,
    continue_watching: Vec<StremioItem>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cache {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    last_sync: Option<u64>,
    #[serde(default)]
    items: Vec<StremioItem>,
}

struct PendingLink {
    code: String,
    link: String,
    qr: String,
    started: Instant,
}

#[derive(Default)]
struct Inner {
    auth_key: Option<String>,
    pending: Option<PendingLink>,
    cache: Cache,
}

pub struct Stremio {
    inner: Mutex<Inner>,
    service: Mutex<Option<std::process::Child>>,
    client: reqwest::Client,
}

impl Default for Stremio {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            service: Mutex::new(None),
            client: reqwest::Client::builder()
                .user_agent(concat!("Luma/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid Stremio HTTP client"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: u64,
    message: String,
}

#[derive(Deserialize)]
struct Envelope<T> {
    result: Option<T>,
    error: Option<ApiError>,
}

enum RemoteError {
    Offline(String),
    Api(ApiError),
    Invalid(String),
}

impl RemoteError {
    fn message(self) -> String {
        match self {
            Self::Offline(message) => message,
            Self::Api(error) => format!("Stremio: {}", error.message),
            Self::Invalid(message) => message,
        }
    }

    fn expired(&self) -> bool {
        matches!(self, Self::Api(error) if error.code == 1)
    }
}

fn transport(error: reqwest::Error) -> RemoteError {
    let message = if error.is_timeout() {
        "Stremio timed out. Check your internet connection and try again."
    } else {
        "Stremio could not be reached. Check your internet connection."
    };
    RemoteError::Offline(message.into())
}

async fn decode<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, RemoteError> {
    if !response.status().is_success() {
        return Err(RemoteError::Invalid(format!(
            "Stremio returned HTTP {}.",
            response.status().as_u16()
        )));
    }
    let envelope: Envelope<T> = response
        .json()
        .await
        .map_err(|_| RemoteError::Invalid("Stremio returned an unexpected response.".into()))?;
    match (envelope.result, envelope.error) {
        (Some(result), _) => Ok(result),
        (None, Some(error)) => Err(RemoteError::Api(error)),
        _ => Err(RemoteError::Invalid(
            "Stremio returned an unexpected response.".into(),
        )),
    }
}

async fn post<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    method: &str,
    body: Value,
) -> Result<T, RemoteError> {
    let response = client
        .post(format!("{API_URL}/{method}"))
        .json(&body)
        .send()
        .await
        .map_err(transport)?;
    decode(response).await
}

#[derive(Deserialize)]
struct LinkCode {
    code: String,
    link: String,
    qrcode: String,
}

async fn create_link(client: &reqwest::Client) -> Result<LinkCode, RemoteError> {
    let response = client
        .get(format!("{LINK_URL}/api/v2/create?type=Create"))
        .send()
        .await
        .map_err(transport)?;
    decode(response).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkAuthKey {
    auth_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiAuthResponse {
    auth_key: String,
}

async fn login_with_token(
    client: &reqwest::Client,
    token: &str,
) -> Result<ApiAuthResponse, RemoteError> {
    post(
        client,
        "loginWithToken",
        json!({ "type": "LoginWithToken", "token": token }),
    )
    .await
}

async fn read_link(client: &reqwest::Client, code: &str) -> Result<Option<String>, RemoteError> {
    let response = client
        .get(format!("{LINK_URL}/api/v2/read?type=Read&code={code}"))
        .send()
        .await
        .map_err(transport)?;
    match decode::<LinkAuthKey>(response).await {
        Ok(auth) => Ok(Some(auth.auth_key)),
        Err(RemoteError::Api(error)) if error.code == 101 => Ok(None),
        Err(error) => Err(error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiUser {
    email: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiLibraryState {
    #[serde(default)]
    time_offset: u64,
    #[serde(default)]
    duration: u64,
    #[serde(default)]
    last_watched: Option<String>,
    #[serde(default, rename = "video_id")]
    video_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiLibraryItem {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    poster: Option<String>,
    #[serde(default)]
    removed: bool,
    #[serde(default)]
    temp: bool,
    #[serde(default, rename = "_mtime")]
    mtime: Option<String>,
    #[serde(default)]
    state: ApiLibraryState,
}

fn map_item(item: ApiLibraryItem) -> StremioItem {
    let progress = if item.state.time_offset > 0 && item.state.duration > 0 {
        ((item.state.time_offset as f64 / item.state.duration as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    };
    StremioItem {
        id: item.id,
        name: item.name,
        kind: item.kind,
        poster: item.poster.unwrap_or_default(),
        progress,
        time_offset: item.state.time_offset,
        duration: item.state.duration,
        last_watched: item.state.last_watched.filter(|value| !value.is_empty()),
        video_id: item.state.video_id.filter(|value| !value.is_empty()),
        mtime: item.mtime.unwrap_or_default(),
    }
}

fn map_library(items: Vec<Value>) -> Vec<StremioItem> {
    items
        .into_iter()
        .filter_map(|value| serde_json::from_value::<ApiLibraryItem>(value).ok())
        .filter(|item| !item.removed || item.temp)
        .map(map_item)
        .collect()
}

fn sort_key(item: &StremioItem) -> &str {
    item.last_watched
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(&item.mtime)
}

fn continue_watching(items: &[StremioItem]) -> Vec<StremioItem> {
    items
        .iter()
        .filter(|item| item.kind != "other" && item.time_offset > 0)
        .cloned()
        .collect()
}

fn build_status(inner: &Inner) -> StremioStatus {
    let linking = inner
        .pending
        .as_ref()
        .filter(|pending| pending.started.elapsed() <= LINK_TIMEOUT)
        .map(|pending| StremioLink {
            code: pending.code.clone(),
            link: pending.link.clone(),
            qr: pending.qr.clone(),
        });
    let mut items = inner.cache.items.clone();
    items.sort_by(|a, b| {
        sort_key(b)
            .cmp(sort_key(a))
            .then_with(|| b.name.cmp(&a.name))
    });
    StremioStatus {
        connected: inner.auth_key.is_some(),
        email: inner.cache.email.clone(),
        last_sync: inner.cache.last_sync,
        linking,
        continue_watching: continue_watching(&items),
        items,
    }
}

fn lock(inner: &Stremio) -> Result<std::sync::MutexGuard<'_, Inner>, String> {
    inner
        .inner
        .lock()
        .map_err(|_| "Stremio integration state is unavailable.".into())
}

fn cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("stremio.json"))
}

fn save_cache(app: &tauri::AppHandle, cache: &Cache) -> Result<(), String> {
    let path = cache_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(
        path,
        serde_json::to_vec(cache).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn remove_cache(app: &tauri::AppHandle) {
    if let Ok(path) = cache_path(app) {
        let _ = std::fs::remove_file(path);
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

pub fn load(app: &tauri::AppHandle) {
    let state = app.state::<Stremio>();
    let mut inner = match state.inner.lock() {
        Ok(inner) => inner,
        Err(_) => return,
    };
    let auth_key = crate::credentials::read_text(CREDENTIAL_TARGET)
        .ok()
        .flatten();
    if auth_key.is_none() {
        inner.cache = Cache::default();
        return;
    }
    inner.cache = cache_path(app)
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    inner.auth_key = auth_key;
}

async fn synchronize(
    app: &tauri::AppHandle,
    state: &Stremio,
    auth_key: &str,
) -> Result<(), String> {
    let user = post::<ApiUser>(&state.client, "getUser", json!({ "authKey": auth_key }));
    let items = post::<Vec<Value>>(
        &state.client,
        "datastoreGet",
        json!({ "authKey": auth_key, "collection": "libraryItem", "ids": [], "all": true }),
    );
    let (user, items) = tokio::try_join!(user, items).map_err(|error| {
        if error.expired() {
            let current = state
                .inner
                .lock()
                .ok()
                .is_some_and(|inner| inner.auth_key.as_deref() == Some(auth_key));
            if current {
                let _ = crate::credentials::delete(CREDENTIAL_TARGET);
                remove_cache(app);
                if let Ok(mut inner) = state.inner.lock() {
                    if inner.auth_key.as_deref() == Some(auth_key) {
                        inner.auth_key = None;
                        inner.cache = Cache::default();
                    }
                }
            }
            "Your Stremio session has expired. Reconnect in Settings → Integrations → Stremio."
                .to_string()
        } else {
            error.message()
        }
    })?;
    let cache = Cache {
        email: user.email,
        last_sync: Some(now_millis()),
        items: map_library(items),
    };
    {
        let mut inner = lock(state)?;
        if inner.auth_key.as_deref() != Some(auth_key) {
            return Err("The Stremio account changed during synchronization.".into());
        }
        inner.cache = cache.clone();
        save_cache(app, &cache)?;
    }
    Ok(())
}

#[tauri::command]
pub fn stremio_status(state: State<Stremio>) -> Result<StremioStatus, String> {
    let inner = lock(&state)?;
    Ok(build_status(&inner))
}

#[tauri::command]
pub async fn stremio_link_start(state: State<'_, Stremio>) -> Result<StremioStatus, String> {
    {
        let inner = lock(&state)?;
        if inner.auth_key.is_some() {
            return Err("Disconnect the current Stremio account before connecting another.".into());
        }
    }
    let code = create_link(&state.client)
        .await
        .map_err(|error| error.message())?;
    {
        let mut inner = lock(&state)?;
        inner.pending = Some(PendingLink {
            code: code.code,
            link: code.link,
            qr: code.qrcode,
            started: Instant::now(),
        });
        Ok(build_status(&inner))
    }
}

#[tauri::command]
pub async fn stremio_link_poll(
    app: tauri::AppHandle,
    state: State<'_, Stremio>,
) -> Result<StremioStatus, String> {
    let code = {
        let mut inner = lock(&state)?;
        match &inner.pending {
            Some(pending) if pending.started.elapsed() <= LINK_TIMEOUT => pending.code.clone(),
            Some(_) => {
                inner.pending = None;
                return Err("The Stremio link code expired. Start again.".into());
            }
            None => return Err("Start connecting a Stremio account first.".into()),
        }
    };
    let link_token = read_link(&state.client, &code)
        .await
        .map_err(|error| error.message())?;
    let Some(link_token) = link_token else {
        let inner = lock(&state)?;
        return Ok(build_status(&inner));
    };
    let session = match login_with_token(&state.client, &link_token).await {
        Ok(session) => session,
        Err(RemoteError::Api(error)) if error.code == 1 => {
            let mut inner = lock(&state)?;
            inner.pending = None;
            return Err(
                "The Stremio link expired before it could be completed. Start again.".into(),
            );
        }
        Err(error) => return Err(error.message()),
    };
    crate::credentials::save_text(CREDENTIAL_TARGET, &session.auth_key)?;
    {
        let mut inner = lock(&state)?;
        inner.auth_key = Some(session.auth_key.clone());
        inner.pending = None;
    }
    if let Some(window) = app.get_webview_window("stremio") {
        window.destroy().map_err(|error| error.to_string())?;
        restore_main(&app);
    }
    synchronize(&app, &state, &session.auth_key).await?;
    let inner = lock(&state)?;
    Ok(build_status(&inner))
}

#[tauri::command]
pub fn stremio_link_cancel(state: State<Stremio>) -> Result<StremioStatus, String> {
    let mut inner = lock(&state)?;
    inner.pending = None;
    Ok(build_status(&inner))
}

#[tauri::command]
pub fn stremio_open_link(state: State<Stremio>) -> Result<(), String> {
    let inner = lock(&state)?;
    let link = inner
        .pending
        .as_ref()
        .map(|pending| pending.link.clone())
        .ok_or("Start connecting a Stremio account first.")?;
    if !link.starts_with("https://link.stremio.com/") {
        return Err("Stremio returned an unexpected link.".into());
    }
    open_url(&link)
}

#[tauri::command]
pub async fn stremio_sync(
    app: tauri::AppHandle,
    state: State<'_, Stremio>,
) -> Result<StremioStatus, String> {
    let auth_key = lock(&state)?
        .auth_key
        .clone()
        .ok_or("Connect a Stremio account first.")?;
    synchronize(&app, &state, &auth_key).await?;
    let inner = lock(&state)?;
    Ok(build_status(&inner))
}

#[tauri::command]
pub async fn stremio_disconnect(
    app: tauri::AppHandle,
    state: State<'_, Stremio>,
) -> Result<StremioStatus, String> {
    if let Some(window) = app.get_webview_window("stremio") {
        window.destroy().map_err(|error| error.to_string())?;
        restore_main(&app);
    }
    let mut inner = lock(&state)?;
    crate::credentials::delete(CREDENTIAL_TARGET)?;
    remove_cache(&app);
    inner.auth_key = None;
    inner.pending = None;
    inner.cache = Cache::default();
    Ok(build_status(&inner))
}

#[tauri::command]
pub async fn stremio_open(
    app: AppHandle,
    state: State<'_, Stremio>,
    id: Option<String>,
    kind: Option<String>,
) -> Result<(), String> {
    let target = match id {
        Some(id) => match kind {
            Some(kind) => Some(web_path(&kind, &id, None)),
            None => {
                let item = lock(&state)?
                    .cache
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .cloned()
                    .ok_or("This title is not in your synchronized Stremio library.")?;
                Some(web_path(&item.kind, &item.id, item.video_id.as_deref()))
            }
        },
        None => None,
    };
    show(app, target.as_deref()).await
}

#[cfg(windows)]
async fn ensure_streaming_service(app: &AppHandle) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let client = app.state::<Stremio>().client.clone();
    if client
        .get("http://127.0.0.1:11470/settings")
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .is_ok_and(|response| response.status().is_success())
    {
        return Ok(());
    }
    let directory = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Programs").join("Stremio"))
        .filter(|path| {
            path.join("stremio-runtime.exe").is_file() && path.join("server.js").is_file()
        })
        .ok_or(
            "Install Stremio Desktop to enable its streaming service, then open Stremio again.",
        )?;
    let state = app.state::<Stremio>();
    let mut service = state
        .service
        .lock()
        .map_err(|_| "The Stremio streaming service state is unavailable.".to_string())?;
    if service
        .as_mut()
        .is_some_and(|child| child.try_wait().ok().flatten().is_none())
    {
        return Ok(());
    }
    let child = std::process::Command::new(directory.join("stremio-runtime.exe"))
        .arg(directory.join("server.js"))
        .current_dir(&directory)
        .creation_flags(0x08000000)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the Stremio streaming service: {error}"))?;
    *service = Some(child);
    Ok(())
}

pub fn shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<Stremio>() else {
        return;
    };
    let Ok(mut service) = state.service.lock() else {
        return;
    };
    if let Some(mut child) = service.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub(crate) async fn show(app: AppHandle, target: Option<&str>) -> Result<(), String> {
    #[cfg(windows)]
    ensure_streaming_service(&app).await?;
    let window = match app.get_webview_window("stremio") {
        Some(window) => {
            let target = serde_json::to_string(&target).map_err(|error| error.to_string())?;
            window
                .eval(format!(
                    "if ({target}) location.hash = {target}; window.__lumaSyncAccount?.({target});"
                ))
                .map_err(|error| error.to_string())?;
            window
        }
        None => {
            let state = app.state::<Stremio>();
            let auth_key = lock(&state)?.auth_key.clone();
            let expected_auth_key = auth_key.clone();
            let session = match auth_key {
                Some(auth_key) => {
                    let user: Value =
                        post(&state.client, "getUser", json!({ "authKey": auth_key }))
                            .await
                            .map_err(RemoteError::message)?;
                    let user_id = user
                        .get("_id")
                        .and_then(Value::as_str)
                        .ok_or("Stremio returned an unexpected account response.")?;
                    Some(json!({ "token": auth_key, "userId": user_id }))
                }
                None => None,
            };
            if lock(&state)?.auth_key != expected_auth_key {
                return Err("The Stremio account changed while opening. Try again.".into());
            }
            create(&app, target, session)?
        }
    };
    let fullscreen = app
        .get_webview_window("main")
        .and_then(|main| main.is_fullscreen().ok());
    crate::background::present(window, fullscreen).await?;
    Ok(())
}

pub(crate) fn close(app: &AppHandle) {
    crate::background::dismiss(app, "stremio");
    restore_main(app);
}

pub(crate) fn forward(app: &AppHandle, command: &str) {
    if command == "home" {
        close(app);
        return;
    }
    let Some(window) = app
        .get_webview_window("stremio")
        .filter(|window| window.is_visible().unwrap_or(false))
    else {
        return;
    };
    let command = serde_json::to_string(command).unwrap_or_default();
    let _ = window.eval(format!(
        "window.__lumaNavigate && window.__lumaNavigate({command});"
    ));
}

fn create(
    app: &AppHandle,
    target: Option<&str>,
    session: Option<Value>,
) -> Result<WebviewWindow, String> {
    let main = app
        .get_webview_window("main")
        .ok_or("Luma window is unavailable")?;
    let size = main.inner_size().map_err(|error| error.to_string())?;
    let position = main.outer_position().map_err(|error| error.to_string())?;
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let fullscreen = main.is_fullscreen().unwrap_or(false);
    let pending = Arc::new(Mutex::new(target.map(str::to_owned)));
    let load_pending = pending.clone();
    let handle = app.clone();
    let new_window = app.clone();
    let url = WEB_URL
        .parse()
        .map_err(|error| format!("Invalid Stremio URL: {error}"))?;
    let window = WebviewWindowBuilder::new(app, "stremio", WebviewUrl::External(url))
        .title("Stremio")
        .additional_browser_args(crate::BROWSER_ARGS)
        .inner_size(size.width as f64 / scale, size.height as f64 / scale)
        .position(position.x as f64 / scale, position.y as f64 / scale)
        .min_inner_size(720.0, 405.0)
        .fullscreen(fullscreen)
        .visible(false)
        .focused(false)
        .initialization_script(
            SCRIPT.replace(
                "__LUMA_STREMIO_SESSION__",
                &json!({
                    "account": session,
                    "target": target,
                })
                .to_string(),
            ),
        )
        .initialization_script(crate::background::MOTION_INIT)
        .on_navigation(move |url| permitted(&handle, url))
        .on_new_window(move |url, _| {
            if web_host(url.host_str()) {
                if let Some(window) = new_window.get_webview_window("stremio") {
                    let _ = window.navigate(url);
                }
            } else {
                external(&url);
            }
            NewWindowResponse::Deny
        })
        .on_page_load(move |window, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            let Some(target) = load_pending
                .lock()
                .ok()
                .and_then(|mut target| target.take())
            else {
                return;
            };
            let Ok(target) = serde_json::to_string(&target) else {
                return;
            };
            let _ = window.eval(format!("location.hash = {target};"));
        })
        .build()
        .map_err(|error| format!("Could not open Stremio: {error}"))?;
    let restore = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let restore = restore.clone();
            tauri::async_runtime::spawn(async move {
                crate::background::dismiss(&restore, "stremio");
                restore_main(&restore);
            });
        }
    });
    Ok(window)
}

fn permitted(app: &AppHandle, url: &tauri::Url) -> bool {
    if url.scheme() == "https"
        && url.host_str() == Some("web.stremio.com")
        && url.path() == "/__luma_close__"
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { if !crate::viewing::return_home(&app) { close(&app); } });
        return false;
    }
    url.scheme() == "about" || (url.scheme() == "https" && web_host(url.host_str()))
}

fn web_host(host: Option<&str>) -> bool {
    host.is_some_and(|host| {
        ["stremio.com", "strem.io"]
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    })
}

fn external(url: &tauri::Url) {
    if matches!(url.scheme(), "http" | "https") {
        let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
    }
}

fn restore_main(app: &AppHandle) {
    crate::background::show(app);
    let _ = app.emit_to("main", "stremio-closed", ());
}

fn web_path(kind: &str, id: &str, video_id: Option<&str>) -> String {
    let raw_id = id;
    let kind = utf8_percent_encode(kind, URI_COMPONENT_ENCODE_SET);
    let id = utf8_percent_encode(raw_id, URI_COMPONENT_ENCODE_SET);
    match video_id.filter(|value| !value.is_empty() && *value != raw_id) {
        Some(video_id) => format!(
            "#/detail/{kind}/{id}/{}",
            utf8_percent_encode(video_id, URI_COMPONENT_ENCODE_SET)
        ),
        None => format!("#/detail/{kind}/{id}"),
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn open_url(target: &str) -> Result<(), String> {
    use windows::{
        core::PCWSTR, Win32::UI::Shell::ShellExecuteW,
        Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
    };
    let operation = wide("open");
    let target = wide(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err(
            "Could not open Stremio. Install it or check its stremio:// protocol registration."
                .into(),
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_url(_: &str) -> Result<(), String> {
    Err("Opening Stremio is available on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, kind: &str, time_offset: u64) -> StremioItem {
        StremioItem {
            id: id.into(),
            name: id.into(),
            kind: kind.into(),
            poster: String::new(),
            progress: 0.0,
            time_offset,
            duration: 1000,
            last_watched: None,
            video_id: None,
            mtime: String::new(),
        }
    }

    #[test]
    fn maps_library_progress_and_episode() {
        let api: ApiLibraryItem = serde_json::from_value(json!({
            "_id": "tt123",
            "name": "Example",
            "type": "series",
            "poster": "https://images.metahub.space/poster/tt123.jpg",
            "_mtime": "2026-01-02T03:04:05.000Z",
            "state": {
                "timeOffset": 2500,
                "duration": 10000,
                "lastWatched": "2026-01-02T03:04:05.000Z",
                "video_id": "tt123:1:2"
            }
        }))
        .expect("valid library item");
        let item = map_item(api);
        assert_eq!(item.progress, 25.0);
        assert_eq!(item.video_id.as_deref(), Some("tt123:1:2"));
    }

    #[test]
    fn login_response_uses_the_session_key() {
        let auth: ApiAuthResponse = serde_json::from_value(json!({
            "authKey": "session-key",
            "user": { "email": "viewer@example.com" }
        }))
        .expect("valid auth response");
        assert_eq!(auth.auth_key, "session-key");
    }

    #[test]
    fn library_mapping_skips_invalid_and_removed_items() {
        let items = vec![
            json!({
                "_id": "tt123",
                "name": "Example",
                "type": "movie",
                "state": { "timeOffset": 0, "duration": 1000 }
            }),
            json!({ "name": "Incomplete" }),
            json!({
                "_id": "tt789",
                "name": "Watched without adding to library",
                "type": "movie",
                "removed": true,
                "temp": true,
                "state": { "timeOffset": 2500, "duration": 10000 }
            }),
            json!({
                "_id": "tt456",
                "name": "Removed",
                "type": "movie",
                "removed": true,
                "state": { "timeOffset": 0, "duration": 1000 }
            }),
        ];
        let mapped = map_library(items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0].id, "tt123");
        assert_eq!(mapped[1].id, "tt789");
        assert_eq!(continue_watching(&mapped)[0].id, "tt789");
    }

    #[test]
    fn continue_watching_matches_upstream_rules() {
        let items = vec![
            item("movie", "movie", 500),
            item("other", "other", 500),
            item("new", "series", 0),
        ];
        let watching = continue_watching(&items);
        assert_eq!(watching.len(), 1);
        assert_eq!(watching[0].id, "movie");
    }

    #[test]
    fn web_links_keep_the_current_episode() {
        let mut series = item("tt123", "series", 1);
        series.video_id = Some("tt123:1:2".into());
        assert_eq!(
            web_path(&series.kind, &series.id, series.video_id.as_deref()),
            "#/detail/series/tt123/tt123%3A1%3A2"
        );
        assert_eq!(
            web_path(&item("tt456", "movie", 1).kind, "tt456", None),
            "#/detail/movie/tt456"
        );
        assert_eq!(
            web_path("movie", "tt456", Some("tt456")),
            "#/detail/movie/tt456"
        );
    }

    #[test]
    fn only_stremio_hosts_can_navigate_the_embedded_window() {
        for host in [
            "web.stremio.com",
            "stremio.com",
            "www.strem.io",
            "api.strem.io",
        ] {
            assert!(web_host(Some(host)), "{host}");
        }
        for host in [
            "example.com",
            "stremio.com.example.com",
            "notstremio.com",
            "strem.io.evil.test",
        ] {
            assert!(!web_host(Some(host)), "{host}");
        }
        assert!(!web_host(None));
    }
}
