use super::{
    document::{self, Config, Document},
    plugin, Commit, Error, Preview,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};
use tauri::Manager;
use tokio::sync::Mutex;

const PORT: u16 = 9879;
const ADDRESS: &str = "127.0.0.1:9879";
const ENDPOINT: &str = "http://127.0.0.1:9879/mcp";
const MAX_BODY: usize = 6 * 1024 * 1024;
const LEGACY_PROTOCOL: &str = "2025-11-25";
const MODERN_PROTOCOL: &str = "2026-07-28";
const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";
const PROTOCOL_HEADER: &str = "mcp-protocol-version";
const METHOD_HEADER: &str = "mcp-method";
const NAME_HEADER: &str = "mcp-name";
const INSTRUCTIONS: &str = "Luma exposes a read, validate, preview plugin-authoring flow. Start with luma_capabilities_get, luma_plugins_list, and luma_config_get. A plugin is a declarative appearance and Home package with manifest-declared settings; executable runtimes are not supported in this Luma version. Author the manifest and files with your coding tools, then call luma_plugin_validate and luma_plugin_stage. Staged plugins are installed disabled. The user must open Luma > Settings > Plugins and press Preview and enable, then press Keep. No MCP tool can enable, keep, disable, remove, export, commit, restore, reset, launch, or execute a plugin. Settings changes for an installed plugin can be staged with luma_plugin_settings_stage for the same user Keep review. Report validation and revision errors exactly, then re-read before retrying.";

const THEMES: [&str; 6] = [
    "violet-night",
    "sapphire",
    "mint",
    "ember",
    "graphite",
    "onyx",
];
const DENSITIES: [&str; 2] = ["comfortable", "compact"];
const TILE_SHAPES: [&str; 2] = ["wide", "square"];
const TEXT_SIZES: [&str; 3] = ["standard", "large", "larger"];
const SOUND_PRESETS: [&str; 4] = ["real", "google", "roku", "off"];
const SOUND_LEVELS: [&str; 3] = ["soft", "standard", "loud"];
const COMPONENTS: [&str; 7] = [
    "hero",
    "continue",
    "apps",
    "popular",
    "featured",
    "recent",
    "recentApps",
];
const APP_IDS: [&str; 16] = [
    "YouTube",
    "Spotify",
    "Stremio",
    "Steam",
    "NBA",
    "NFL",
    "MLB",
    "NHL",
    "Netflix",
    "Disney+",
    "Prime Video",
    "Apple TV+",
    "Hulu",
    "Max",
    "Peacock",
    "Custom",
];
const PLACEMENTS: [&str; 4] = ["bar", "top-right", "bottom-right", "bottom-left"];

type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;

trait Backend: Send + Sync + 'static {
    fn read<'a>(&'a self) -> BackendFuture<'a, Option<Document>>;
    fn preview<'a>(&'a self, request: Commit) -> BackendFuture<'a, Preview>;
    fn cancel_preview<'a>(&'a self, id: &'a str) -> BackendFuture<'a, ()>;
    fn current_preview<'a>(&'a self) -> BackendFuture<'a, Option<Preview>>;
    fn launcher_visible<'a>(&'a self) -> BackendFuture<'a, bool>;
    fn plugins<'a>(&'a self) -> BackendFuture<'a, plugin::PluginsState>;
    fn validate_plugin<'a>(&'a self, submission: Value) -> BackendFuture<'a, Value>;
    fn stage_plugin<'a>(&'a self, submission: Value) -> BackendFuture<'a, plugin::ImportResult>;
    fn stage_plugin_settings<'a>(
        &'a self,
        plugin_id: String,
        expected_revision: u64,
        values: BTreeMap<String, Value>,
    ) -> BackendFuture<'a, Preview>;
}

#[derive(Clone)]
struct AppBackend(tauri::AppHandle);

impl Backend for AppBackend {
    fn read<'a>(&'a self) -> BackendFuture<'a, Option<Document>> {
        let app = self.0.clone();
        Box::pin(async move {
            tauri::async_runtime::spawn_blocking(move || app.state::<super::Store>().read_effective())
                .await
                .map_err(|error| failure("storage", error))?
        })
    }

    fn preview<'a>(&'a self, request: Commit) -> BackendFuture<'a, Preview> {
        let app = self.0.clone();
        Box::pin(async move { super::customization_preview(app, request).await })
    }

    fn cancel_preview<'a>(&'a self, id: &'a str) -> BackendFuture<'a, ()> {
        let app = self.0.clone();
        let id = id.to_string();
        Box::pin(async move {
            super::customization_preview_cancel(app, super::PreviewAction { preview_id: id }).await
        })
    }

    fn current_preview<'a>(&'a self) -> BackendFuture<'a, Option<Preview>> {
        let app = self.0.clone();
        Box::pin(async move { super::customization_preview_read(app).await })
    }

    fn launcher_visible<'a>(&'a self) -> BackendFuture<'a, bool> {
        let visible = self.0.get_webview_window("main").is_some_and(|window| {
            window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
        });
        Box::pin(async move { Ok(visible) })
    }

    fn plugins<'a>(&'a self) -> BackendFuture<'a, plugin::PluginsState> {
        let app = self.0.clone();
        Box::pin(async move {
            tauri::async_runtime::spawn_blocking(move || plugin::state_for_app(&app))
                .await
                .map_err(|error| failure("storage", error))?
        })
    }

    fn validate_plugin<'a>(&'a self, submission: Value) -> BackendFuture<'a, Value> {
        Box::pin(async move {
            let package = plugin::validate_submission(&submission)?;
            Ok(json!({
                "valid": true,
                "manifest": package.manifest,
                "contentHash": package.hash,
                "files": package.files.len(),
                "bytes": package.size
            }))
        })
    }

    fn stage_plugin<'a>(&'a self, submission: Value) -> BackendFuture<'a, plugin::ImportResult> {
        let app = self.0.clone();
        Box::pin(async move {
            tauri::async_runtime::spawn_blocking(move || {
                let store = app.state::<super::Store>();
                let outcome = plugin::stage_submission(&store, &submission, "mcp")?;
                if let Ok(Some(snapshot)) = store.snapshot() {
                    super::publish(&app, &snapshot.document);
                }
                Ok(plugin::ImportResult {
                    plugin: outcome.plugin,
                    already_installed: outcome.already_installed,
                    replaced_version: outcome.replaced_version,
                })
            })
            .await
            .map_err(|error| failure("storage", error))?
        })
    }

    fn stage_plugin_settings<'a>(
        &'a self,
        plugin_id: String,
        expected_revision: u64,
        values: BTreeMap<String, Value>,
    ) -> BackendFuture<'a, Preview> {
        let app = self.0.clone();
        Box::pin(async move {
            tauri::async_runtime::spawn_blocking(move || {
                let store = app.state::<super::Store>();
                let preview = store.preview_plugin_settings(&plugin_id, expected_revision, &values)?;
                super::publish_preview(&app);
                Ok(preview)
            })
            .await
            .map_err(|error| failure("storage", error))?
        })
    }
}

struct Access {
    enabled: bool,
    available: bool,
    token: Option<String>,
    generation: u64,
    owned: Option<OwnedPreview>,
    start_error: Option<String>,
}

#[derive(Clone)]
struct OwnedPreview {
    id: String,
    base_revision: u64,
}

#[derive(Clone)]
pub struct McpAccess {
    state: Arc<Mutex<Access>>,
}

impl McpAccess {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(Access {
                enabled: false,
                available: false,
                token: None,
                generation: 0,
                owned: None,
                start_error: None,
            })),
        }
    }
}

impl Default for McpAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl McpAccess {
    async fn status(&self) -> Status {
        let state = self.state.lock().await;
        Status {
            enabled: state.enabled && state.token.is_some() && state.available,
            endpoint: ENDPOINT,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub endpoint: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub enabled: bool,
    pub endpoint: &'static str,
    pub token: String,
}

#[derive(Clone)]
struct McpServer {
    access: McpAccess,
    backend: Arc<dyn Backend>,
    host: String,
}

impl McpServer {
    fn app(app: &tauri::AppHandle, access: McpAccess) -> Self {
        Self {
            access,
            backend: Arc::new(AppBackend(app.clone())),
            host: ADDRESS.into(),
        }
    }

    async fn status(&self) -> Status {
        self.access.status().await
    }

    async fn enable(&self) -> Result<Credentials, Error> {
        let mut state = self.access.state.lock().await;
        if !state.available {
            let message = state.start_error.clone().unwrap_or_else(|| {
                format!("Agent access is not listening on {ADDRESS}. Restart Luma and try again.")
            });
            return Err(failure("mcp_unavailable", message));
        }
        if !state.enabled || state.token.is_none() {
            state.generation = state.generation.saturating_add(1);
            state.token = Some(random_token()?);
            state.enabled = true;
        }
        Ok(Credentials {
            enabled: true,
            endpoint: ENDPOINT,
            token: state.token.clone().unwrap_or_default(),
        })
    }

    async fn disable(&self) -> Result<Status, Error> {
        let owned = {
            let mut state = self.access.state.lock().await;
            state.enabled = false;
            state.token = None;
            state.generation = state.generation.saturating_add(1);
            state.owned.take()
        };
        if let Some(preview) = owned {
            let _ = self.backend.cancel_preview(&preview.id).await;
        }
        Ok(Status {
            enabled: false,
            endpoint: ENDPOINT,
        })
    }

    async fn rotate(&self) -> Result<Credentials, Error> {
        let (token, owned) = {
            let mut state = self.access.state.lock().await;
            if !state.available {
                let message = state.start_error.clone().unwrap_or_else(|| {
                    format!(
                        "Agent access is not listening on {ADDRESS}. Restart Luma and try again."
                    )
                });
                return Err(failure("mcp_unavailable", message));
            }
            if !state.enabled || state.token.is_none() {
                return Err(failure(
                    "mcp_disabled",
                    "Enable agent access before rotating its credential.",
                ));
            }
            let token = random_token()?;
            state.generation = state.generation.saturating_add(1);
            state.token = Some(token.clone());
            (token, state.owned.take())
        };
        if let Some(preview) = owned {
            let _ = self.backend.cancel_preview(&preview.id).await;
        }
        Ok(Credentials {
            enabled: true,
            endpoint: ENDPOINT,
            token,
        })
    }

    async fn authorize(&self, token: &str) -> Result<u64, Error> {
        let state = self.access.state.lock().await;
        let valid = state.enabled
            && state.available
            && state
                .token
                .as_deref()
                .is_some_and(|expected| same(token, expected));
        if !valid {
            return Err(failure(
                "unauthorized",
                "Agent access is disabled or the credential is no longer valid.",
            ));
        }
        Ok(state.generation)
    }

    async fn read(&self) -> Result<Option<Document>, Error> {
        self.backend.read().await
    }

    async fn validate(&self, expected_revision: u64, candidate: Value) -> Result<Value, Error> {
        let current = self.read().await?.ok_or_else(|| {
            failure(
                "not_initialized",
                "Customization is not initialized. Open Luma once.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(failure(
                "conflict",
                format!(
                    "Expected revision {expected_revision}, but the current revision is {}. Read the current configuration again.",
                    current.revision
                ),
            ));
        }
        let config: Config = serde_json::from_value(candidate).map_err(|error| {
            failure(
                "validation",
                format!("Invalid candidate configuration: {error}"),
            )
        })?;
        if let Err(message) = document::validate(&config) {
            return Ok(json!({
                "valid": false,
                "revision": current.revision,
                "errors": [{
                    "message": message,
                    "action": action("validation")
                }]
            }));
        }
        Ok(json!({
            "valid": true,
            "revision": current.revision,
            "expectedRevision": expected_revision
        }))
    }

    async fn stage_preview(&self, generation: u64, request: Commit) -> Result<Preview, Error> {
        let mut state = self.access.state.lock().await;
        if !state.enabled || !state.available {
            return Err(failure(
                "grant_revoked",
                "Agent access was disabled before this preview could start.",
            ));
        }
        if state.generation != generation {
            return Err(failure(
                "grant_revoked",
                "Agent access was rotated before this preview could start.",
            ));
        }
        if !self.backend.launcher_visible().await? {
            return Err(failure(
                "launcher_unavailable",
                "Open Luma on the desktop and keep the launcher visible before previewing.",
            ));
        }
        let preview = self.backend.preview(request).await?;
        state.owned = Some(OwnedPreview {
            id: preview.id.clone(),
            base_revision: preview.base_revision,
        });
        Ok(preview)
    }

    async fn cancel_own(&self, generation: u64, id: &str) -> Result<(), Error> {
        let mut state = self.access.state.lock().await;
        if state.generation != generation {
            return Err(failure(
                "grant_revoked",
                "Agent access was rotated or disabled. This preview is no longer owned by this grant.",
            ));
        }
        let owned = state.owned.as_ref().ok_or_else(|| {
            failure(
                "not_owner",
                "This grant has no pending preview. This tool cancels only a preview created by this grant.",
            )
        })?;
        if owned.id != id {
            return Err(failure(
                "not_owner",
                "This preview was not created by this grant. Ask the user to reject it in Luma.",
            ));
        }
        let result = self.backend.cancel_preview(id).await;
        state.owned = None;
        result
    }

    async fn pending(&self) -> Result<Option<OwnedPreview>, Error> {
        let owned = {
            let state = self.access.state.lock().await;
            state.owned.clone()
        };
        let Some(owned) = owned else {
            return Ok(None);
        };
        let current = self.backend.current_preview().await?;
        if current
            .as_ref()
            .is_some_and(|preview| preview.id == owned.id)
        {
            return Ok(Some(owned));
        }
        let mut state = self.access.state.lock().await;
        if state
            .owned
            .as_ref()
            .is_some_and(|preview| preview.id == owned.id)
        {
            state.owned = None;
        }
        Ok(None)
    }
}

pub async fn start(app: tauri::AppHandle) -> Result<(), String> {
    let access = app
        .try_state::<McpAccess>()
        .map(|state| state.inner().clone())
        .ok_or_else(|| "McpAccess must be managed before starting the MCP endpoint.".to_string())?;
    start_with(app, access).await
}

async fn start_with(app: tauri::AppHandle, access: McpAccess) -> Result<(), String> {
    {
        let state = access.state.lock().await;
        if state.available {
            return Ok(());
        }
    }
    let listener = match tokio::net::TcpListener::bind(ADDRESS).await {
        Ok(listener) => listener,
        Err(error) => {
            let message = format!("Cannot listen on {ADDRESS}: {error}");
            let mut state = access.state.lock().await;
            state.available = false;
            state.enabled = false;
            state.token = None;
            state.start_error = Some(format!(
                "Agent access cannot listen on {ADDRESS}. Stop the application using that port or restart Luma. {error}"
            ));
            return Err(message);
        }
    };
    {
        let mut state = access.state.lock().await;
        if state.available {
            return Ok(());
        }
        state.available = true;
        state.start_error = None;
    }
    let server = McpServer::app(&app, access.clone());
    let result = axum::serve(listener, router(server)).await;
    let owned = {
        let mut state = access.state.lock().await;
        state.available = false;
        state.enabled = false;
        state.token = None;
        state.generation = state.generation.saturating_add(1);
        state.start_error = Some(format!("Agent access stopped listening on {ADDRESS}."));
        state.owned.take()
    };
    if let Some(preview) = owned {
        let _ = AppBackend(app).cancel_preview(&preview.id).await;
    }
    result.map_err(|error| error.to_string())
}

fn router(server: McpServer) -> Router {
    Router::new()
        .route("/mcp", post(endpoint))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn(harden))
        .with_state(server)
}

async fn harden(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    response
}

async fn endpoint(State(server): State<McpServer>, headers: HeaderMap, body: Bytes) -> Response {
    if let Err(response) = guard_transport(&server, &headers) {
        return response;
    }
    let Some(token) = bearer(&headers) else {
        return unauthorized();
    };
    let generation = match server.authorize(token).await {
        Ok(generation) => generation,
        Err(_) => return unauthorized(),
    };
    let request = match parse_request(&body) {
        Ok(request) => request,
        Err(error) => return error_response(None, error.status, error.code, error.message, None),
    };
    let modern = request
        .params
        .get("_meta")
        .and_then(Value::as_object)
        .is_some_and(|meta| meta.contains_key(META_VERSION))
        || request.method == "server/discover";
    let protocol = if modern {
        match modern_request(&headers, &request) {
            Ok(protocol) => protocol,
            Err(error) => {
                return error_response(
                    request.id.as_ref(),
                    error.status,
                    error.code,
                    error.message,
                    error.data,
                )
            }
        }
    } else {
        match legacy_request(&headers, &request) {
            Ok(protocol) => protocol,
            Err(error) => {
                return error_response(
                    request.id.as_ref(),
                    error.status,
                    error.code,
                    error.message,
                    error.data,
                )
            }
        }
    };
    if request.id.is_none() {
        if request.method.starts_with("notifications/") {
            return accepted();
        }
        return error_response(
            None,
            StatusCode::BAD_REQUEST,
            -32600,
            "Requests must include an id.".into(),
            None,
        );
    }
    dispatch(&server, generation, protocol, request).await
}

fn guard_transport(server: &McpServer, headers: &HeaderMap) -> Result<(), Response> {
    if headers.contains_key(header::ORIGIN) {
        return Err(error_response(
            None,
            StatusCode::FORBIDDEN,
            -32600,
            "This endpoint does not accept browser requests.".into(),
            None,
        ));
    }
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase());
    let expected = server.host.to_ascii_lowercase();
    let loopback = format!("localhost:{PORT}");
    if !host
        .as_deref()
        .is_some_and(|host| host == expected || host == loopback)
    {
        return Err(error_response(
            None,
            StatusCode::FORBIDDEN,
            -32600,
            format!("Requests must be sent to {ADDRESS}."),
            None,
        ));
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    {
        return Err(error_response(
            None,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            -32600,
            "Content-Type must be application/json.".into(),
            None,
        ));
    }
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let json_accept = accept.is_empty()
        || accept.split(',').any(|value| {
            let value = value.trim().to_ascii_lowercase();
            value == "*/*" || value.starts_with("application/json")
        });
    if !json_accept {
        return Err(error_response(
            None,
            StatusCode::NOT_ACCEPTABLE,
            -32600,
            "This endpoint returns application/json; accept application/json.".into(),
            None,
        ));
    }
    Ok(())
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
}

fn unauthorized() -> Response {
    let mut response = error_response(
        None,
        StatusCode::UNAUTHORIZED,
        -32600,
        "Missing or invalid agent credential.".into(),
        None,
    );
    response
        .headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

struct RpcRequest {
    id: Option<Value>,
    method: String,
    params: Value,
}

struct RpcFailure {
    status: StatusCode,
    code: i64,
    message: String,
}

struct ProtocolError {
    status: StatusCode,
    code: i64,
    message: String,
    data: Option<Value>,
}

#[derive(Clone, Copy)]
enum Protocol {
    Legacy,
    Modern,
}

fn parse_request(body: &[u8]) -> Result<RpcRequest, RpcFailure> {
    let value: Value = serde_json::from_slice(body).map_err(|error| RpcFailure {
        status: StatusCode::BAD_REQUEST,
        code: -32700,
        message: format!("Parse error: {error}"),
    })?;
    let object = value.as_object().ok_or_else(|| RpcFailure {
        status: StatusCode::BAD_REQUEST,
        code: -32600,
        message: "JSON-RPC messages must be objects.".into(),
    })?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(RpcFailure {
            status: StatusCode::BAD_REQUEST,
            code: -32600,
            message: "jsonrpc must be \"2.0\".".into(),
        });
    }
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcFailure {
            status: StatusCode::BAD_REQUEST,
            code: -32600,
            message: "method must be a string.".into(),
        })?
        .to_string();
    let id = match object.get("id") {
        None => None,
        Some(Value::String(id)) => Some(Value::String(id.clone())),
        Some(Value::Number(id)) => Some(Value::Number(id.clone())),
        Some(Value::Null) => {
            return Err(RpcFailure {
                status: StatusCode::BAD_REQUEST,
                code: -32600,
                message: "Request id must not be null.".into(),
            })
        }
        Some(_) => {
            return Err(RpcFailure {
                status: StatusCode::BAD_REQUEST,
                code: -32600,
                message: "Request id must be a string or number.".into(),
            })
        }
    };
    let params = match object.get("params") {
        None => json!({}),
        Some(Value::Object(_)) => object["params"].clone(),
        Some(_) => {
            return Err(RpcFailure {
                status: StatusCode::BAD_REQUEST,
                code: -32602,
                message: "params must be an object.".into(),
            })
        }
    };
    Ok(RpcRequest { id, method, params })
}

fn legacy_request(headers: &HeaderMap, request: &RpcRequest) -> Result<Protocol, ProtocolError> {
    if request.method != "initialize" {
        if let Some(version) = header(headers, PROTOCOL_HEADER) {
            if version != LEGACY_PROTOCOL {
                return Err(ProtocolError {
                    status: StatusCode::BAD_REQUEST,
                    code: -32602,
                    message: format!(
                        "This server supports MCP {LEGACY_PROTOCOL} with initialize, or {MODERN_PROTOCOL} with per-request _meta."
                    ),
                    data: Some(json!({
                        "supported": [MODERN_PROTOCOL, LEGACY_PROTOCOL],
                        "requested": version
                    })),
                });
            }
        }
    }
    Ok(Protocol::Legacy)
}

fn modern_request(headers: &HeaderMap, request: &RpcRequest) -> Result<Protocol, ProtocolError> {
    let meta = request
        .params
        .get("_meta")
        .and_then(Value::as_object)
        .ok_or_else(|| ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32602,
            message: format!("Modern MCP requests require params._meta.{META_VERSION}."),
            data: None,
        })?;
    let version = meta
        .get(META_VERSION)
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32602,
            message: format!("params._meta.{META_VERSION} must be a string."),
            data: None,
        })?;
    let header_version = header(headers, PROTOCOL_HEADER).ok_or_else(|| ProtocolError {
        status: StatusCode::BAD_REQUEST,
        code: -32020,
        message: format!("{PROTOCOL_HEADER} header is required."),
        data: None,
    })?;
    if header_version != version {
        return Err(ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32020,
            message: format!(
                "Header mismatch: {PROTOCOL_HEADER} is {header_version}, body version is {version}."
            ),
            data: None,
        });
    }
    if version != MODERN_PROTOCOL {
        return Err(ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32022,
            message: "Unsupported protocol version.".into(),
            data: Some(json!({
                "supported": [MODERN_PROTOCOL, LEGACY_PROTOCOL],
                "requested": version
            })),
        });
    }
    if !meta
        .get(META_CLIENT_CAPABILITIES)
        .is_some_and(Value::is_object)
    {
        return Err(ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32602,
            message: format!("params._meta.{META_CLIENT_CAPABILITIES} must be an object."),
            data: None,
        });
    }
    let method_header = header(headers, METHOD_HEADER).ok_or_else(|| ProtocolError {
        status: StatusCode::BAD_REQUEST,
        code: -32020,
        message: format!("{METHOD_HEADER} header is required."),
        data: None,
    })?;
    if method_header != request.method {
        return Err(ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32020,
            message: format!(
                "Header mismatch: {METHOD_HEADER} is {method_header}, body method is {}.",
                request.method
            ),
            data: None,
        });
    }
    let expected_name = match request.method.as_str() {
        "tools/call" => request.params.get("name").and_then(Value::as_str),
        "resources/read" => request.params.get("uri").and_then(Value::as_str),
        _ => None,
    };
    if matches!(request.method.as_str(), "tools/call" | "resources/read") {
        let name = header(headers, NAME_HEADER).ok_or_else(|| ProtocolError {
            status: StatusCode::BAD_REQUEST,
            code: -32020,
            message: format!("{NAME_HEADER} header is required for {}.", request.method),
            data: None,
        })?;
        if Some(name) != expected_name {
            return Err(ProtocolError {
                status: StatusCode::BAD_REQUEST,
                code: -32020,
                message: format!(
                    "Header mismatch: {NAME_HEADER} does not match params for {}.",
                    request.method
                ),
                data: None,
            });
        }
    }
    Ok(Protocol::Modern)
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn dispatch(
    server: &McpServer,
    generation: u64,
    protocol: Protocol,
    request: RpcRequest,
) -> Response {
    let modern = matches!(protocol, Protocol::Modern);
    let id = request.id.as_ref().expect("checked above").clone();
    match request.method.as_str() {
        "initialize" if !modern => {
            let requested = request
                .params
                .get("protocolVersion")
                .and_then(Value::as_str);
            if requested.is_none() {
                return error_response(
                    Some(&id),
                    StatusCode::OK,
                    -32602,
                    "initialize requires params.protocolVersion.".into(),
                    None,
                );
            }
            success(
                id,
                json!({
                    "protocolVersion": LEGACY_PROTOCOL,
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    },
                    "serverInfo": server_info(),
                    "instructions": INSTRUCTIONS
                }),
                false,
            )
        }
        "notifications/initialized" | "notifications/cancelled" => accepted(),
        "ping" => success(id, json!({}), modern),
        "server/discover" if modern => success(
            id,
            json!({
                "supportedVersions": [MODERN_PROTOCOL, LEGACY_PROTOCOL],
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "instructions": INSTRUCTIONS,
                "ttlMs": 60_000,
                "cacheScope": "private"
            }),
            true,
        ),
        "tools/list" => {
            let mut result = tool_list();
            if modern {
                result["ttlMs"] = json!(60_000);
                result["cacheScope"] = json!("private");
            }
            success(id, result, modern)
        }
        "tools/call" => {
            let name = request.params.get("name").and_then(Value::as_str);
            let Some(name) = name else {
                return error_response(
                    Some(&id),
                    StatusCode::OK,
                    -32602,
                    "tools/call requires params.name.".into(),
                    None,
                );
            };
            let arguments = match request.params.get("arguments") {
                None => json!({}),
                Some(Value::Object(_)) => request.params["arguments"].clone(),
                Some(_) => {
                    return error_response(
                        Some(&id),
                        StatusCode::OK,
                        -32602,
                        "tools/call arguments must be an object.".into(),
                        None,
                    )
                }
            };
            if !known_tool(name) {
                return error_response(
                    Some(&id),
                    StatusCode::OK,
                    -32602,
                    unknown_tool_message(name),
                    Some(json!({ "action": "Ask the user to perform this action in Luma." })),
                );
            }
            let result = call_tool(server, generation, name, arguments).await;
            success(id, result, modern)
        }
        "resources/list" => {
            let mut result = resource_list();
            if modern {
                result["ttlMs"] = json!(60_000);
                result["cacheScope"] = json!("private");
            }
            success(id, result, modern)
        }
        "resources/templates/list" => {
            let mut result = json!({ "resourceTemplates": [] });
            if modern {
                result["ttlMs"] = json!(60_000);
                result["cacheScope"] = json!("private");
            }
            success(id, result, modern)
        }
        "resources/read" => {
            let Some(uri) = request.params.get("uri").and_then(Value::as_str) else {
                return error_response(
                    Some(&id),
                    StatusCode::OK,
                    -32602,
                    "resources/read requires params.uri.".into(),
                    None,
                );
            };
            match read_resource(uri) {
                Ok((mime, text)) => {
                    let mut result = json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": mime,
                            "text": text
                        }]
                    });
                    if modern {
                        result["ttlMs"] = json!(60_000);
                        result["cacheScope"] = json!("private");
                    }
                    success(id, result, modern)
                }
                Err(message) => error_response(
                    Some(&id),
                    StatusCode::OK,
                    if modern { -32602 } else { -32002 },
                    message,
                    Some(json!({ "uri": uri })),
                ),
            }
        }
        _ => error_response(
            Some(&id),
            if modern {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            },
            -32601,
            format!("Method not found: {}.", request.method),
            None,
        ),
    }
}

fn known_tool(name: &str) -> bool {
    matches!(
        name,
        "luma_capabilities_get"
            | "luma_config_get"
            | "luma_candidate_validate"
            | "luma_candidate_preview"
            | "luma_candidate_get"
            | "luma_candidate_cancel"
            | "luma_status"
            | "luma_plugins_list"
            | "luma_plugin_validate"
            | "luma_plugin_stage"
            | "luma_plugin_settings_stage"
    )
}

#[cfg(test)]
pub(crate) fn known_tool_for_test(name: &str) -> bool {
    known_tool(name)
}

fn unknown_tool_message(name: &str) -> String {
    format!(
        "Unknown tool: {name}. Luma exposes only discover, read, validate, preview, plugin validation, disabled plugin staging, plugin settings staging, candidate status, candidate cancellation, and service status. It never exposes enable, keep, commit, disable, remove, export, restore, reset, launch, install-active, or execute. Ask the user to perform those actions in Luma."
    )
}

async fn call_tool(server: &McpServer, generation: u64, name: &str, arguments: Value) -> Value {
    match name {
        "luma_capabilities_get" => match no_arguments(arguments) {
            Ok(()) => tool_success(capabilities()),
            Err(message) => tool_protocol_error(message),
        },
        "luma_config_get" => match no_arguments(arguments) {
            Ok(()) => match server.read().await {
                Ok(Some(document)) => tool_success(json!({
                    "initialized": true,
                    "revision": document.revision,
                    "document": document
                })),
                Ok(None) => tool_success(json!({
                    "initialized": false,
                    "revision": null,
                    "document": null,
                    "action": "Open Luma once so customization can be initialized."
                })),
                Err(error) => tool_failure(error),
            },
            Err(message) => tool_protocol_error(message),
        },
        "luma_candidate_validate" => match candidate_arguments(arguments) {
            Ok((expected_revision, config)) => {
                match server.validate(expected_revision, config).await {
                    Ok(result) => tool_success(result),
                    Err(error) => tool_failure(error),
                }
            }
            Err(message) => tool_protocol_error(message),
        },
        "luma_candidate_preview" => match candidate_arguments(arguments) {
            Ok((expected_revision, candidate)) => {
                let config: Config = match serde_json::from_value(candidate) {
                    Ok(config) => config,
                    Err(error) => {
                        return tool_success(json!({
                            "valid": false,
                            "errors": [{
                                "message": format!("Invalid candidate configuration: {error}"),
                                "action": action("validation")
                            }]
                        }))
                    }
                };
                match server
                    .stage_preview(
                        generation,
                        Commit {
                            expected_revision,
                            config,
                        },
                    )
                    .await
                {
                    Ok(preview) => tool_success(json!({
                        "previewId": preview.id,
                        "baseRevision": preview.base_revision,
                        "detached": preview.detached,
                        "status": "waiting_for_user_review",
                        "reviewRequiresVisibleLauncher": true,
                        "next": "Tell the user that the change is waiting for review in Luma. The user must press Keep in Luma; no MCP tool can keep, commit, restore, reset, launch, or install changes. Keeping this candidate detaches any listed fields from their controlling plugin."
                    })),
                    Err(error) => tool_failure(error),
                }
            }
            Err(message) => tool_protocol_error(message),
        },
        "luma_candidate_get" => match no_arguments(arguments) {
            Ok(()) => match server.pending().await {
                Ok(Some(preview)) => tool_success(json!({
                    "pending": {
                        "previewId": preview.id,
                        "baseRevision": preview.base_revision,
                        "ownedByThisGrant": true
                    },
                    "next": "Ask the user to review and press Keep or Reject in Luma."
                })),
                Ok(None) => tool_success(json!({ "pending": null })),
                Err(error) => tool_failure(error),
            },
            Err(message) => tool_protocol_error(message),
        },
        "luma_candidate_cancel" => {
            let arguments: CancelArguments = match serde_json::from_value(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_protocol_error(format!("Invalid arguments: {error}")),
            };
            match server.cancel_own(generation, &arguments.preview_id).await {
                Ok(()) => tool_success(json!({ "cancelled": true })),
                Err(error) => tool_failure(error),
            }
        }
        "luma_status" => match no_arguments(arguments) {
            Ok(()) => match status_value(server).await {
                Ok(status) => tool_success(status),
                Err(error) => tool_failure(error),
            },
            Err(message) => tool_protocol_error(message),
        },
        "luma_plugins_list" => match no_arguments(arguments) {
            Ok(()) => match server.backend.plugins().await {
                Ok(state) => tool_success(json!({
                    "plugins": state.plugins,
                    "effective": state.effective,
                    "enable": "Installed plugins start disabled. The user must open Luma > Settings > Plugins, preview the plugin, and press Keep to enable it.",
                    "settingsReview": "Use luma_plugin_settings_stage to stage setting values. The user must press Keep."
                })),
                Err(error) => tool_failure(error),
            },
            Err(message) => tool_protocol_error(message),
        },
        "luma_plugin_validate" => match plugin_submission_arguments(arguments) {
            Ok(submission) => match server.backend.validate_plugin(submission).await {
                Ok(result) => tool_success(result),
                Err(error) => tool_failure(error),
            },
            Err(message) => tool_protocol_error(message),
        },
        "luma_plugin_stage" => match plugin_submission_arguments(arguments) {
            Ok(submission) => match server.backend.stage_plugin(submission).await {
                Ok(result) => tool_success(json!({
                    "plugin": result.plugin,
                    "alreadyInstalled": result.already_installed,
                    "replacedVersion": result.replaced_version,
                    "enabled": result.plugin.enabled,
                    "status": if result.already_installed { "already_installed" } else { "installed_disabled" },
                    "next": if result.already_installed && result.plugin.enabled {
                        "The same package is already installed and enabled. No change was made."
                    } else {
                        "Tell the user the plugin is installed but disabled. They must open Luma > Settings > Plugins, preview it, and press Keep to enable it. No MCP tool can enable or keep it."
                    }
                })),
                Err(error) => tool_failure(error),
            },
            Err(message) => tool_protocol_error(message),
        },
        "luma_plugin_settings_stage" => {
            let arguments: PluginSettingsArguments = match serde_json::from_value(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_protocol_error(format!("Invalid arguments: {error}")),
            };
            let state = match server.backend.plugins().await {
                Ok(state) => state,
                Err(error) => return tool_failure(error),
            };
            let Some(record) = state.plugins.iter().find(|plugin| plugin.id == arguments.plugin_id) else {
                return tool_failure(failure(
                    "plugin_missing",
                    "That plugin is not installed. Stage the plugin first.",
                ));
            };
            let declared: Vec<&str> = record.settings.iter().map(|setting| setting.key.as_str()).collect();
            for key in arguments.settings.keys() {
                if !declared.contains(&key.as_str()) && !record.settings.iter().any(|setting| &setting.qualified_key == key) {
                    return tool_failure(failure(
                        "plugin_settings",
                        format!("{} does not declare a setting named {key}.", record.name),
                    ));
                }
            }
            let mut values = BTreeMap::new();
            for (key, value) in arguments.settings {
                let key = key.rsplit_once('.').map(|(_, key)| key.to_string()).unwrap_or(key);
                values.insert(key, value);
            }
            let visible = match server.backend.launcher_visible().await {
                Ok(visible) => visible,
                Err(error) => return tool_failure(error),
            };
            if !visible {
                return tool_failure(failure(
                    "launcher_unavailable",
                    "Open Luma on the desktop and keep the launcher visible so the user can review these plugin settings.",
                ));
            }
            match server
                .backend
                .stage_plugin_settings(record.id.clone(), arguments.expected_revision, values)
                .await
            {
                Ok(preview) => tool_success(json!({
                    "previewId": preview.id,
                    "baseRevision": preview.base_revision,
                    "pluginId": record.id,
                    "status": "waiting_for_user_review",
                    "next": "Tell the user that the plugin settings are waiting for review in Luma. They must press Keep."
                })),
                Err(error) => tool_failure(error),
            }
        }
        forbidden => tool_protocol_error(unknown_tool_message(forbidden)),
    }
}

async fn status_value(server: &McpServer) -> Result<Value, Error> {
    let status = server.status().await;
    let visible = server.backend.launcher_visible().await?;
    let (initialized, revision) = match server.read().await? {
        Some(document) => (true, Some(document.revision)),
        None => (false, None),
    };
    let pending = server.pending().await?;
    Ok(json!({
        "enabled": status.enabled,
        "endpoint": status.endpoint,
        "launcherVisible": visible,
        "initialized": initialized,
        "revision": revision,
        "pendingCandidate": pending.map(|preview| json!({
            "previewId": preview.id,
            "baseRevision": preview.base_revision,
            "ownedByThisGrant": true
        })),
        "review": {
            "keep": "The user must press Keep in Luma.",
            "keepToolAvailable": false
        },
        "protocols": {
            "modern": MODERN_PROTOCOL,
            "legacy": LEGACY_PROTOCOL
        }
    }))
}

fn no_arguments(arguments: Value) -> Result<(), String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "Arguments must be an object.".to_string())?;
    if object.is_empty() {
        Ok(())
    } else {
        Err("This tool accepts no arguments.".into())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CandidateArguments {
    expected_revision: u64,
    config: Value,
}

fn candidate_arguments(arguments: Value) -> Result<(u64, Value), String> {
    let arguments: CandidateArguments =
        serde_json::from_value(arguments).map_err(|error| format!("Invalid arguments: {error}"))?;
    Ok((arguments.expected_revision, arguments.config))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginSettingsArguments {
    plugin_id: String,
    expected_revision: u64,
    settings: BTreeMap<String, Value>,
}

fn plugin_submission_arguments(arguments: Value) -> Result<Value, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "Arguments must be an object.".to_string())?;
    if object.contains_key("manifest") {
        Ok(arguments)
    } else {
        Err("Plugin submission requires manifest and optional files.".into())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelArguments {
    preview_id: String,
}

fn tool_success(value: Value) -> Value {
    let text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|_| "Luma returned structured content.".into());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": false
    })
}

fn tool_failure(error: Error) -> Value {
    let code = error.code;
    let action = action(code);
    let structured = json!({
        "error": {
            "code": code,
            "message": error.message,
            "action": action
        }
    });
    json!({
        "content": [{
            "type": "text",
            "text": format!("{} {}", error.message, action)
        }],
        "structuredContent": structured,
        "isError": true
    })
}

fn tool_protocol_error(message: impl ToString) -> Value {
    let message = message.to_string();
    json!({
        "content": [{ "type": "text", "text": message }],
        "structuredContent": {
            "error": {
                "code": "invalid_arguments",
                "message": message,
                "action": "Correct the tool arguments and call the tool again."
            }
        },
        "isError": true
    })
}

fn action(code: &str) -> &'static str {
    match code {
        "validation" => {
            "Fix the reported field, then validate the complete candidate again. Do not retry an unchanged candidate."
        }
        "conflict" => {
            "Read the current configuration and revision again, apply the change to that revision, then validate and preview again."
        }
        "preview_busy" => {
            "A review is already waiting in Luma. Ask the user to Keep or Reject it before previewing another change."
        }
        "preview_mismatch" => {
            "This preview is no longer active. Start again from a current read, validate, and preview."
        }
        "launcher_unavailable" => {
            "Ask the user to open Luma and keep the launcher visible while you preview. Reads and validation work while Luma is in the tray."
        }
        "not_initialized" => {
            "Ask the user to open Luma once so customization is initialized."
        }
        "grant_revoked" | "mcp_disabled" => {
            "Agent access changed. Ask the user to enable or rotate access in Luma, then use the new connection details."
        }
        "not_owner" => {
            "This tool can cancel only the preview created by the current agent grant. Ask the user to reject any other review in Luma."
        }
        "plugin_missing" => {
            "Stage the plugin package first, or ask the user to import it in Settings > Plugins."
        }
        "plugin_enabled" => {
            "Ask the user to disable the plugin in Luma > Settings > Plugins before staging an update. Do not try to bypass this."
        }
        "plugin_conflict" => {
            "Another enabled plugin already controls one of these fields. Ask the user to disable or remove it, or choose different contributions."
        }
        "plugin_unsupported" => {
            "This Luma version runs declarative plugins only. Report the runtime limitation and do not claim the plugin can be enabled."
        }
        "plugin_settings" | "immutable_version" => {
            "Correct the declared setting or package version, then validate again."
        }
        "storage" => {
            "Luma could not reach customization storage. Ask the user to check Luma before retrying."
        }
        _ => "Report this error to the user. Do not retry blindly.",
    }
}

fn failure(code: &'static str, message: impl ToString) -> Error {
    Error {
        code,
        message: message.to_string(),
    }
}

fn random_token() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| failure("storage", error))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut difference = a.len() ^ b.len();
    for index in 0..a.len().max(b.len()) {
        difference |= usize::from(a.get(index).unwrap_or(&0) ^ b.get(index).unwrap_or(&0));
    }
    difference == 0
}

fn server_info() -> Value {
    json!({
        "name": "luma",
        "title": "Luma",
        "version": env!("CARGO_PKG_VERSION")
    })
}

fn success(id: Value, mut result: Value, modern: bool) -> Response {
    if modern {
        if let Some(object) = result.as_object_mut() {
            object.insert("resultType".into(), json!("complete"));
            let meta = object.entry("_meta").or_insert_with(|| json!({}));
            if let Some(meta) = meta.as_object_mut() {
                meta.entry(META_SERVER_INFO).or_insert_with(server_info);
            }
        }
    }
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    }))
    .into_response()
}

fn error_response(
    id: Option<&Value>,
    status: StatusCode,
    code: i64,
    message: String,
    data: Option<Value>,
) -> Response {
    let mut error = json!({ "code": code, "message": message });
    if let Some(data) = data {
        error["data"] = data;
    }
    let body = match id {
        Some(id) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
        None => json!({ "jsonrpc": "2.0", "error": error }),
    };
    (status, Json(body)).into_response()
}

fn accepted() -> Response {
    StatusCode::ACCEPTED.into_response()
}

fn tool(name: &str, title: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": read_only,
            "destructiveHint": false,
            "idempotentHint": read_only,
            "openWorldHint": false
        }
    })
}

fn empty_schema() -> Value {
    json!({ "type": "object", "additionalProperties": false })
}

fn tool_list() -> Value {
    json!({
        "tools": [
            tool(
                "luma_capabilities_get",
                "Get Luma customization capabilities",
                "Return the supported appearance, layout, widget, app-order, discovery, and review contract. Start here.",
                empty_schema(),
                true
            ),
            tool(
                "luma_config_get",
                "Get current Luma customization",
                "Return the current committed customization document and revision. Does not include credentials, accounts, playback history, or executable paths.",
                empty_schema(),
                true
            ),
            tool(
                "luma_candidate_validate",
                "Validate a Luma customization candidate",
                "Validate a complete candidate configuration against an expected revision. Returns field-level messages without writing anything.",
                candidate_tool_schema(),
                true
            ),
            tool(
                "luma_candidate_preview",
                "Preview a Luma customization candidate",
                "Stage a complete validated candidate as the single review in Luma. Requires the launcher to be open and visible. The user must press Keep in Luma; this tool cannot commit.",
                candidate_tool_schema(),
                false
            ),
            tool(
                "luma_candidate_get",
                "Get this grant's pending Luma preview",
                "Return the preview created by the current agent grant, if it is still pending.",
                empty_schema(),
                true
            ),
            tool(
                "luma_candidate_cancel",
                "Cancel this grant's pending Luma preview",
                "Withdraw only the preview created by the current agent grant. It cannot cancel a preview created in Luma.",
                json!({
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "previewId": {
                            "type": "string",
                            "description": "The previewId returned by luma_candidate_preview."
                        }
                    },
                    "required": ["previewId"]
                }),
                false
            ),
            tool(
                "luma_status",
                "Get Luma agent access status",
                "Return whether agent access is enabled, whether the launcher is visible, the current revision, and this grant's pending preview. No credential is returned.",
                empty_schema(),
                true
            ),
            tool(
                "luma_plugins_list",
                "List installed Luma plugins",
                "Return installed declarative plugins, their enabled state, declared controls, settings schema, current setting values, unsupported runtime state, and the effective configuration. No machine paths or credentials are returned.",
                empty_schema(),
                true
            ),
            tool(
                "luma_plugin_validate",
                "Validate a Luma plugin submission",
                "Validate a manifest plus optional text or base64 files with the same rules as native import. Returns a content hash or field-level validation errors. Writes nothing.",
                plugin_submission_schema(),
                true
            ),
            tool(
                "luma_plugin_stage",
                "Stage a Luma plugin as installed and disabled",
                "Install a validated plugin package into Luma-owned storage with enabled set to false. This never enables, keeps, activates, or executes the plugin. The user must review it in Settings > Plugins.",
                plugin_submission_schema(),
                false
            ),
            tool(
                "luma_plugin_settings_stage",
                "Stage plugin settings for user review",
                "Validate declared plugin setting values and stage them as the single pending Keep review. This cannot keep, enable, or apply the settings.",
                plugin_settings_schema(),
                false
            ),
        ]
    })
}

fn plugin_submission_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "manifest": {
                "type": "object",
                "description": "The luma-extension.json manifest object."
            },
            "files": {
                "type": "array",
                "maxItems": 256,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Package-relative path using forward slashes. Never absolute."
                        },
                        "text": {
                            "type": "string",
                            "description": "UTF-8 file content."
                        },
                        "base64": {
                            "type": "string",
                            "description": "Base64 content for binary assets. Use exactly one of text or base64."
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        "required": ["manifest"]
    })
}

fn plugin_settings_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "pluginId": { "type": "string" },
            "expectedRevision": {
                "type": "integer",
                "minimum": 1,
                "maximum": 9_007_199_254_740_991u64
            },
            "settings": {
                "type": "object",
                "description": "Setting keys declared by the plugin, or qualified pluginId.key names, mapped to boolean, enum string, number, or string values."
            }
        },
        "required": ["pluginId", "expectedRevision", "settings"]
    })
}

fn candidate_tool_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "expectedRevision": {
                "type": "integer",
                "minimum": 1,
                "maximum": 9_007_199_254_740_991u64,
                "description": "The revision returned by luma_config_get."
            },
            "config": candidate_schema()
        },
        "required": ["expectedRevision", "config"]
    })
}

fn candidate_schema() -> Value {
    let defaults = serde_json::to_value(Config::default()).unwrap_or_else(|_| json!({}));
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "A complete customization configuration. Home must contain exactly one row for each of the seven components, and order must contain every app exactly once with Custom last.",
        "properties": {
            "theme": {
                "type": "string",
                "enum": THEMES,
                "default": defaults["theme"],
                "description": "Built-in appearance theme."
            },
            "density": {
                "type": "string",
                "enum": DENSITIES,
                "default": defaults["density"]
            },
            "appTileShape": {
                "type": "string",
                "enum": TILE_SHAPES,
                "default": defaults["appTileShape"]
            },
            "textSize": {
                "type": "string",
                "enum": TEXT_SIZES,
                "default": defaults["textSize"]
            },
            "reducedMotion": {
                "type": "boolean",
                "default": defaults["reducedMotion"]
            },
            "interfaceSounds": {
                "type": "string",
                "enum": SOUND_PRESETS,
                "default": defaults["interfaceSounds"]
            },
            "soundLevel": {
                "type": "string",
                "enum": SOUND_LEVELS,
                "default": defaults["soundLevel"]
            },
            "homeColumns": {
                "type": "integer",
                "enum": [1, 2],
                "default": defaults["homeColumns"]
            },
            "home": {
                "type": "array",
                "minItems": 7,
                "maxItems": 7,
                "description": "Exactly one row for every component.",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "id": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 64,
                            "pattern": "^[A-Za-z0-9_-]+$"
                        },
                        "component": {
                            "type": "string",
                            "enum": COMPONENTS
                        },
                        "visible": { "type": "boolean" },
                        "span": { "type": "integer", "enum": [1, 2] }
                    },
                    "required": ["id", "component", "visible", "span"]
                }
            },
            "widgets": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "spotify": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "enabled": { "type": "boolean" },
                            "placement": { "type": "string", "enum": PLACEMENTS }
                        },
                        "required": ["enabled", "placement"]
                    }
                },
                "required": ["spotify"]
            },
            "order": {
                "type": "array",
                "minItems": 12,
                "maxItems": 12,
                "uniqueItems": true,
                "description": "Every app exactly once. Custom must be last.",
                "items": { "type": "string", "enum": APP_IDS }
            }
        },
        "required": [
            "theme",
            "density",
            "appTileShape",
            "homeColumns",
            "home",
            "widgets",
            "order"
        ]
    })
}

fn capabilities() -> Value {
    let defaults = serde_json::to_value(Config::default()).unwrap_or_else(|_| json!({}));
    let tools = tool_list();
    let tool_summaries: Vec<Value> = tools["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool["name"],
                        "title": tool["title"],
                        "readOnly": tool["annotations"]["readOnlyHint"]
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "schemaVersion": 1,
        "product": {
            "name": "Luma",
            "version": env!("CARGO_PKG_VERSION"),
            "platform": "Windows"
        },
        "protocols": {
            "modern": MODERN_PROTOCOL,
            "legacy": LEGACY_PROTOCOL,
            "legacyHandshake": true
        },
        "layers": {
            "appearance": {
                "supported": true,
                "fields": [
                    "theme",
                    "density",
                    "appTileShape",
                    "textSize",
                    "reducedMotion",
                    "interfaceSounds",
                    "soundLevel"
                ]
            },
            "composition": {
                "supported": true,
                "fields": ["homeColumns", "home", "widgets", "order"]
            },
            "screens": {
                "supported": false,
                "reason": "Custom screens are not implemented in this Luma version."
            },
            "extensions": {
                "supported": true,
                "declarative": true,
                "executable": false,
                "reason": "Declarative plugins that contribute appearance, Home layout, and settings are supported. Executable runtimes are not implemented; such packages install disabled and cannot be enabled."
            },
            "plugins": {
                "supported": true,
                "manifest": "luma-extension.json",
                "fields": plugin::FIELDS,
                "settingTypes": ["boolean", "enum", "number", "string"],
                "runtimes": {
                    "supported": ["declarative"],
                    "installedDisabled": ["isolated-ui", "background", "main-page"]
                },
                "importFormats": ["luma-extension.zip", "manifest plus text/base64 files"],
                "limits": plugin::limits(),
                "review": "MCP installs plugins disabled. Only the user can preview and enable a plugin and press Keep. Settings staged by MCP also require user Keep."
            }
        },
        "appearance": {
            "themes": THEMES,
            "densities": DENSITIES,
            "tileShapes": TILE_SHAPES,
            "textSizes": TEXT_SIZES,
            "reducedMotion": { "type": "boolean" },
            "interfaceSounds": SOUND_PRESETS,
            "soundLevels": SOUND_LEVELS
        },
        "composition": {
            "homeColumns": [1, 2],
            "components": COMPONENTS,
            "homeRow": {
                "id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 64,
                    "pattern": "^[A-Za-z0-9_-]+$"
                },
                "component": COMPONENTS,
                "visible": "boolean",
                "span": [1, 2]
            },
            "homeRows": {
                "count": 7,
                "rule": "Exactly one row for every component; instance ids must be unique."
            },
            "widgets": {
                "spotify": {
                    "enabled": "boolean",
                    "placement": PLACEMENTS
                }
            },
            "apps": {
                "ids": APP_IDS,
                "count": 12,
                "rules": [
                    "Every app exactly once.",
                    "Custom must be the last app."
                ]
            }
        },
        "defaults": defaults,
        "candidateSchema": candidate_schema(),
        "limits": {
            "maxRequestBodyBytes": MAX_BODY,
            "homeRows": 7,
            "apps": 12,
            "pendingPreviews": 1
        },
        "tools": tool_summaries,
        "resources": [
            "luma://customization/capabilities",
            "luma://customization/guidance",
            plugin::SCHEMA_RESOURCE,
            plugin::EXAMPLE_RESOURCE
        ],
        "review": {
            "keep": "The user must press Keep in Luma. No MCP tool can keep, enable, commit, restore, reset, launch, or install-active.",
            "previewRequiresVisibleLauncher": true,
            "readsWorkInTray": true,
            "agentCanCancelOnlyOwnPreview": true,
            "pluginStage": "Installs disabled only.",
            "pluginSettingsStage": "Requires user Keep."
        },
        "unavailable": [
            "Seeing or committing without user Keep.",
            "Enabling, keeping, disabling, removing, or exporting a plugin.",
            "Restoring history or resetting appearance.",
            "Launching apps, changing app paths, or reading account credentials.",
            "Running executable plugin source, scripts, HTML, or native code.",
            "Custom screens."
        ]
    })
}

fn resource_list() -> Value {
    json!({
        "resources": [
            {
                "uri": "luma://customization/capabilities",
                "name": "Luma customization capabilities",
                "title": "Luma customization capabilities",
                "description": "Versioned capability and limit document for the supported appearance and composition layers.",
                "mimeType": "application/json"
            },
            {
                "uri": "luma://customization/guidance",
                "name": "Luma customization guidance",
                "title": "Luma customization guidance",
                "description": "Worked read, validate, preview, and review examples with the rules an agent must not break.",
                "mimeType": "text/markdown"
            },
            {
                "uri": plugin::SCHEMA_RESOURCE,
                "name": "Luma plugin schema",
                "title": "Luma plugin schema",
                "description": "Exact declarative plugin manifest, settings binding, contribution, runtime, and limit contract.",
                "mimeType": "application/json"
            },
            {
                "uri": plugin::EXAMPLE_RESOURCE,
                "name": "Luma plugin example",
                "title": "Luma plugin example",
                "description": "A complete declarative manifest with enum, boolean, and number settings bound to appearance and Home contributions.",
                "mimeType": "application/json"
            }
        ]
    })
}

fn read_resource(uri: &str) -> Result<(&'static str, String), String> {
    match uri {
        "luma://customization/capabilities" => Ok((
            "application/json",
            serde_json::to_string_pretty(&capabilities()).unwrap_or_else(|_| "{}".into()),
        )),
        "luma://customization/guidance" => Ok(("text/markdown", guidance().into())),
        plugin::SCHEMA_RESOURCE => Ok((
            "application/json",
            serde_json::to_string_pretty(&plugin::schema_document())
                .unwrap_or_else(|_| "{}".into()),
        )),
        plugin::EXAMPLE_RESOURCE => Ok((
            "application/json",
            serde_json::to_string_pretty(&plugin::example_manifest())
                .unwrap_or_else(|_| "{}".into()),
        )),
        _ => Err(format!("Resource not found: {uri}.")),
    }
}

fn guidance() -> &'static str {
    "# Luma customization guidance\n\n\
Luma is a Windows TV launcher. Use MCP to inspect the committed customization, validate a complete candidate, and stage it for the user to review. Theme, Home layout, and widgets are Luma's own settings under Settings > Appearance & Home; external declarative plugins can override those fields.\n\n\
## Required flow\n\n\
1. Call `luma_capabilities_get`, `luma_plugins_list`, and read `luma://plugins/schema` and `luma://plugins/example`.\n\
2. Call `luma_config_get` and keep its `revision`.\n\
3. For a configuration change, build a complete effective configuration, not a patch.\n\
4. Call `luma_candidate_validate` with `expectedRevision` and `config`.\n\
5. If it is valid, call `luma_candidate_preview` with the same `expectedRevision` and `config`.\n\
6. Tell the user that the change is waiting in Luma and that they must press Keep. Stop there.\n\n\
## Authoring a declarative plugin\n\n\
1. Write `luma-extension.json` with a stable id, `major.minor.patch` version, `apiVersion: 1`, bounded settings, and declarative contributions. Use only `runtime.kind: \"declarative\"` for an enableable plugin.\n\
2. Put package-relative source and assets in `files`; never use absolute paths, `..`, backslashes, symlinks, credentials, or machine paths.\n\
3. Call `luma_plugin_validate` with `manifest` and optional `files` (`text` or `base64`).\n\
4. Call `luma_plugin_stage` with the same submission. Luma installs it disabled.\n\
5. Tell the user to open Luma > Settings > Plugins > Installed plugins and press Preview and enable, then Keep. No MCP tool can enable it.\n\n\
## Plugin settings\n\n\
A manifest can declare boolean, enum, number, or string settings with validated defaults. Bind settings to supported fields, for example `theme: {\"setting\":\"theme\"}`, `homeColumns: {\"setting\":\"columns\"}`, or `home.rows.featured.visible: {\"setting\":\"showFeatured\"}`. Stage new values with `luma_plugin_settings_stage`; the user must press Keep. Settings for disabled plugins remain stored and apply again when the plugin is enabled.\n\n\
## Examples\n\n\
### Change the theme\n\n\
Read the current document. Change only `config.theme` to a value listed by `luma_capabilities_get`. Keep every other field unchanged, including the full `home` array and `order`. Validate and preview the complete document.\n\n\
### Rearrange Home\n\n\
Read the current document. Reorder `config.home` while keeping exactly one row for each of the seven components. Keep each row's `id`, `component`, `visible`, and `span`. Validate and preview the complete document.\n\n\
### Change the app order\n\n\
Read the current document. Reorder `config.order` so every app appears exactly once and `Custom` remains last. Validate and preview the complete document.\n\n\
## Rules\n\n\
- Preview requires the Luma launcher to be open and visible. If it is in the tray, ask the user to open Luma.\n\
- Keep is a user action in Luma. The MCP tools cannot keep, enable, disable, remove, export, commit, restore, reset, launch, or execute anything.\n\
- A preview creates one pending review. If another review is already pending, ask the user to Keep or Reject it before staging another.\n\
- If a configuration candidate changes a field controlled by an enabled plugin, the preview reports the detached fields and Keep preserves the user's value while detaching that field. Do not describe this as applying the plugin.\n\
- Use `luma_candidate_cancel` only for the preview returned to this grant. Never try to cancel a review created in Luma.\n\
- Revoking or rotating access stops the next request and cancels this grant's pending preview.\n\
- Report validation and revision errors exactly. Re-read the current configuration before retrying after a conflict.\n\
- Executable plugin runtimes are not supported. Packages may include source for future use, but they are installed disabled and cannot be enabled.\n\
- Do not ask for or print tokens, passwords, pairing codes, or account credentials. Luma never returns them through these tools.\n"
}

#[tauri::command]
pub async fn customization_mcp_status(
    access: tauri::State<'_, McpAccess>,
) -> Result<Status, Error> {
    Ok(access.inner().status().await)
}

#[tauri::command]
pub async fn customization_mcp_enable(
    app: tauri::AppHandle,
    access: tauri::State<'_, McpAccess>,
) -> Result<Credentials, Error> {
    McpServer::app(&app, access.inner().clone()).enable().await
}

#[tauri::command]
pub async fn customization_mcp_disable(
    app: tauri::AppHandle,
    access: tauri::State<'_, McpAccess>,
) -> Result<Status, Error> {
    McpServer::app(&app, access.inner().clone()).disable().await
}

#[tauri::command]
pub async fn customization_mcp_rotate(
    app: tauri::AppHandle,
    access: tauri::State<'_, McpAccess>,
) -> Result<Credentials, Error> {
    McpServer::app(&app, access.inner().clone()).rotate().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        sync::{
            atomic::{AtomicBool, Ordering},
            Mutex as StdMutex,
        },
        time::Duration,
    };

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const AUTH: &str = "Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    struct TestState {
        document: Option<Document>,
        preview: Option<Preview>,
        next: usize,
        reads: usize,
        cancels: Vec<String>,
    }

    struct TestBackend {
        state: StdMutex<TestState>,
        visible: AtomicBool,
    }

    impl TestBackend {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                state: StdMutex::new(TestState {
                    document: Some(Document {
                        version: 3,
                        revision: 1,
                        config: Config::default(),
                    }),
                    preview: None,
                    next: 0,
                    reads: 0,
                    cancels: Vec::new(),
                }),
                visible: AtomicBool::new(false),
            })
        }

        fn set_visible(&self, visible: bool) {
            self.visible.store(visible, Ordering::SeqCst);
        }

        fn manual_preview(&self) {
            let mut state = self.state.lock().unwrap();
            let config = state
                .document
                .as_ref()
                .map(|document| document.config.clone())
                .unwrap_or_default();
            state.preview = Some(Preview {
                id: "manual".into(),
                base_revision: 1,
                config,
                plugin: None,
                detached: Vec::new(),
                prepared: None,
                plugin_action: None,
            });
        }

        fn preview(&self) -> Option<Preview> {
            self.state.lock().unwrap().preview.clone()
        }

        fn reads(&self) -> usize {
            self.state.lock().unwrap().reads
        }

        fn cancels(&self) -> Vec<String> {
            self.state.lock().unwrap().cancels.clone()
        }

        fn read_sync(&self) -> Result<Option<Document>, Error> {
            let mut state = self.state.lock().unwrap();
            state.reads += 1;
            Ok(state.document.clone())
        }

        fn preview_sync(&self, request: Commit) -> Result<Preview, Error> {
            if !self.visible.load(Ordering::SeqCst) {
                return Err(failure(
                    "launcher_unavailable",
                    "Open Luma before previewing a layout.",
                ));
            }
            let mut state = self.state.lock().unwrap();
            if state.preview.is_some() {
                return Err(failure(
                    "preview_busy",
                    "Finish or cancel the current preview first.",
                ));
            }
            let current_revision = state
                .document
                .as_ref()
                .map(|document| document.revision)
                .ok_or_else(|| {
                    failure(
                        "not_initialized",
                        "Open Luma once before previewing a layout.",
                    )
                })?;
            if current_revision != request.expected_revision {
                return Err(failure("conflict", "The saved layout changed."));
            }
            document::validate(&request.config)
                .map_err(|message| failure("validation", message))?;
            state.next += 1;
            let preview = Preview {
                id: format!("preview-{}", state.next),
                base_revision: current_revision,
                config: request.config,
                plugin: None,
                detached: Vec::new(),
                prepared: None,
                plugin_action: None,
            };
            state.preview = Some(preview.clone());
            Ok(preview)
        }

        fn cancel_sync(&self, id: &str) -> Result<(), Error> {
            let mut state = self.state.lock().unwrap();
            if state
                .preview
                .as_ref()
                .is_none_or(|preview| preview.id != id)
            {
                return Err(failure(
                    "preview_mismatch",
                    "This preview is no longer active.",
                ));
            }
            state.preview = None;
            state.cancels.push(id.to_string());
            Ok(())
        }

        fn current_sync(&self) -> Result<Option<Preview>, Error> {
            Ok(self.state.lock().unwrap().preview.clone())
        }
    }

    impl Backend for TestBackend {
        fn read<'a>(&'a self) -> BackendFuture<'a, Option<Document>> {
            Box::pin(async move { self.read_sync() })
        }

        fn preview<'a>(&'a self, request: Commit) -> BackendFuture<'a, Preview> {
            Box::pin(async move { self.preview_sync(request) })
        }

        fn cancel_preview<'a>(&'a self, id: &'a str) -> BackendFuture<'a, ()> {
            Box::pin(async move { self.cancel_sync(id) })
        }

        fn current_preview<'a>(&'a self) -> BackendFuture<'a, Option<Preview>> {
            Box::pin(async move { self.current_sync() })
        }

        fn launcher_visible<'a>(&'a self) -> BackendFuture<'a, bool> {
            Box::pin(async move { Ok(self.visible.load(Ordering::SeqCst)) })
        }

        fn plugins<'a>(&'a self) -> BackendFuture<'a, plugin::PluginsState> {
            Box::pin(async move {
                let config = self
                    .state
                    .lock()
                    .unwrap()
                    .document
                    .as_ref()
                    .map(|document| document.config.clone())
                    .unwrap_or_default();
                Ok(plugin::PluginsState {
                    plugins: Vec::new(),
                    effective: config,
                })
            })
        }

        fn validate_plugin<'a>(&'a self, _submission: Value) -> BackendFuture<'a, Value> {
            Box::pin(async move { Ok(json!({ "valid": true })) })
        }

        fn stage_plugin<'a>(&'a self, _submission: Value) -> BackendFuture<'a, plugin::ImportResult> {
            Box::pin(async move {
                Err(failure(
                    "storage",
                    "The test backend does not have plugin storage.",
                ))
            })
        }

        fn stage_plugin_settings<'a>(
            &'a self,
            _plugin_id: String,
            _expected_revision: u64,
            _values: BTreeMap<String, Value>,
        ) -> BackendFuture<'a, Preview> {
            Box::pin(async move {
                Err(failure(
                    "storage",
                    "The test backend does not have plugin storage.",
                ))
            })
        }
    }

    fn access(enabled: bool) -> McpAccess {
        let access = McpAccess::new();
        tauri::async_runtime::block_on(async {
            let mut state = access.state.lock().await;
            state.available = true;
            state.enabled = enabled;
            state.token = enabled.then(|| TOKEN.to_string());
            state.generation = 1;
        });
        access
    }

    fn serve(access: McpAccess, backend: Arc<TestBackend>) -> (u16, McpServer) {
        tauri::async_runtime::block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = McpServer {
                access,
                backend: backend as Arc<dyn Backend>,
                host: format!("127.0.0.1:{port}"),
            };
            let router = router(server.clone());
            tauri::async_runtime::spawn(async move {
                let _ = axum::serve(listener, router).await;
            });
            (port, server)
        })
    }

    fn send(
        port: u16,
        host: &str,
        method: &str,
        headers: &[(&str, String)],
        body: &[u8],
    ) -> (u16, String) {
        let mut request = format!(
            "{method} /mcp HTTP/1.1\r\nhost: {host}\r\ncontent-length: {}\r\nconnection: close\r\n",
            body.len()
        )
        .into_bytes();
        for (name, value) in headers {
            request.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        request.extend_from_slice(b"\r\n");
        request.extend_from_slice(body);
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(&request).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response
            .split("\r\n")
            .next()
            .and_then(|line| line.split(' ').nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap();
        (status, response)
    }

    fn post(port: u16, headers: &[(&str, String)], body: &str) -> (u16, String) {
        send(
            port,
            &format!("127.0.0.1:{port}"),
            "POST",
            headers,
            body.as_bytes(),
        )
    }

    fn headers(protocol: &str) -> Vec<(&'static str, String)> {
        headers_with_auth(protocol, AUTH)
    }

    fn headers_with_auth(protocol: &str, auth: &str) -> Vec<(&'static str, String)> {
        vec![
            ("content-type", "application/json".into()),
            ("accept", "application/json, text/event-stream".into()),
            ("authorization", auth.into()),
            ("mcp-protocol-version", protocol.into()),
        ]
    }

    fn with(
        mut headers: Vec<(&'static str, String)>,
        name: &'static str,
        value: &str,
    ) -> Vec<(&'static str, String)> {
        headers.push((name, value.into()));
        headers
    }

    fn rpc(response: &str) -> Value {
        let (_, body) = response.split_once("\r\n\r\n").unwrap();
        serde_json::from_str(body.trim()).unwrap()
    }

    fn initialize_body() -> String {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": LEGACY_PROTOCOL,
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "1.0.0" }
            }
        })
        .to_string()
    }

    fn call(port: u16, name: &str, arguments: Value) -> Value {
        call_with_auth(port, AUTH, name, arguments)
    }

    fn call_with_auth(port: u16, auth: &str, name: &str, arguments: Value) -> Value {
        call_rpc(port, auth, name, arguments)["result"].clone()
    }

    fn call_rpc(port: u16, auth: &str, name: &str, arguments: Value) -> Value {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        })
        .to_string();
        let (status, response) = post(port, &headers_with_auth(LEGACY_PROTOCOL, auth), &body);
        assert_eq!(status, 200);
        rpc(&response)
    }

    fn legacy_tool_error(result: &Value) -> &str {
        assert_eq!(result["isError"], true);
        result["structuredContent"]["error"]["code"]
            .as_str()
            .unwrap()
    }

    #[test]
    fn legacy_handshake_ping_tools_and_resources_work() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend);
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &initialize_body());
        assert_eq!(status, 200);
        let value = rpc(&response);
        assert_eq!(value["result"]["protocolVersion"], LEGACY_PROTOCOL);
        assert_eq!(value["result"]["serverInfo"]["name"], "luma");
        assert!(value["result"]["capabilities"]["tools"].is_object());
        assert!(value["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("press Keep"));

        let notification =
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &notification);
        assert_eq!(status, 202);
        assert!(!response.contains("\"error\""));

        let ping = json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &ping);
        assert_eq!(status, 200);
        assert_eq!(rpc(&response)["result"], json!({}));

        let list = json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &list);
        assert_eq!(status, 200);
        let value = rpc(&response);
        let tools = value["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"luma_capabilities_get"));
        assert!(names.contains(&"luma_candidate_preview"));
        assert!(names.contains(&"luma_candidate_cancel"));
        assert!(names.contains(&"luma_plugin_validate"));
        assert!(names.contains(&"luma_plugin_stage"));
        assert!(names.contains(&"luma_plugin_settings_stage"));
        assert!(names.contains(&"luma_plugins_list"));
        assert!(!names.iter().any(|name| {
            ["keep", "commit", "restore", "reset", "launch", "install", "enable", "remove", "export"]
                .iter()
                .any(|word| name.contains(word))
        }));
        assert!(value["result"]["resultType"].is_null());

        let resources =
            json!({ "jsonrpc": "2.0", "id": 4, "method": "resources/list" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &resources);
        assert_eq!(status, 200);
        let value = rpc(&response);
        let uris: Vec<&str> = value["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|resource| resource["uri"].as_str().unwrap())
            .collect();
        assert!(uris.contains(&"luma://customization/capabilities"));
        assert!(uris.contains(&"luma://customization/guidance"));
        assert!(uris.contains(&plugin::SCHEMA_RESOURCE));
        assert!(uris.contains(&plugin::EXAMPLE_RESOURCE));

        let read = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "resources/read",
            "params": { "uri": "luma://customization/capabilities" }
        })
        .to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &read);
        assert_eq!(status, 200);
        let value = rpc(&response);
        let text = value["result"]["contents"][0]["text"].as_str().unwrap();
        let capabilities: Value = serde_json::from_str(text).unwrap();
        assert_eq!(
            capabilities["appearance"]["themes"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            capabilities["composition"]["components"]
                .as_array()
                .unwrap()
                .len(),
            7
        );
        assert_eq!(capabilities["layers"]["plugins"]["supported"], true);
        assert_eq!(capabilities["layers"]["plugins"]["runtimes"]["supported"][0], "declarative");

        let read = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "resources/read",
            "params": { "uri": plugin::SCHEMA_RESOURCE }
        })
        .to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &read);
        assert_eq!(status, 200);
        let value = rpc(&response);
        let schema: Value =
            serde_json::from_str(value["result"]["contents"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(schema["apiVersion"], 1);
        assert_eq!(schema["runtime"]["supported"][0], "declarative");
    }

    #[test]
    fn modern_discover_and_headers_are_conformant() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend);
        let discover = json!({
            "jsonrpc": "2.0",
            "id": "d",
            "method": "server/discover",
            "params": {
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let discover_headers = with(headers(MODERN_PROTOCOL), "mcp-method", "server/discover");
        let (status, response) = post(port, &discover_headers, &discover);
        assert_eq!(status, 200);
        let value = rpc(&response);
        assert_eq!(value["result"]["resultType"], "complete");
        assert_eq!(value["result"]["supportedVersions"][0], MODERN_PROTOCOL);
        assert_eq!(value["result"]["supportedVersions"][1], LEGACY_PROTOCOL);
        assert_eq!(value["result"]["_meta"][META_SERVER_INFO]["name"], "luma");

        let list = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let list_headers = with(headers(MODERN_PROTOCOL), "mcp-method", "tools/list");
        let (status, response) = post(port, &list_headers, &list);
        assert_eq!(status, 200);
        let value = rpc(&response);
        assert_eq!(value["result"]["resultType"], "complete");
        assert_eq!(value["result"]["ttlMs"], 60_000);
        assert_eq!(value["result"]["cacheScope"], "private");

        let call = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "luma_capabilities_get",
                "arguments": {},
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let call_headers = with(
            with(headers(MODERN_PROTOCOL), "mcp-method", "tools/call"),
            "mcp-name",
            "luma_capabilities_get",
        );
        let (status, response) = post(port, &call_headers, &call);
        assert_eq!(status, 200);
        let value = rpc(&response);
        assert_eq!(value["result"]["resultType"], "complete");
        assert_eq!(value["result"]["isError"], false);
    }

    #[test]
    fn modern_validation_rejects_bad_versions_and_header_mismatches() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend);
        let unsupported = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {
                "_meta": {
                    META_VERSION: "1900-01-01",
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let unsupported_headers = with(headers("1900-01-01"), "mcp-method", "tools/list");
        let (status, response) = post(port, &unsupported_headers, &unsupported);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32022);

        let mismatch = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let mismatch_headers = with(headers("2025-06-18"), "mcp-method", "tools/list");
        let (status, response) = post(port, &mismatch_headers, &mismatch);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32020);

        let missing_method = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let (status, response) = post(port, &headers(MODERN_PROTOCOL), &missing_method);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32020);

        let missing_meta = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
            "params": {}
        })
        .to_string();
        let missing_headers = with(headers(MODERN_PROTOCOL), "mcp-method", "server/discover");
        let (status, response) = post(port, &missing_headers, &missing_meta);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32602);

        let ping = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping",
            "params": {
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let (status, response) = post(port, &headers(MODERN_PROTOCOL), &ping);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32020);
        let ping_headers = with(headers(MODERN_PROTOCOL), "mcp-method", "ping");
        let (status, response) = post(port, &ping_headers, &ping);
        assert_eq!(status, 200);
        assert_eq!(rpc(&response)["result"]["resultType"], "complete");

        let unknown = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/list",
            "params": {
                "_meta": {
                    META_VERSION: MODERN_PROTOCOL,
                    META_CLIENT_CAPABILITIES: {}
                }
            }
        })
        .to_string();
        let unknown_headers = with(headers(MODERN_PROTOCOL), "mcp-method", "prompts/list");
        let (status, response) = post(port, &unknown_headers, &unknown);
        assert_eq!(status, 404);
        assert_eq!(rpc(&response)["error"]["code"], -32601);
    }

    #[test]
    fn jsonrpc_parse_and_request_shape_errors_are_reported() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend);
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), "{");
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32700);

        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), "[]");
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32600);

        let null_id = json!({ "jsonrpc": "2.0", "id": null, "method": "ping" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &null_id);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32600);

        let missing_id = json!({ "jsonrpc": "2.0", "method": "tools/list" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &missing_id);
        assert_eq!(status, 400);
        assert_eq!(rpc(&response)["error"]["code"], -32600);

        let notification =
            json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" }).to_string();
        let (status, response) = post(port, &headers(LEGACY_PROTOCOL), &notification);
        assert_eq!(status, 202);
        assert!(!response.contains("\"error\""));
    }

    #[test]
    fn transport_rejects_unknown_hosts_origins_and_credentials() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend.clone());
        let body = initialize_body();

        let (status, _) = send(
            port,
            &format!("127.0.0.1:{port}"),
            "POST",
            &headers(LEGACY_PROTOCOL)
                .into_iter()
                .filter(|(name, _)| *name != "authorization")
                .collect::<Vec<_>>(),
            body.as_bytes(),
        );
        assert_eq!(status, 401);
        assert_eq!(backend.reads(), 0);

        let mut wrong = headers(LEGACY_PROTOCOL);
        for (name, value) in &mut wrong {
            if *name == "authorization" {
                *value = "Bearer wrong".into();
            }
        }
        let (status, _) = post(port, &wrong, &body);
        assert_eq!(status, 401);
        assert_eq!(backend.reads(), 0);

        let origin = with(headers(LEGACY_PROTOCOL), "origin", "http://127.0.0.1:9879");
        let (status, _) = post(port, &origin, &body);
        assert_eq!(status, 403);

        let (status, _) = send(
            port,
            "evil.example:9879",
            "POST",
            &headers(LEGACY_PROTOCOL),
            body.as_bytes(),
        );
        assert_eq!(status, 403);

        let (status, _) = send(
            port,
            "localhost:9879",
            "POST",
            &headers(LEGACY_PROTOCOL),
            body.as_bytes(),
        );
        assert_eq!(status, 200);

        let (status, _) = send(port, &format!("127.0.0.1:{port}"), "GET", &[], b"");
        assert_eq!(status, 405);

        let huge = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"{}\"}}",
            "a".repeat(MAX_BODY)
        );
        let (status, _) = post(port, &headers(LEGACY_PROTOCOL), &huge);
        assert_eq!(status, 413);
    }

    #[test]
    fn read_validate_and_status_work_without_a_visible_launcher() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend.clone());
        let status = call(port, "luma_status", json!({}));
        assert_eq!(status["isError"], false);
        assert_eq!(
            status["structuredContent"]["launcherVisible"],
            Value::Bool(false)
        );
        assert_eq!(status["structuredContent"]["revision"], 1);

        let config = call(port, "luma_config_get", json!({}));
        assert_eq!(config["isError"], false);
        assert_eq!(config["structuredContent"]["initialized"], true);
        assert_eq!(config["structuredContent"]["revision"], 1);

        let mut invalid = serde_json::to_value(Config::default()).unwrap();
        invalid["home"][0]["span"] = json!(3);
        let validation = call(
            port,
            "luma_candidate_validate",
            json!({ "expectedRevision": 1, "config": invalid }),
        );
        assert_eq!(validation["isError"], false);
        assert_eq!(validation["structuredContent"]["valid"], false);
        assert!(validation["structuredContent"]["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("span"));

        let conflict = call(
            port,
            "luma_candidate_validate",
            json!({ "expectedRevision": 99, "config": Config::default() }),
        );
        assert_eq!(legacy_tool_error(&conflict), "conflict");

        let preview = call(
            port,
            "luma_candidate_preview",
            json!({ "expectedRevision": 1, "config": Config::default() }),
        );
        assert_eq!(legacy_tool_error(&preview), "launcher_unavailable");
        assert!(backend.preview().is_none());
    }

    #[test]
    fn validation_and_preview_use_the_exact_candidate_and_cancel_own_only() {
        let backend = TestBackend::new();
        backend.set_visible(true);
        let (port, server) = serve(access(true), backend.clone());

        let mut candidate = Config::default();
        candidate.theme = "onyx".into();
        let preview = call(
            port,
            "luma_candidate_preview",
            json!({ "expectedRevision": 1, "config": candidate }),
        );
        assert_eq!(preview["isError"], false);
        let id = preview["structuredContent"]["previewId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(backend.preview().unwrap().id, id);
        assert_eq!(backend.preview().unwrap().config.theme, "onyx");

        let pending = call(port, "luma_candidate_get", json!({}));
        assert_eq!(pending["structuredContent"]["pending"]["previewId"], id);

        let wrong = call(
            port,
            "luma_candidate_cancel",
            json!({ "previewId": "not-mine" }),
        );
        assert_eq!(legacy_tool_error(&wrong), "not_owner");
        assert_eq!(backend.preview().unwrap().id, id);

        let cancelled = call(port, "luma_candidate_cancel", json!({ "previewId": id }));
        assert_eq!(cancelled["structuredContent"]["cancelled"], true);
        assert!(backend.preview().is_none());
        assert_eq!(backend.cancels(), vec![id]);

        let generation = tauri::async_runtime::block_on(server.authorize(TOKEN)).unwrap();
        tauri::async_runtime::block_on(server.rotate()).unwrap();
        let race = tauri::async_runtime::block_on(server.stage_preview(
            generation,
            Commit {
                expected_revision: 1,
                config: Config::default(),
            },
        ))
        .unwrap_err();
        assert_eq!(race.code, "grant_revoked");
    }

    #[test]
    fn rotation_and_disable_revoke_old_tokens_and_cancel_only_owned_previews() {
        let backend = TestBackend::new();
        backend.set_visible(true);
        let (port, server) = serve(access(true), backend.clone());

        let preview = call(
            port,
            "luma_candidate_preview",
            json!({ "expectedRevision": 1, "config": Config::default() }),
        );
        let id = preview["structuredContent"]["previewId"]
            .as_str()
            .unwrap()
            .to_string();
        let credentials = tauri::async_runtime::block_on(server.rotate()).unwrap();
        assert_eq!(credentials.enabled, true);
        assert_eq!(credentials.token.len(), 64);
        assert!(backend.preview().is_none());
        assert_eq!(backend.cancels(), vec![id]);

        let body = initialize_body();
        let (status, _) = post(port, &headers(LEGACY_PROTOCOL), &body);
        assert_eq!(status, 401);

        let auth = format!("Bearer {}", credentials.token);
        let fresh = headers_with_auth(LEGACY_PROTOCOL, &auth);
        let (status, _) = post(port, &fresh, &body);
        assert_eq!(status, 200);

        let preview = call_with_auth(
            port,
            &auth,
            "luma_candidate_preview",
            json!({ "expectedRevision": 1, "config": Config::default() }),
        );
        assert_eq!(preview["isError"], false);
        let id = preview["structuredContent"]["previewId"]
            .as_str()
            .unwrap()
            .to_string();
        let status = tauri::async_runtime::block_on(server.disable()).unwrap();
        assert_eq!(status.enabled, false);
        assert!(backend.preview().is_none());
        assert!(backend.cancels().contains(&id));
        let (status, _) = post(port, &fresh, &body);
        assert_eq!(status, 401);
    }

    #[test]
    fn revocation_never_cancels_a_preview_created_in_luma() {
        let backend = TestBackend::new();
        backend.set_visible(true);
        let (port, server) = serve(access(true), backend.clone());
        let preview = call(
            port,
            "luma_candidate_preview",
            json!({ "expectedRevision": 1, "config": Config::default() }),
        );
        assert_eq!(preview["isError"], false);
        backend.manual_preview();
        let credentials = tauri::async_runtime::block_on(server.rotate()).unwrap();
        assert_eq!(credentials.enabled, true);
        assert_eq!(backend.preview().unwrap().id, "manual");
        assert!(backend.cancels().is_empty());
        let (status, _) = post(port, &headers(LEGACY_PROTOCOL), &initialize_body());
        assert_eq!(status, 401);
    }

    #[test]
    fn forbidden_tools_are_absent_and_unknown_tools_explain_the_boundary() {
        let backend = TestBackend::new();
        backend.set_visible(true);
        let (port, _) = serve(access(true), backend.clone());
        for name in [
            "luma_candidate_keep",
            "luma_keep",
            "luma_commit",
            "luma_restore",
            "luma_reset",
            "luma_launch",
            "luma_extension_install",
            "luma_plugin_enable",
            "luma_plugin_keep",
            "luma_plugin_remove",
            "luma_plugin_export",
        ] {
            let response = call_rpc(port, AUTH, name, json!({}));
            assert_eq!(response["error"]["code"], -32602);
            assert!(response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("never exposes"));
            assert!(response["result"].is_null());
        }
        assert!(backend.preview().is_none());
    }

    #[test]
    fn enable_requires_a_bound_listener_and_returns_a_strong_token() {
        let access = access(false);
        let server = McpServer {
            access: access.clone(),
            backend: TestBackend::new() as Arc<dyn Backend>,
            host: ADDRESS.into(),
        };
        assert_eq!(
            tauri::async_runtime::block_on(server.status()).enabled,
            false
        );
        let credentials = tauri::async_runtime::block_on(server.enable()).unwrap();
        assert_eq!(credentials.enabled, true);
        assert_eq!(credentials.endpoint, ENDPOINT);
        assert_eq!(credentials.token.len(), 64);
        assert_eq!(
            tauri::async_runtime::block_on(server.status()).enabled,
            true
        );
        assert_eq!(
            tauri::async_runtime::block_on(server.disable())
                .unwrap()
                .enabled,
            false
        );
        assert_eq!(
            tauri::async_runtime::block_on(server.status()).enabled,
            false
        );

        let unavailable = McpServer {
            access: McpAccess::new(),
            backend: TestBackend::new() as Arc<dyn Backend>,
            host: ADDRESS.into(),
        };
        match tauri::async_runtime::block_on(unavailable.enable()) {
            Ok(_) => panic!("enable must fail when no listener is bound"),
            Err(error) => assert_eq!(error.code, "mcp_unavailable"),
        }
        assert_eq!(
            tauri::async_runtime::block_on(unavailable.status()).enabled,
            false
        );
    }

    #[test]
    fn tokens_are_strong_and_comparisons_are_exact() {
        let first = random_token().unwrap();
        let second = random_token().unwrap();
        assert_eq!(first.len(), 64);
        assert_eq!(second.len(), 64);
        assert_ne!(first, second);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(same(&first, &first));
        assert!(!same(&first, &second));
        assert!(!same(&first, &first[..63]));
        assert!(!same(&first, &format!("{first}x")));
    }

    #[test]
    fn capability_choices_remain_pinned_to_the_shared_validator() {
        for theme in THEMES {
            let mut config = Config::default();
            config.theme = theme.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for density in DENSITIES {
            let mut config = Config::default();
            config.density = density.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for shape in TILE_SHAPES {
            let mut config = Config::default();
            config.app_tile_shape = shape.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for size in TEXT_SIZES {
            let mut config = Config::default();
            config.text_size = size.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for sounds in SOUND_PRESETS {
            let mut config = Config::default();
            config.interface_sounds = sounds.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for level in SOUND_LEVELS {
            let mut config = Config::default();
            config.sound_level = level.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for placement in PLACEMENTS {
            let mut config = Config::default();
            config.widgets.spotify.placement = placement.into();
            assert_eq!(document::validate(&config), Ok(()));
        }
        for span in [1, 2] {
            let mut config = Config::default();
            config.home[0].span = span;
            assert_eq!(document::validate(&config), Ok(()));
        }
        for columns in [1, 2] {
            let mut config = Config::default();
            config.home_columns = columns;
            assert_eq!(document::validate(&config), Ok(()));
        }
        let mut config = Config::default();
        config.order = APP_IDS.iter().map(|app| (*app).to_string()).collect();
        assert_eq!(document::validate(&config), Ok(()));
        for component in COMPONENTS {
            let mut config = Config::default();
            config.home.retain(|row| row.component != component);
            config.home.push(super::document::HomeRow {
                id: format!("{component}-replacement"),
                component: component.into(),
                visible: true,
                span: 2,
            });
            if component == "hero" {
                config.home.retain(|row| row.id != "hero");
            }
            assert_eq!(document::validate(&config), Ok(()));
        }
        let defaults: Config = serde_json::from_value(capabilities()["defaults"].clone()).unwrap();
        assert_eq!(defaults, Config::default());
        let schema = capabilities()["candidateSchema"].clone();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["theme"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            schema["properties"]["home"]["items"]["properties"]["component"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            7
        );
        assert_eq!(
            schema["properties"]["order"]["items"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            APP_IDS.len()
        );
    }
}
