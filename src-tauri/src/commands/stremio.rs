use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};
use url::Url;

const API_URL: &str = "https://api.strem.io/api";
const LINK_URL: &str = "https://link.stremio.com";
const CREDENTIAL_TARGET: &str = "Luma/Stremio";
const LINK_TIMEOUT: Duration = Duration::from_secs(600);

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
    client: reqwest::Client,
}

impl Default for Stremio {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            client: reqwest::Client::builder()
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
    items.sort_by(|a, b| sort_key(b).cmp(sort_key(a)).then_with(|| b.name.cmp(&a.name)));
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
    let auth_key = credential_read().ok().flatten();
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
    let user = post::<ApiUser>(
        &state.client,
        "getUser",
        json!({ "authKey": auth_key }),
    );
    let items = post::<Vec<ApiLibraryItem>>(
        &state.client,
        "datastoreGet",
        json!({ "authKey": auth_key, "collection": "libraryItem", "all": true }),
    );
    let (user, items) = tokio::try_join!(user, items).map_err(|error| {
        if error.expired() {
            let _ = credential_delete();
            remove_cache(app);
            if let Ok(mut inner) = state.inner.lock() {
                if inner.auth_key.as_deref() == Some(auth_key) {
                    inner.auth_key = None;
                    inner.cache = Cache::default();
                }
            }
            "Your Stremio session has expired. Disconnect and reconnect in Settings → Integrations → Stremio.".to_string()
        } else {
            error.message()
        }
    })?;
    let cache = Cache {
        email: user.email,
        last_sync: Some(now_millis()),
        items: items
            .into_iter()
            .filter(|item| !item.removed)
            .map(map_item)
            .collect(),
    };
    {
        let mut inner = lock(state)?;
        if inner.auth_key.as_deref() != Some(auth_key) {
            return Err("The Stremio account changed during synchronization.".into());
        }
        inner.cache = cache.clone();
    }
    save_cache(app, &cache)
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
    let auth_key = read_link(&state.client, &code)
        .await
        .map_err(|error| error.message())?;
    let Some(auth_key) = auth_key else {
        let inner = lock(&state)?;
        return Ok(build_status(&inner));
    };
    credential_write(&auth_key)?;
    {
        let mut inner = lock(&state)?;
        inner.auth_key = Some(auth_key.clone());
        inner.pending = None;
    }
    synchronize(&app, &state, &auth_key).await?;
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
pub fn stremio_disconnect(
    app: tauri::AppHandle,
    state: State<Stremio>,
) -> Result<StremioStatus, String> {
    credential_delete()?;
    remove_cache(&app);
    let mut inner = lock(&state)?;
    inner.auth_key = None;
    inner.pending = None;
    inner.cache = Cache::default();
    Ok(build_status(&inner))
}

#[tauri::command]
pub fn stremio_open(
    app: tauri::AppHandle,
    state: State<Stremio>,
    id: String,
) -> Result<(), String> {
    let item = lock(&state)?
        .cache
        .items
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .ok_or("This title is not in your synchronized Stremio library.")?;
    open_url(&deep_link(&item)?)?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.minimize();
    }
    Ok(())
}

fn deep_link(item: &StremioItem) -> Result<String, String> {
    let mut url = Url::parse("stremio:///").map_err(|error| error.to_string())?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "Could not build a Stremio link.".to_string())?;
        path.push("detail").push(&item.kind).push(&item.id);
        if let Some(video_id) = item
            .video_id
            .as_deref()
            .filter(|value| !value.is_empty() && *value != item.id)
        {
            path.push(video_id);
        }
    }
    Ok(url.to_string())
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn credential_read() -> Result<Option<String>, String> {
    use windows::{
        core::PCWSTR,
        Win32::Foundation::ERROR_NOT_FOUND,
        Win32::Security::Credentials::{CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC},
    };
    let target = wide(CREDENTIAL_TARGET);
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    if let Err(error) =
        unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut credential) }
    {
        if error.code() == ERROR_NOT_FOUND.to_hresult() {
            return Ok(None);
        }
        return Err(format!("Windows Credential Manager: {error}"));
    }
    let value = unsafe {
        let credential = &*credential;
        let bytes = std::slice::from_raw_parts(
            credential.CredentialBlob,
            credential.CredentialBlobSize as usize,
        );
        String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
    };
    unsafe { CredFree(credential.cast()) };
    value.map(Some)
}

#[cfg(windows)]
fn credential_write(value: &str) -> Result<(), String> {
    use windows::{
        core::PWSTR,
        Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        },
    };
    let mut target = wide(CREDENTIAL_TARGET);
    let mut user = wide("Stremio");
    let mut secret = value.as_bytes().to_vec();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        CredentialBlobSize: secret.len() as u32,
        CredentialBlob: secret.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(user.as_mut_ptr()),
        ..Default::default()
    };
    unsafe { CredWriteW(&credential, 0) }
        .map_err(|error| format!("Windows Credential Manager: {error}"))
}

#[cfg(windows)]
fn credential_delete() -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::Foundation::ERROR_NOT_FOUND,
        Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC},
    };
    let target = wide(CREDENTIAL_TARGET);
    match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(error) => Err(format!("Windows Credential Manager: {error}")),
    }
}

#[cfg(windows)]
fn open_url(target: &str) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::Shell::ShellExecuteW,
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
        return Err("Could not open Stremio. Install it or check its stremio:// protocol registration.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn credential_read() -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(not(windows))]
fn credential_write(_: &str) -> Result<(), String> {
    Err("Secure Stremio credential storage is available on Windows only.".into())
}

#[cfg(not(windows))]
fn credential_delete() -> Result<(), String> {
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
    fn continue_watching_matches_upstream_rules() {
        let items = vec![item("movie", "movie", 500), item("other", "other", 500), item("new", "series", 0)];
        let watching = continue_watching(&items);
        assert_eq!(watching.len(), 1);
        assert_eq!(watching[0].id, "movie");
    }

    #[test]
    fn deep_links_keep_the_current_episode() {
        let mut series = item("tt123", "series", 1);
        series.video_id = Some("tt123:1:2".into());
        assert_eq!(
            deep_link(&series).unwrap(),
            "stremio:///detail/series/tt123/tt123:1:2"
        );
        assert_eq!(
            deep_link(&item("tt456", "movie", 1)).unwrap(),
            "stremio:///detail/movie/tt456"
        );
    }
}
