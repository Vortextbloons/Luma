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
const DECLARATIVE_TEMPLATE: &str = "luma://plugins/templates/declarative";
const WEATHER_TEMPLATE: &str = "luma://plugins/templates/weather";
const PHONE_COMMAND_TEMPLATE: &str = "luma://plugins/templates/phone-command";
const SVELTE_TEMPLATE: &str = "luma://plugins/templates/svelte";
const WEATHER_MANIFEST: &str =
    include_str!("../../../examples/plugins/com.example.weather/luma-extension.json");
const WEATHER_HTML: &str =
    include_str!("../../../examples/plugins/com.example.weather/ui/index.html");
const WEATHER_SCRIPT: &str =
    include_str!("../../../examples/plugins/com.example.weather/ui/page.js");
const WEATHER_STYLE: &str =
    include_str!("../../../examples/plugins/com.example.weather/ui/page.css");
const PHONE_MANIFEST: &str = include_str!("../../templates/phone-command/luma-extension.json");
const PHONE_HTML: &str = include_str!("../../templates/phone-command/ui/index.html");
const PHONE_SCRIPT: &str = include_str!("../../templates/phone-command/ui/main.js");
const PHONE_STYLE: &str = include_str!("../../templates/phone-command/ui/style.css");
const INSTRUCTIONS: &str = "Luma is a Windows TV launcher with an agent-first authoring contract for customization and plugins. Start with luma_capabilities_get, luma_plugins_list, and luma_status, then read the resource that matches the task. For customization, call luma_config_get, build a complete candidate, call luma_candidate_validate, then luma_candidate_preview; the user must press Keep in Luma. For a plugin, read luma://plugins/schema and one complete template from luma://plugins/templates/declarative, luma://plugins/templates/weather, or luma://plugins/templates/phone-command. Edit the template in your workspace and pass its submission object to luma_plugin_validate. luma_plugin_stage installs a package disabled for production review: declarative packages are then previewed and enabled by the user with Keep, while executable isolated-ui and background packages require the user to review their exact permissions, network origins, and trigger in Settings > Plugins before anything runs. For an executable draft, call luma_plugin_dev_start with the validated submission; it returns a sessionId and starts a disposable Luma profile with separate app data and grants. The draft stays disabled until the user approves access in the development window. Use luma_plugin_dev_invoke for a declared command, luma_plugin_dev_status for readiness and diagnostics, luma_plugin_dev_reload for the same session, and luma_plugin_dev_stop when finished. Development tools never change the production profile. No MCP tool can approve access, enable, keep, commit, disable, remove, export, restore, reset, launch, or execute a package in production. Settings changes for an installed plugin can be staged with luma_plugin_settings_stage for the same user Keep review. Report validation and revision errors exactly, then re-read before retrying.";

const THEMES: [&str; 9] = [
    "violet-night",
    "sapphire",
    "mint",
    "ember",
    "graphite",
    "onyx",
    "aurora",
    "synthwave",
    "biolume",
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
const PLACEMENTS: [&str; 9] = [
    "bar",
    "top-left",
    "top-center",
    "top-right",
    "bottom-left",
    "bottom-center",
    "bottom-right",
    "rail-left",
    "rail-right",
];

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
    fn dev_start<'a>(&'a self, submission: Value) -> BackendFuture<'a, Value>;
    fn dev_reload<'a>(&'a self, session_id: String, submission: Value) -> BackendFuture<'a, Value>;
    fn dev_invoke<'a>(&'a self, session_id: String, command: String) -> BackendFuture<'a, Value>;
    fn dev_status<'a>(&'a self, session_id: String) -> BackendFuture<'a, Value>;
    fn dev_stop<'a>(&'a self, session_id: String) -> BackendFuture<'a, Value>;
}

#[derive(Clone)]
struct AppBackend(tauri::AppHandle);

impl Backend for AppBackend {
    fn read<'a>(&'a self) -> BackendFuture<'a, Option<Document>> {
        let app = self.0.clone();
        Box::pin(async move {
            tauri::async_runtime::spawn_blocking(move || {
                app.state::<super::Store>().read_effective()
            })
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
                if !outcome.already_installed {
                    crate::extension_spike::invalidate(&app, &outcome.plugin.id);
                }
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
                let preview =
                    store.preview_plugin_settings(&plugin_id, expected_revision, &values)?;
                super::publish_preview(&app);
                Ok(preview)
            })
            .await
            .map_err(|error| failure("storage", error))?
        })
    }

    fn dev_start<'a>(&'a self, submission: Value) -> BackendFuture<'a, Value> {
        let app = self.0.clone();
        Box::pin(async move { crate::developer::mcp_start(&app, submission).await })
    }

    fn dev_reload<'a>(&'a self, session_id: String, submission: Value) -> BackendFuture<'a, Value> {
        let app = self.0.clone();
        Box::pin(async move { crate::developer::mcp_reload(&app, &session_id, submission).await })
    }

    fn dev_invoke<'a>(&'a self, session_id: String, command: String) -> BackendFuture<'a, Value> {
        let app = self.0.clone();
        Box::pin(async move { crate::developer::mcp_invoke(&app, &session_id, &command).await })
    }

    fn dev_status<'a>(&'a self, session_id: String) -> BackendFuture<'a, Value> {
        let app = self.0.clone();
        Box::pin(async move { crate::developer::mcp_status(&app, &session_id).await })
    }

    fn dev_stop<'a>(&'a self, session_id: String) -> BackendFuture<'a, Value> {
        let app = self.0.clone();
        Box::pin(async move { crate::developer::mcp_stop(&app, &session_id).await })
    }
}

struct Access {
    enabled: bool,
    available: bool,
    token: Option<String>,
    generation: u64,
    owned: Option<OwnedPreview>,
    dev_sessions: Vec<String>,
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
                dev_sessions: Vec::new(),
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
        let (owned, dev_sessions) = {
            let mut state = self.access.state.lock().await;
            state.enabled = false;
            state.token = None;
            state.generation = state.generation.saturating_add(1);
            (state.owned.take(), std::mem::take(&mut state.dev_sessions))
        };
        if let Some(preview) = owned {
            let _ = self.backend.cancel_preview(&preview.id).await;
        }
        self.stop_dev_sessions(dev_sessions).await;
        Ok(Status {
            enabled: false,
            endpoint: ENDPOINT,
        })
    }

    async fn rotate(&self) -> Result<Credentials, Error> {
        let (token, owned, dev_sessions) = {
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
            (
                token,
                state.owned.take(),
                std::mem::take(&mut state.dev_sessions),
            )
        };
        if let Some(preview) = owned {
            let _ = self.backend.cancel_preview(&preview.id).await;
        }
        self.stop_dev_sessions(dev_sessions).await;
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

    async fn ensure_write(&self, generation: u64) -> Result<(), Error> {
        let state = self.access.state.lock().await;
        if !state.enabled || !state.available {
            return Err(failure(
                "grant_revoked",
                "Agent access was disabled before this change could start.",
            ));
        }
        if state.generation != generation {
            return Err(failure(
                "grant_revoked",
                "Agent access was rotated before this change could start.",
            ));
        }
        Ok(())
    }

    async fn start_dev(&self, submission: Value, generation: u64) -> Result<Value, Error> {
        let result = self.backend.dev_start(submission).await?;
        let session = result
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_string);
        {
            let mut state = self.access.state.lock().await;
            if state.enabled && state.available && state.generation == generation {
                if let Some(session) = session {
                    state.dev_sessions.clear();
                    state.dev_sessions.push(session);
                }
                return Ok(result);
            }
        }
        if let Some(session) = session {
            let _ = self.backend.dev_stop(session).await;
        }
        Err(failure(
            "grant_revoked",
            "Agent access changed before the development session could start.",
        ))
    }

    async fn stop_dev(&self, session_id: &str) -> Result<Value, Error> {
        let result = self.backend.dev_stop(session_id.to_string()).await;
        let mut state = self.access.state.lock().await;
        state.dev_sessions.retain(|known| known != session_id);
        result
    }

    async fn stop_dev_sessions(&self, sessions: Vec<String>) {
        for session in sessions {
            let _ = self.backend.dev_stop(session).await;
        }
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
    let (owned, dev_sessions) = {
        let mut state = access.state.lock().await;
        state.available = false;
        state.enabled = false;
        state.token = None;
        state.generation = state.generation.saturating_add(1);
        state.start_error = Some(format!("Agent access stopped listening on {ADDRESS}."));
        (state.owned.take(), std::mem::take(&mut state.dev_sessions))
    };
    let backend = AppBackend(app);
    if let Some(preview) = owned {
        let _ = backend.cancel_preview(&preview.id).await;
    }
    for session in dev_sessions {
        let _ = backend.dev_stop(session).await;
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
            | "luma_plugin_dev_start"
            | "luma_plugin_dev_reload"
            | "luma_plugin_dev_invoke"
            | "luma_plugin_dev_status"
            | "luma_plugin_dev_stop"
    )
}

#[cfg(test)]
pub(crate) fn known_tool_for_test(name: &str) -> bool {
    known_tool(name)
}

fn unknown_tool_message(name: &str) -> String {
    format!(
        "Unknown tool: {name}. Luma exposes discover, read, validate, preview, plugin validation, disabled plugin staging, plugin settings staging, disposable executable plugin development, candidate status, candidate cancellation, and service status. It never exposes production access approval, enable, keep, commit, disable, remove, export, restore, reset, launch, install-active, or unapproved execution. Ask the user to perform those actions in Luma."
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
                    "settingsReview": "Use luma_plugin_settings_stage to stage setting values. The user must press Keep.",
                    "development": "Use luma_plugin_dev_start for an executable draft. Development sessions are disposable, keep separate app data and grants, and require the user to approve access in the development window."
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
            Ok(submission) => {
                let package = match plugin::validate_submission(&submission) {
                    Ok(package) => package,
                    Err(error) => return tool_failure(error),
                };
                let declared = submission_details(&package);
                let review = staging_review(&package);
                match server.backend.stage_plugin(submission).await {
                    Ok(result) => tool_success(json!({
                        "plugin": result.plugin,
                        "alreadyInstalled": result.already_installed,
                        "replacedVersion": result.replaced_version,
                        "enabled": result.plugin.enabled,
                        "status": if result.already_installed { "already_installed" } else { "installed_disabled" },
                        "declared": declared,
                        "review": review,
                        "next": staging_next(&package, result.already_installed, result.plugin.enabled)
                    })),
                    Err(error) => tool_failure(error),
                }
            }
            Err(message) => tool_protocol_error(message),
        },
        "luma_plugin_dev_start" => match plugin_submission_arguments(arguments) {
            Ok(submission) => {
                let package = match plugin::validate_submission(&submission) {
                    Ok(package) => package,
                    Err(error) => return tool_failure(error),
                };
                if !package.manifest.runtime.kind.is_executable() {
                    return tool_failure(failure(
                        "plugin_unsupported",
                        "Development sessions run only executable isolated-ui or background packages. Validate and stage a declarative package, then ask the user to preview and enable it in Settings > Plugins.",
                    ));
                }
                if let Err(error) = server.ensure_write(generation).await {
                    return tool_failure(error);
                }
                let declared = submission_details(&package);
                let review = development_review(&package);
                match server.start_dev(submission, generation).await {
                    Ok(result) => tool_success(with_dev_review(
                        result,
                        declared,
                        review,
                        "Ask the user to review the draft in the Luma Development window: open Settings > Plugins, check the exact permissions, network origins, and trigger, then choose Allow and enable. Use luma_plugin_dev_status to check readiness. MCP cannot approve access.",
                    )),
                    Err(error) => tool_failure(error),
                }
            }
            Err(message) => tool_protocol_error(message),
        },
        "luma_plugin_dev_reload" => {
            let arguments = match dev_reload_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(message) => return tool_protocol_error(message),
            };
            let submission = json!({
                "manifest": arguments.manifest,
                "files": arguments.files
            });
            let package = match plugin::validate_submission(&submission) {
                Ok(package) => package,
                Err(error) => return tool_failure(error),
            };
            if !package.manifest.runtime.kind.is_executable() {
                return tool_failure(failure(
                    "plugin_unsupported",
                    "Development sessions run only executable isolated-ui or background packages. Declarative packages are staged for user preview and Keep.",
                ));
            }
            if let Err(error) = server.ensure_write(generation).await {
                return tool_failure(error);
            }
            let declared = submission_details(&package);
            let review = development_review(&package);
            match server
                .backend
                .dev_reload(arguments.session_id, submission)
                .await
            {
                Ok(result) => tool_success(with_dev_review(
                    result,
                    declared,
                    review,
                    "If the edit changed permissions or network origins, the draft is disabled again until the user reviews it in the Luma Development window. Otherwise the same session keeps its existing approval. Use luma_plugin_dev_status to verify readiness.",
                )),
                Err(error) => tool_failure(error),
            }
        }
        "luma_plugin_dev_invoke" => {
            let arguments = match dev_command_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(message) => return tool_protocol_error(message),
            };
            if let Err(error) = server.ensure_write(generation).await {
                return tool_failure(error);
            }
            match server
                .backend
                .dev_invoke(arguments.session_id, arguments.command)
                .await
            {
                Ok(result) => tool_success(with_next(
                    result,
                    "Call luma_plugin_dev_status and match commandRun.invocationId to the returned invocationId. A plugin can report its feature result in commandRun.outcome. Until then, delivery is the only confirmed result.",
                )),
                Err(error) => tool_failure(error),
            }
        }
        "luma_plugin_dev_status" => {
            let arguments = match dev_session_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(message) => return tool_protocol_error(message),
            };
            match server.backend.dev_status(arguments.session_id).await {
                Ok(result) => tool_success(without_dev_paths(result)),
                Err(error) => tool_failure(error),
            }
        }
        "luma_plugin_dev_stop" => {
            let arguments = match dev_session_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(message) => return tool_protocol_error(message),
            };
            if let Err(error) = server.ensure_write(generation).await {
                return tool_failure(error);
            }
            match server.stop_dev(&arguments.session_id).await {
                Ok(result) => tool_success(with_next(
                    result,
                    "The disposable development window closes. Call luma_plugin_stage with the final submission and ask the user to review the installed package in Settings > Plugins.",
                )),
                Err(error) => tool_failure(error),
            }
        }
        "luma_plugin_settings_stage" => {
            let arguments: PluginSettingsArguments = match serde_json::from_value(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_protocol_error(format!("Invalid arguments: {error}")),
            };
            let state = match server.backend.plugins().await {
                Ok(state) => state,
                Err(error) => return tool_failure(error),
            };
            let Some(record) = state
                .plugins
                .iter()
                .find(|plugin| plugin.id == arguments.plugin_id)
            else {
                return tool_failure(failure(
                    "plugin_missing",
                    "That plugin is not installed. Stage the plugin first.",
                ));
            };
            let declared: Vec<&str> = record
                .settings
                .iter()
                .map(|setting| setting.key.as_str())
                .collect();
            for key in arguments.settings.keys() {
                if !declared.contains(&key.as_str())
                    && !record
                        .settings
                        .iter()
                        .any(|setting| &setting.qualified_key == key)
                {
                    return tool_failure(failure(
                        "plugin_settings",
                        format!("{} does not declare a setting named {key}.", record.name),
                    ));
                }
            }
            let mut values = BTreeMap::new();
            for (key, value) in arguments.settings {
                let key = key
                    .rsplit_once('.')
                    .map(|(_, key)| key.to_string())
                    .unwrap_or(key);
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
        "development": {
            "supported": true,
            "disposable": true,
            "approvalRequired": true,
            "tools": [
                "luma_plugin_dev_start",
                "luma_plugin_dev_reload",
                "luma_plugin_dev_invoke",
                "luma_plugin_dev_status",
                "luma_plugin_dev_stop"
            ]
        },
        "protocols": {
            "modern": MODERN_PROTOCOL,
            "legacy": LEGACY_PROTOCOL
        }
    }))
}

fn submission_details(package: &plugin::PreparedPackage) -> Value {
    let manifest = &package.manifest;
    let permissions: Vec<&str> = manifest
        .permissions
        .iter()
        .map(|permission| permission.label())
        .collect();
    let network: Vec<String> = manifest
        .network
        .as_ref()
        .map(|network| network.allow.clone())
        .unwrap_or_default();
    json!({
        "apiVersion": manifest.api_version,
        "runtime": manifest.runtime.kind.label(),
        "executable": manifest.runtime.kind.is_executable(),
        "permissions": permissions,
        "network": network,
        "homeCards": manifest.contributes.cards,
        "commands": manifest.contributes.commands,
        "remote": manifest.remote,
        "files": package.files.len(),
        "bytes": package.size,
        "contentHash": package.hash
    })
}

fn staging_review(package: &plugin::PreparedPackage) -> Value {
    let executable = package.manifest.runtime.kind.is_executable();
    json!({
        "kind": if executable { "executable" } else { "declarative" },
        "requiresExplicitAccessApproval": executable,
        "permissions": package.manifest.permissions.iter().map(|permission| permission.label()).collect::<Vec<_>>(),
        "network": package.manifest.network.as_ref().map(|network| network.allow.clone()).unwrap_or_default(),
        "homeCards": package.manifest.contributes.cards,
        "commands": package.manifest.contributes.commands,
        "remote": package.manifest.remote,
        "trigger": if package.manifest.api_version == plugin::HOME_API_VERSION {
            json!("on-demand")
        } else if executable {
            json!("chosen during access review")
        } else {
            Value::Null
        },
        "approval": if executable {
            "The user must open Luma > Settings > Plugins, review the exact permissions, network origins, and trigger, then choose Allow and enable. MCP cannot approve access."
        } else {
            "The user must open Luma > Settings > Plugins, preview the plugin, and press Keep. MCP cannot enable or keep it."
        }
    })
}

fn development_review(package: &plugin::PreparedPackage) -> Value {
    json!({
        "kind": "executable",
        "scope": "disposable",
        "productionChanged": false,
        "requiresExplicitAccessApproval": true,
        "permissions": package.manifest.permissions.iter().map(|permission| permission.label()).collect::<Vec<_>>(),
        "network": package.manifest.network.as_ref().map(|network| network.allow.clone()).unwrap_or_default(),
        "homeCards": package.manifest.contributes.cards,
        "commands": package.manifest.contributes.commands,
        "remote": package.manifest.remote,
        "trigger": if package.manifest.api_version == plugin::HOME_API_VERSION {
            json!("on-demand")
        } else {
            json!("chosen by the user during access review")
        },
        "approval": "The user must review the draft in the Luma Development window: open Settings > Plugins, check the exact permissions, network origins, and trigger, then choose Allow and enable. This approval belongs only to the disposable session. MCP cannot approve access."
    })
}

fn staging_next(
    package: &plugin::PreparedPackage,
    already_installed: bool,
    enabled: bool,
) -> String {
    if already_installed && enabled {
        return "The same package is already installed and enabled. No change was made.".into();
    }
    if already_installed {
        return if package.manifest.runtime.kind.is_executable() {
            "The same package is already installed and still disabled. Ask the user to review its exact permissions, network origins, and trigger in Luma > Settings > Plugins before enabling it.".into()
        } else {
            "The same package is already installed and still disabled. Ask the user to preview it and press Keep in Luma > Settings > Plugins.".into()
        };
    }
    if package.manifest.runtime.kind.is_executable() {
        "Tell the user the plugin is installed but disabled. They must open Luma > Settings > Plugins, review its exact permissions, network origins, and trigger, then choose Allow and enable. No MCP tool can approve access or enable it.".into()
    } else {
        "Tell the user the plugin is installed but disabled. They must open Luma > Settings > Plugins, preview the plugin, and press Keep. No MCP tool can enable or keep it.".into()
    }
}

fn with_dev_review(mut result: Value, declared: Value, review: Value, next: &str) -> Value {
    if !result.is_object() {
        result = json!({ "session": result });
    }
    if let Some(object) = result.as_object_mut() {
        object.remove("appDataDir");
        object.insert("declared".into(), declared);
        object.insert("review".into(), review);
        object.insert("next".into(), json!(next));
    }
    result
}

fn with_next(mut result: Value, next: &str) -> Value {
    if !result.is_object() {
        result = json!({ "result": result });
    }
    if let Some(object) = result.as_object_mut() {
        object.remove("appDataDir");
        object.insert("next".into(), json!(next));
    }
    result
}

fn without_dev_paths(mut result: Value) -> Value {
    if let Some(object) = result.as_object_mut() {
        object.remove("appDataDir");
    }
    result
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DevReloadArguments {
    session_id: String,
    manifest: Value,
    #[serde(default)]
    files: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DevCommandArguments {
    session_id: String,
    command: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DevSessionArguments {
    session_id: String,
}

fn plugin_submission_arguments(arguments: Value) -> Result<Value, String> {
    let mut object = arguments
        .as_object()
        .cloned()
        .ok_or_else(|| "Arguments must be an object.".to_string())?;
    if !object.contains_key("manifest") {
        return Err("Plugin submission requires manifest and optional files.".into());
    }
    if !object.contains_key("files") {
        object.insert("files".into(), json!([]));
    }
    Ok(Value::Object(object))
}

fn dev_reload_arguments(arguments: Value) -> Result<DevReloadArguments, String> {
    let arguments: DevReloadArguments =
        serde_json::from_value(arguments).map_err(|error| format!("Invalid arguments: {error}"))?;
    if arguments.session_id.trim().is_empty() {
        return Err("sessionId must not be empty.".into());
    }
    Ok(arguments)
}

fn dev_command_arguments(arguments: Value) -> Result<DevCommandArguments, String> {
    let arguments: DevCommandArguments =
        serde_json::from_value(arguments).map_err(|error| format!("Invalid arguments: {error}"))?;
    if arguments.session_id.trim().is_empty() {
        return Err("sessionId must not be empty.".into());
    }
    if !valid_local_id(&arguments.command) {
        let limit = plugin::limits()["localId"].as_u64().unwrap_or(32);
        return Err(format!(
            "command must be 1 to {limit} characters, start with a lowercase letter, and use only letters, digits, '-', or '_'."
        ));
    }
    Ok(arguments)
}

fn dev_session_arguments(arguments: Value) -> Result<DevSessionArguments, String> {
    let arguments: DevSessionArguments =
        serde_json::from_value(arguments).map_err(|error| format!("Invalid arguments: {error}"))?;
    if arguments.session_id.trim().is_empty() {
        return Err("sessionId must not be empty.".into());
    }
    Ok(arguments)
}

fn valid_local_id(value: &str) -> bool {
    let limit = plugin::limits()["localId"].as_u64().unwrap_or(32) as usize;
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(byte) if byte.is_ascii_lowercase())
        && value.len() <= limit
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
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
            "Use an executable isolated-ui or background package for a development session. Declarative packages are staged and enabled by the user in Settings > Plugins. Report main-page limitations without claiming the package can run."
        }
        "plugin_runtime" => {
            "Use an executable package for a development session. Declarative packages are staged for user preview and Keep."
        }
        "plugin_settings" | "immutable_version" => {
            "Correct the declared setting or package version, then validate again."
        }
        "developer" | "dev_unavailable" => {
            "Development sessions require a running desktop Luma. Ask the user to open Luma, then start the session again."
        }
        "dev_session" => {
            "That development session is not active. Call luma_plugin_dev_start again and use the returned sessionId."
        }
        "dev_approval" | "plugin_grant" | "plugin_denied" => {
            "Ask the user to open the Luma Development window, review Settings > Plugins, and allow the exact access. Do not retry until the user approves."
        }
        "dev_command" | "plugin_command" => {
            "Use a command declared by the active package. Read luma_plugin_dev_status for the current declaration."
        }
        "dev_busy" | "plugin_busy" => {
            "Wait for the current development command or reload to finish, then call luma_plugin_dev_status."
        }
        "developer_session" => {
            "That development session is not active or its host exited. Call luma_plugin_dev_start again and use the returned sessionId."
        }
        "developer_start" | "developer_reload" => {
            "The development host could not complete the last step. Check the returned message, keep the last working draft, and retry or start a new session."
        }
        "plugin_disabled" => {
            "Ask the user to approve and enable the draft in the Luma Development window before invoking a command."
        }
        "plugin_not_ready" => {
            "Wait for the runtime to become ready, then call luma_plugin_dev_status and retry the declared command."
        }
        "plugin_command_timeout" | "stale" => {
            "The runtime did not finish in time. Call luma_plugin_dev_status, check the development window, then retry if the draft is still active."
        }
        "incompatible" => {
            "Check the package apiVersion and runtime pairing. Development sessions run only executable isolated-ui or background packages; declarative packages are staged for user preview and Keep."
        }
        "denied" => {
            "The package's declared permissions did not allow that action. Ask the user to review access instead of bypassing it."
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
        path: None,
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
                "Return the supported appearance, layout, widget, app-order, plugin, development, discovery, and review contract. Start here.",
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
                "Return whether agent access is enabled, whether the launcher is visible, the current revision, this grant's pending preview, and the available development tools. No credential is returned.",
                empty_schema(),
                true
            ),
            tool(
                "luma_plugins_list",
                "List installed Luma plugins",
                "Return installed plugins, their enabled state, declared controls, settings schema, current setting values, runtime permissions, unsupported runtime state, and the effective configuration. No machine paths or credentials are returned.",
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
            tool(
                "luma_plugin_dev_start",
                "Start a disposable plugin development session",
                "Start a separate Luma development profile for an executable isolated-ui or background package. The draft installs disabled and starts only after the user approves its exact permissions, network origins, and trigger in the development window. Production settings and grants are never changed. Returns the sessionId used by the other development tools.",
                plugin_submission_schema(),
                false
            ),
            tool(
                "luma_plugin_dev_reload",
                "Reload a disposable plugin development session",
                "Validate a complete package against the same limits and replace the active snapshot in an existing development session. A failed validation leaves the last working draft available. A change to requested access requires another explicit user approval.",
                plugin_dev_reload_schema(),
                false
            ),
            tool(
                "luma_plugin_dev_invoke",
                "Invoke a declared command in a development session",
                "Deliver a command declared by the active package to its development runtime. The user must already have approved the draft's access. Delivery is separate from the plugin's feature outcome; use luma_plugin_dev_status for readiness and diagnostics.",
                dev_command_schema(),
                false
            ),
            tool(
                "luma_plugin_dev_status",
                "Get development session status",
                "Return the active plugin, runtime readiness, approval state, and bounded diagnostics for one disposable development session. No credential is returned.",
                dev_session_schema(),
                true
            ),
            tool(
                "luma_plugin_dev_stop",
                "Stop a disposable plugin development session",
                "Close one development window and discard its disposable profile. The running production launcher, installed plugins, settings, and grants are unchanged.",
                dev_session_schema(),
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

fn plugin_dev_reload_schema() -> Value {
    let mut schema = plugin_submission_schema();
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        properties.insert(
            "sessionId".into(),
            json!({
                "type": "string",
                "minLength": 1,
                "description": "The sessionId returned by luma_plugin_dev_start."
            }),
        );
    }
    if let Some(object) = schema.as_object_mut() {
        object.insert("required".into(), json!(["sessionId", "manifest"]));
    }
    schema
}

fn dev_session_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sessionId": {
                "type": "string",
                "minLength": 1,
                "description": "The sessionId returned by luma_plugin_dev_start."
            }
        },
        "required": ["sessionId"]
    })
}

fn dev_command_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sessionId": {
                "type": "string",
                "minLength": 1,
                "description": "The sessionId returned by luma_plugin_dev_start."
            },
            "command": {
                "type": "string",
                "minLength": 1,
                "maxLength": plugin::limits()["localId"],
                "pattern": "^[a-z][A-Za-z0-9_-]*$",
                "description": "A command declared in the active package's contributes.commands."
            }
        },
        "required": ["sessionId", "command"]
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
                "maxProperties": 32,
                "additionalProperties": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "type": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 64,
                            "pattern": "^[A-Za-z0-9_.-]+$"
                        },
                        "enabled": { "type": "boolean" },
                        "placement": { "type": "string", "enum": PLACEMENTS },
                        "visibility": { "type": "string", "enum": ["always", "available", "playing"] },
                        "position": { "anyOf": [{ "type": "null" }, { "type": "object", "additionalProperties": false, "properties": { "x": { "type": "integer", "minimum": 0, "maximum": 1000 }, "y": { "type": "integer", "minimum": 0, "maximum": 1000 } }, "required": ["x", "y"] }] },
                        "size": { "type": "string", "enum": ["small", "medium", "large"] },
                        "variant": { "type": "string", "minLength": 1, "maxLength": 32, "pattern": "^[A-Za-z0-9_-]+$" },
                        "presence": { "type": "array", "minItems": 1, "maxItems": 3, "uniqueItems": true, "items": { "type": "string", "enum": ["home", "music", "idle"] } },
                        "order": { "type": "integer", "minimum": 0, "maximum": 1000 },
                        "settings": { "type": "object", "maxProperties": 32 }
                    },
                    "required": ["enabled", "placement"]
                }
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
                "executable": true,
                "homeCards": true,
                "reason": "Declarative plugins contribute appearance, Home layout, settings, and host-rendered remote panels. Isolated-ui and background plugins install disabled; only the user can approve their declared permissions, network origins, and trigger in Settings > Plugins before the runtime starts. API version 3 isolated-ui packages can register host-rendered Home cards and paired-phone panels backed by on-demand commands. Executable drafts may run in a disposable development session after explicit user approval; MCP cannot approve access or change production."
            },
            "plugins": {
                "supported": true,
                "manifest": "luma-extension.json",
                "fields": plugin::FIELDS,
                "settingTypes": ["boolean", "enum", "number", "string"],
                "apiVersions": {
                    "declarative": plugin::API_VERSION,
                    "executable": plugin::EXECUTABLE_API_VERSION,
                    "homeCards": plugin::HOME_API_VERSION
                },
                "homeCards": {
                    "apiVersion": plugin::HOME_API_VERSION,
                    "hostVersion": plugin::HOST_VERSION,
                    "manifest": "Requires minHostVersion, an isolated-ui runtime, unique contributes.commands, and one or more contributes.cards that each reference a declared command.",
                    "projection": "Only enabled, compatible, granted packages with a matching grant version are projected. Rendering Home never starts a runtime.",
                    "dispatch": "Selecting a card invokes customization_plugin_command. The host rechecks the enabled package, declared command, installed version, and grant, starts the isolated-ui runtime on demand, waits for surface.ready, and delivers the command through the bridge.",
                    "remote": "An API version 3 package may also declare remote.commands with host-rendered labels. Only commands declared by the same package and listed in the current panel are accepted. A paired phone action rechecks pairing, the current panel, the command, and the grant, then starts a hidden on-demand runtime without opening or focusing a desktop surface."
                },
                "runtimes": {
                    "supported": ["declarative", "isolated-ui", "background"],
                    "installedDisabled": ["main-page"]
                },
                "executablePermissions": plugin::PERMISSIONS
                    .iter()
                    .map(|permission| permission.label())
                    .collect::<Vec<_>>(),
                "executableNetwork": "An executable package may declare network.get with up to eight public HTTPS origins. The host resolves the name, rejects non-public targets, and pins the request to the resolved address.",
                "importFormats": ["luma-extension.zip", "manifest plus text/base64 files"],
                "limits": plugin::limits(),
                "review": "MCP installs plugins disabled. Only the user can preview and enable a declarative plugin or approve an executable runtime grant; both require an explicit user action in Luma. Settings staged by MCP also require user Keep.",
                "development": {
                    "supported": true,
                    "disposable": true,
                    "scope": "Executable isolated-ui and background packages only. Declarative packages use luma_plugin_stage and a user preview.",
                    "submission": "The same manifest plus text/base64 files used by luma_plugin_validate and luma_plugin_stage.",
                    "isolation": "Each session has separate temporary app data, settings, grants, and identity. It never changes the running production profile.",
                    "approval": "The draft starts disabled. Only the user can review the exact permissions, network origins, and trigger in the development window and choose Allow and enable. MCP cannot approve access.",
                    "tools": [
                        "luma_plugin_dev_start",
                        "luma_plugin_dev_reload",
                        "luma_plugin_dev_invoke",
                        "luma_plugin_dev_status",
                        "luma_plugin_dev_stop"
                    ],
                    "commandApi": "luma_plugin_dev_invoke accepts a command declared in contributes.commands. Delivery is checked against the active enabled package and grant, and is separate from the plugin's feature outcome.",
                    "templates": {
                        "declarative": {
                            "uri": DECLARATIVE_TEMPLATE,
                            "apiVersion": plugin::API_VERSION,
                            "runtime": "declarative",
                            "description": "Complete editable appearance and Home package with enum, boolean, and number settings. Use luma_plugin_validate and luma_plugin_stage; the user previews and presses Keep."
                        },
                        "weather": {
                            "uri": WEATHER_TEMPLATE,
                            "apiVersion": plugin::HOME_API_VERSION,
                            "runtime": "isolated-ui",
                            "description": "Complete editable Weather Home card with settings.read, network.get for api.open-meteo.com, and an on-demand openForecast command. Use luma_plugin_dev_start."
                        },
                        "phoneCommand": {
                            "uri": PHONE_COMMAND_TEMPLATE,
                            "apiVersion": plugin::HOME_API_VERSION,
                            "runtime": "isolated-ui",
                            "description": "Complete editable paired-phone command panel with host-rendered playback controls and remote.commands labels. Use luma_plugin_dev_start."
                        },
                        "svelte": {
                            "uri": SVELTE_TEMPLATE,
                            "kind": "source-project",
                            "description": "Standalone Svelte, Tailwind, and TypeScript source with a local SDK. Build it in your coding workspace and submit dist."
                        }
                    }
                }
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
                "instances": "keyed by instance id; max 32",
                "instance": {
                    "type": "string (built-in widget type or plugin type)",
                    "enabled": "boolean",
                    "placement": PLACEMENTS,
                    "visibility": ["always", "available", "playing"],
                    "position": "null for preset, or x/y integers from 0 through 1000",
                    "size": ["small", "medium", "large"],
                    "variant": "style variant such as glass, cinematic, ticker, or minimal",
                    "presence": ["home", "music", "idle"],
                    "order": "integer from 0 through 1000",
                    "settings": "widget-specific settings, max 32 keys"
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
            "pendingPreviews": 1,
            "pluginSubmission": plugin::limits()
        },
        "tools": tool_summaries,
        "resources": [
            "luma://customization/capabilities",
            "luma://customization/guidance",
            plugin::SCHEMA_RESOURCE,
            plugin::EXAMPLE_RESOURCE,
            DECLARATIVE_TEMPLATE,
            WEATHER_TEMPLATE,
            PHONE_COMMAND_TEMPLATE,
            SVELTE_TEMPLATE
        ],
        "review": {
            "keep": "The user must press Keep in Luma. No MCP tool can keep, enable, commit, restore, reset, launch, or install-active.",
            "previewRequiresVisibleLauncher": true,
            "readsWorkInTray": true,
            "agentCanCancelOnlyOwnPreview": true,
            "pluginStage": "Installs disabled only.",
            "pluginSettingsStage": "Requires user Keep.",
            "executableDevelopment": "The user must approve the draft's exact permissions, network origins, and trigger in the disposable development window. MCP cannot approve access.",
            "declarativeReview": "The user must preview the staged declarative plugin and press Keep in Settings > Plugins."
        },
        "unavailable": [
            "Seeing or committing without user Keep.",
            "Approving plugin access, enabling, keeping, disabling, removing, or exporting a plugin in production.",
            "Restoring history or resetting appearance.",
            "Launching apps, changing app paths, or reading account credentials.",
            "Running executable plugin source outside a user-approved disposable development session.",
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
            },
            {
                "uri": DECLARATIVE_TEMPLATE,
                "name": "Luma declarative plugin template",
                "title": "Declarative plugin template",
                "description": "A complete editable API v1 declarative package bundle. Its manifest binds enum, boolean, and number settings to appearance and Home. Validate and stage it; the user previews and presses Keep in Settings > Plugins.",
                "mimeType": "application/json"
            },
            {
                "uri": WEATHER_TEMPLATE,
                "name": "Luma Weather v3 plugin template",
                "title": "Weather v3 plugin template",
                "description": "A complete editable API v3 isolated-ui Weather package from the repository example. It declares an openForecast command, a Home card, settings.read, network.get for api.open-meteo.com, and the on-demand trigger. Run it through luma_plugin_dev_start.",
                "mimeType": "application/json"
            },
            {
                "uri": PHONE_COMMAND_TEMPLATE,
                "name": "Luma phone command plugin template",
                "title": "Phone command plugin template",
                "description": "A complete editable API v3 isolated-ui package with host-rendered paired-phone controls. It declares playback commands, a Home card, remote.commands labels, and playback.control. Run it through luma_plugin_dev_start.",
                "mimeType": "application/json"
            },
            {
                "uri": SVELTE_TEMPLATE,
                "name": "Luma Svelte plugin source template",
                "title": "Svelte plugin source template",
                "description": "Svelte, Tailwind, TypeScript, and Vite project with the Luma SDK. Requires a build workspace. Submit only its built package.",
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
        DECLARATIVE_TEMPLATE | WEATHER_TEMPLATE | PHONE_COMMAND_TEMPLATE | SVELTE_TEMPLATE => Ok((
            "application/json",
            serde_json::to_string_pretty(&template_bundle(uri)?).unwrap_or_else(|_| "{}".into()),
        )),
        _ => Err(format!("Resource not found: {uri}.")),
    }
}

fn template_bundle(uri: &str) -> Result<Value, String> {
    match uri {
        SVELTE_TEMPLATE => Ok(json!({
            "kind": "source-project",
            "name": "Svelte plugin starter",
            "installCommand": "npm install",
            "checkCommand": "npm run check",
            "buildCommand": "npm run build",
            "outputDirectory": "dist",
            "files": [
                { "path": "package.json", "text": include_str!("../../../starters/svelte/package.json").replace("file:../../sdk", "file:./vendor/plugin-sdk") },
                { "path": "luma-extension.json", "text": include_str!("../../../starters/svelte/luma-extension.json") },
                { "path": "vite.config.ts", "text": include_str!("../../../starters/svelte/vite.config.ts") },
                { "path": "svelte.config.js", "text": include_str!("../../../starters/svelte/svelte.config.js") },
                { "path": "tsconfig.json", "text": include_str!("../../../starters/svelte/tsconfig.json") },
                { "path": "src/App.svelte", "text": include_str!("../../../starters/svelte/src/App.svelte") },
                { "path": "src/main.ts", "text": include_str!("../../../starters/svelte/src/main.ts") },
                { "path": "src/style.css", "text": include_str!("../../../starters/svelte/src/style.css") },
                { "path": "ui/index.html", "text": include_str!("../../../starters/svelte/ui/index.html") },
                { "path": "vendor/plugin-sdk/package.json", "text": include_str!("../../../sdk/package.json") },
                { "path": "vendor/plugin-sdk/index.js", "text": include_str!("../../../sdk/index.js") },
                { "path": "vendor/plugin-sdk/index.d.ts", "text": include_str!("../../../sdk/index.d.ts") }
            ],
            "next": [
                "Write these source files to your coding workspace and edit the manifest and UI.",
                "Install dependencies, check the project, and run the build command in that workspace.",
                "Submit dist/luma-extension.json as manifest and the other dist files as package-relative files to luma_plugin_validate.",
                "Start or reload a disposable development session with the built package."
            ]
        })),
        DECLARATIVE_TEMPLATE => Ok(json!({
            "kind": "declarative",
            "name": "Declarative appearance and Home package",
            "description": "A complete editable API v1 declarative package. Its manifest declares enum, boolean, and number settings bound to the theme, Home columns, and the featured Home row. Copy and edit the submission, then call luma_plugin_validate and luma_plugin_stage. The user previews the staged plugin and presses Keep; MCP cannot enable it.",
            "source": plugin::EXAMPLE_RESOURCE,
            "requiresUserReview": true,
            "mcpCanEnable": false,
            "submission": {
                "manifest": plugin::example_manifest(),
                "files": []
            },
            "next": [
                "Call luma_plugin_validate with submission.",
                "Call luma_plugin_stage with the same submission.",
                "Ask the user to open Luma > Settings > Plugins, preview the plugin, and press Keep."
            ]
        })),
        WEATHER_TEMPLATE => Ok(json!({
            "kind": "executable",
            "name": "Weather v3 Home card",
            "description": "A complete editable API v3 isolated-ui Weather package from the repository example. It declares an openForecast command, a Weather Home card, settings.read, network.get for https://api.open-meteo.com, and the on-demand trigger. Start a disposable development session to review and test it.",
            "source": "examples/plugins/com.example.weather",
            "requiresUserReview": true,
            "mcpCanApproveAccess": false,
            "submission": {
                "manifest": template_manifest(WEATHER_MANIFEST),
                "files": [
                    { "path": "ui/index.html", "text": WEATHER_HTML },
                    { "path": "ui/page.js", "text": WEATHER_SCRIPT },
                    { "path": "ui/page.css", "text": WEATHER_STYLE }
                ]
            },
            "next": [
                "Edit the manifest and files in your workspace.",
                "Call luma_plugin_validate with submission.",
                "Call luma_plugin_dev_start with the same submission.",
                "Ask the user to review access in the development window and choose Allow and enable.",
                "Call luma_plugin_dev_invoke with command openForecast, then luma_plugin_dev_status.",
                "Edit, call luma_plugin_dev_reload, and call luma_plugin_dev_stop when finished.",
                "Call luma_plugin_stage with the final submission for the user's production review."
            ]
        })),
        PHONE_COMMAND_TEMPLATE => Ok(json!({
            "kind": "executable",
            "name": "Paired-phone command panel",
            "description": "A complete editable API v3 isolated-ui package with host-rendered paired-phone controls. It declares togglePlayback and nextTrack commands, a Phone controls Home card, remote.commands labels, and playback.control. Start a disposable development session to review and test it.",
            "source": "src-tauri/templates/phone-command",
            "requiresUserReview": true,
            "mcpCanApproveAccess": false,
            "submission": {
                "manifest": template_manifest(PHONE_MANIFEST),
                "files": [
                    { "path": "ui/index.html", "text": PHONE_HTML },
                    { "path": "ui/main.js", "text": PHONE_SCRIPT },
                    { "path": "ui/style.css", "text": PHONE_STYLE }
                ]
            },
            "next": [
                "Edit the manifest and files in your workspace.",
                "Call luma_plugin_validate with submission.",
                "Call luma_plugin_dev_start with the same submission.",
                "Ask the user to review playback.control access in the development window and choose Allow and enable.",
                "Call luma_plugin_dev_invoke with togglePlayback or nextTrack, then luma_plugin_dev_status.",
                "Edit, call luma_plugin_dev_reload, and call luma_plugin_dev_stop when finished.",
                "Call luma_plugin_stage with the final submission for the user's production review."
            ]
        })),
        _ => Err(format!("Template not found: {uri}.")),
    }
}

fn template_manifest(source: &str) -> Value {
    serde_json::from_str(source).unwrap_or_else(|_| json!({}))
}

fn guidance() -> &'static str {
    r#"# Luma authoring guidance

Luma is a Windows TV launcher. This MCP server is the agent-first authoring contract for customization and plugin packages. Discovery belongs to the contract: call `luma_capabilities_get`, read `luma://plugins/schema`, and choose a complete template before writing a package.

## Start here

1. Call `luma_capabilities_get`, `luma_plugins_list`, and `luma_status`.
2. Read `luma://plugins/schema` for the manifest and limit contract.
3. Choose one template bundle:
   - `luma://plugins/templates/declarative` for appearance and Home.
   - `luma://plugins/templates/weather` for an API v3 Home card with network access.
   - `luma://plugins/templates/phone-command` for API v3 paired-phone command controls.
   - `luma://plugins/templates/svelte` for a Svelte, Tailwind, and TypeScript source project. Build it locally and submit only the package in `dist`.
4. Each bundle returns a `submission` object in the exact shape used by `luma_plugin_validate`, `luma_plugin_stage`, and `luma_plugin_dev_start`. Edit it in your workspace, then validate the complete package.

## Customization flow

1. Call `luma_config_get` and keep its `revision`.
2. Build a complete effective configuration, not a patch.
3. Call `luma_candidate_validate` with `expectedRevision` and `config`.
4. If valid, call `luma_candidate_preview` with the same values.
5. Tell the user the change is waiting in Luma and that they must press Keep. Stop there.

## Declarative plugins

1. Start from the declarative template or write `luma-extension.json` with a stable id, `major.minor.patch` version, `apiVersion: 1`, bounded settings, and declared contributions.
2. Call `luma_plugin_validate`.
3. Call `luma_plugin_stage`. It installs disabled.
4. Tell the user to open Luma > Settings > Plugins, preview the plugin, and press Keep. No MCP tool can enable it.

## Executable plugin development

1. Start from the Weather v3 or phone command template, or write an `isolated-ui` package with `apiVersion: 2` (or 3 for Home cards and commands), exact `permissions`, and, for `network.get`, a `network.allow` list.
2. Call `luma_plugin_validate`.
3. Call `luma_plugin_dev_start` with the validated submission. The response includes `sessionId`, `declared` permissions and origins, and a `review` action.
4. Ask the user to review the draft in the Luma Development window: open Settings > Plugins, check the exact permissions, network origins, and trigger, then choose Allow and enable. The session is disposable and has separate app data and grants.
5. Call `luma_plugin_dev_status` to inspect readiness. The draft stays disabled until the user approves it.
6. Call `luma_plugin_dev_invoke` with a command declared in `contributes.commands`. Delivery is not the feature outcome; check status and the development window.
7. Edit and call `luma_plugin_dev_reload` for the same session. A failed validation leaves the last working draft available. A change to requested access requires another user review.
8. Call `luma_plugin_dev_stop` when finished.
9. Call `luma_plugin_stage` with the final package, then ask the user to review the installed package in the production launcher.

## Staging and review

`luma_plugin_stage` returns the installed plugin, `declared` permissions and network origins, `homeCards`, `commands`, `remote` phone contributions, and a `review` object. For a declarative package the user previews and presses Keep. For an executable package the user must review the exact permissions, network origins, and trigger and choose Allow and enable. MCP never approves access, enables, keeps, or executes anything in production.

## Plugin settings

A manifest can declare boolean, enum, number, or string settings with validated defaults. Bind settings to supported fields, for example `theme: {"setting":"theme"}`, `homeColumns: {"setting":"columns"}`, or `home.rows.featured.visible: {"setting":"showFeatured"}`. Stage new values with `luma_plugin_settings_stage`; the user must press Keep. Settings for disabled plugins remain stored and apply again when the plugin is enabled.

## Home cards and phone commands

An isolated-ui package can use `apiVersion: 3` with `minHostVersion`, `contributes.commands`, and one or more `contributes.cards`. Each card references a command declared by the same package. The host renders the card and starts the package only when the user selects it; rendering Home never starts a runtime. The user approves the on-demand grant in Settings > Plugins. The same package can declare `remote.commands` with host-rendered labels for a paired phone; each entry must reference a command declared by the package. Phone actions start a hidden runtime and never open or focus a desktop surface.

## Examples

### Change the theme

Read the current document. Change only `config.theme` to a value listed by `luma_capabilities_get`. Keep every other field unchanged, including the full `home` array and `order`. Validate and preview the complete document.

### Rearrange Home

Read the current document. Reorder `config.home` while keeping exactly one row for each of the seven components. Keep each row's `id`, `component`, `visible`, and `span`. Validate and preview the complete document.

### Change the app order

Read the current document. Reorder `config.order` so every app appears exactly once and `Custom` remains last. Validate and preview the complete document.

### Add a Home card

Read `luma://plugins/templates/weather` or `luma://plugins/templates/phone-command`. Edit the manifest id, name, version, commands, and card. Validate, start a development session, ask the user to approve access, invoke a declared command, then stage the final package.

## Rules

- Preview requires the Luma launcher to be open and visible. If it is in the tray, ask the user to open Luma.
- Keep is a user action in Luma. The MCP tools cannot keep, enable, disable, remove, export, commit, restore, reset, launch, or execute anything in production.
- A preview creates one pending review. If another is pending, ask the user to Keep or Reject it before staging another.
- If a candidate changes a field controlled by an enabled plugin, the preview reports the detached fields and Keep preserves the user's value while detaching that field. Do not describe this as applying the plugin.
- Use `luma_candidate_cancel` only for the preview returned to this grant. Never try to cancel a review created in Luma.
- Revoking or rotating access stops the next request and cancels this grant's pending preview.
- Report validation and revision errors exactly. Re-read the current configuration before retrying after a conflict.
- Executable packages install disabled. The user must approve the exact permissions, network origins, and trigger. API version 3 packages use the on-demand trigger and start only for a selected Home card or paired-phone command.
- Development sessions are disposable. The user's approval applies only to that session; a production package still needs its own review.
- Do not ask for or print tokens, passwords, pairing codes, or account credentials. Luma never returns them through these tools.
"#
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
        dev_starts: usize,
        dev_reloads: usize,
        dev_invokes: Vec<String>,
        dev_statuses: usize,
        dev_stops: Vec<String>,
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
                    dev_starts: 0,
                    dev_reloads: 0,
                    dev_invokes: Vec::new(),
                    dev_statuses: 0,
                    dev_stops: Vec::new(),
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

        fn dev_starts(&self) -> usize {
            self.state.lock().unwrap().dev_starts
        }

        fn dev_reloads(&self) -> usize {
            self.state.lock().unwrap().dev_reloads
        }

        fn dev_invokes(&self) -> Vec<String> {
            self.state.lock().unwrap().dev_invokes.clone()
        }

        fn dev_statuses(&self) -> usize {
            self.state.lock().unwrap().dev_statuses
        }

        fn dev_stops(&self) -> Vec<String> {
            self.state.lock().unwrap().dev_stops.clone()
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
                    home_cards: Vec::new(),
                })
            })
        }

        fn validate_plugin<'a>(&'a self, _submission: Value) -> BackendFuture<'a, Value> {
            Box::pin(async move { Ok(json!({ "valid": true })) })
        }

        fn stage_plugin<'a>(
            &'a self,
            _submission: Value,
        ) -> BackendFuture<'a, plugin::ImportResult> {
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

        fn dev_start<'a>(&'a self, submission: Value) -> BackendFuture<'a, Value> {
            Box::pin(async move {
                self.state.lock().unwrap().dev_starts += 1;
                Ok(json!({
                    "sessionId": "dev-session",
                    "pluginId": submission["manifest"]["id"],
                    "status": "installed_disabled",
                    "enabled": false,
                    "appDataDir": "C:\\Temp\\luma-dev"
                }))
            })
        }

        fn dev_reload<'a>(
            &'a self,
            session_id: String,
            _submission: Value,
        ) -> BackendFuture<'a, Value> {
            Box::pin(async move {
                self.state.lock().unwrap().dev_reloads += 1;
                Ok(json!({
                    "sessionId": session_id,
                    "status": "reloaded",
                    "reviewRequired": false
                }))
            })
        }

        fn dev_invoke<'a>(
            &'a self,
            session_id: String,
            command: String,
        ) -> BackendFuture<'a, Value> {
            Box::pin(async move {
                self.state.lock().unwrap().dev_invokes.push(command.clone());
                Ok(json!({
                    "sessionId": session_id,
                    "command": command,
                    "delivered": true
                }))
            })
        }

        fn dev_status<'a>(&'a self, session_id: String) -> BackendFuture<'a, Value> {
            Box::pin(async move {
                self.state.lock().unwrap().dev_statuses += 1;
                Ok(json!({
                    "sessionId": session_id,
                    "ready": true,
                    "enabled": true,
                    "appDataDir": "C:\\Temp\\luma-dev"
                }))
            })
        }

        fn dev_stop<'a>(&'a self, session_id: String) -> BackendFuture<'a, Value> {
            Box::pin(async move {
                self.state
                    .lock()
                    .unwrap()
                    .dev_stops
                    .push(session_id.clone());
                Ok(json!({
                    "sessionId": session_id,
                    "stopped": true
                }))
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
        assert!(names.contains(&"luma_plugin_dev_start"));
        assert!(names.contains(&"luma_plugin_dev_reload"));
        assert!(names.contains(&"luma_plugin_dev_invoke"));
        assert!(names.contains(&"luma_plugin_dev_status"));
        assert!(names.contains(&"luma_plugin_dev_stop"));
        assert!(!names.iter().any(|name| {
            [
                "keep", "commit", "restore", "reset", "launch", "install", "enable", "remove",
                "export",
            ]
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
        assert!(uris.contains(&DECLARATIVE_TEMPLATE));
        assert!(uris.contains(&WEATHER_TEMPLATE));
        assert!(uris.contains(&PHONE_COMMAND_TEMPLATE));
        assert!(uris.contains(&SVELTE_TEMPLATE));

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
        assert_eq!(
            capabilities["layers"]["plugins"]["runtimes"]["supported"][0],
            "declarative"
        );
        assert_eq!(
            capabilities["layers"]["plugins"]["development"]["templates"]["weather"]["uri"],
            WEATHER_TEMPLATE
        );
        assert_eq!(
            capabilities["layers"]["plugins"]["development"]["templates"]["phoneCommand"]["uri"],
            PHONE_COMMAND_TEMPLATE
        );

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
            "luma_plugin_dev_approve",
            "luma_plugin_dev_enable",
            "luma_plugin_dev_execute",
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

    fn template(uri: &str) -> Value {
        let (mime, text) = read_resource(uri).unwrap();
        assert_eq!(mime, "application/json");
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn svelte_template_includes_a_standalone_build_and_sdk() {
        let source = template(SVELTE_TEMPLATE);
        assert_eq!(source["kind"], "source-project");
        assert_eq!(source["outputDirectory"], "dist");
        assert!(source.get("submission").is_none());
        let files = source["files"].as_array().unwrap();
        let get = |path: &str| {
            files.iter().find(|file| file["path"] == path).unwrap()["text"]
                .as_str()
                .unwrap()
        };
        let package: Value = serde_json::from_str(get("package.json")).unwrap();
        assert_eq!(
            package["dependencies"]["@luma/plugin-sdk"],
            "file:./vendor/plugin-sdk"
        );
        assert!(get("vendor/plugin-sdk/index.js").contains("extension_bridge"));
        assert!(!get("vendor/plugin-sdk/index.d.ts").is_empty());
        assert!(!get("src/App.svelte").is_empty());
    }

    #[test]
    fn template_bundles_are_complete_and_use_the_shared_validator() {
        let declarative = template(DECLARATIVE_TEMPLATE);
        let weather = template(WEATHER_TEMPLATE);
        let phone = template(PHONE_COMMAND_TEMPLATE);

        for bundle in [&declarative, &weather, &phone] {
            let submission = bundle["submission"].clone();
            let package = plugin::validate_submission(&submission).unwrap();
            assert_eq!(
                bundle["kind"],
                if package.manifest.runtime.kind.is_executable() {
                    "executable"
                } else {
                    "declarative"
                }
            );
            assert_eq!(bundle["submission"]["manifest"]["id"], package.manifest.id);
            assert_eq!(
                bundle["submission"]["manifest"]["version"],
                package.manifest.version
            );
            assert!(!bundle["next"].as_array().unwrap().is_empty());
        }

        assert_eq!(declarative["submission"]["manifest"]["apiVersion"], 1);
        assert!(declarative["submission"]["files"]
            .as_array()
            .unwrap()
            .is_empty());

        assert_eq!(weather["submission"]["manifest"]["apiVersion"], 3);
        assert_eq!(
            weather["submission"]["manifest"]["runtime"]["kind"],
            "isolated-ui"
        );
        assert_eq!(
            weather["submission"]["manifest"]["contributes"]["cards"][0]["command"],
            "openForecast"
        );
        assert!(weather["submission"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "ui/index.html"));

        assert_eq!(phone["submission"]["manifest"]["apiVersion"], 3);
        assert_eq!(
            phone["submission"]["manifest"]["remote"]["commands"][0]["command"],
            "togglePlayback"
        );
        assert_eq!(
            phone["submission"]["manifest"]["permissions"][0],
            "playback.control"
        );
    }

    #[test]
    fn staging_review_names_access_contributions_and_the_user_action() {
        let weather = template(WEATHER_TEMPLATE);
        let package = plugin::validate_submission(&weather["submission"]).unwrap();
        let review = staging_review(&package);
        assert_eq!(
            review["permissions"],
            json!(["settings.read", "network.get"])
        );
        assert_eq!(review["network"], json!(["https://api.open-meteo.com"]));
        assert_eq!(review["homeCards"][0]["command"], "openForecast");
        assert_eq!(review["trigger"], "on-demand");
        assert_eq!(review["requiresExplicitAccessApproval"], true);
        assert!(staging_next(&package, false, false).contains("Allow and enable"));

        let declarative = template(DECLARATIVE_TEMPLATE);
        let package = plugin::validate_submission(&declarative["submission"]).unwrap();
        let review = staging_review(&package);
        assert_eq!(review["requiresExplicitAccessApproval"], false);
        assert!(staging_next(&package, false, false).contains("press Keep"));
        assert!(staging_next(&package, true, true).contains("No change was made"));
    }

    #[test]
    fn development_tools_require_executable_packages_and_explicit_review() {
        let backend = TestBackend::new();
        let (port, server) = serve(access(true), backend.clone());
        let weather = template(WEATHER_TEMPLATE);
        let submission = weather["submission"].clone();

        let start = call(port, "luma_plugin_dev_start", submission.clone());
        assert_eq!(start["isError"], false);
        assert_eq!(start["structuredContent"]["sessionId"], "dev-session");
        assert_eq!(
            start["structuredContent"]["declared"]["permissions"],
            json!(["settings.read", "network.get"])
        );
        assert_eq!(
            start["structuredContent"]["declared"]["network"],
            json!(["https://api.open-meteo.com"])
        );
        assert_eq!(
            start["structuredContent"]["declared"]["homeCards"][0]["command"],
            "openForecast"
        );
        assert_eq!(
            start["structuredContent"]["review"]["requiresExplicitAccessApproval"],
            true
        );
        assert_eq!(
            start["structuredContent"]["review"]["productionChanged"],
            false
        );
        let next = start["structuredContent"]["next"].as_str().unwrap();
        assert!(next.contains("Settings > Plugins"));
        assert!(next.contains("MCP cannot approve access"));
        assert!(start["structuredContent"]["appDataDir"].is_null());
        assert_eq!(backend.dev_starts(), 1);

        let mut reload_arguments = submission.clone();
        reload_arguments["sessionId"] = json!("dev-session");
        let reload = call(port, "luma_plugin_dev_reload", reload_arguments);
        assert_eq!(reload["isError"], false);
        assert_eq!(reload["structuredContent"]["sessionId"], "dev-session");
        assert_eq!(backend.dev_reloads(), 1);

        let invoke = call(
            port,
            "luma_plugin_dev_invoke",
            json!({ "sessionId": "dev-session", "command": "openForecast" }),
        );
        assert_eq!(invoke["isError"], false);
        assert_eq!(invoke["structuredContent"]["delivered"], true);
        assert_eq!(backend.dev_invokes(), vec!["openForecast"]);

        let status = call(
            port,
            "luma_plugin_dev_status",
            json!({ "sessionId": "dev-session" }),
        );
        assert_eq!(status["isError"], false);
        assert_eq!(status["structuredContent"]["ready"], true);
        assert!(status["structuredContent"]["appDataDir"].is_null());
        assert_eq!(backend.dev_statuses(), 1);

        let stop = call(
            port,
            "luma_plugin_dev_stop",
            json!({ "sessionId": "dev-session" }),
        );
        assert_eq!(stop["isError"], false);
        assert_eq!(backend.dev_stops(), vec!["dev-session"]);

        let second = call(port, "luma_plugin_dev_start", weather["submission"].clone());
        assert_eq!(second["isError"], false);
        assert_eq!(backend.dev_starts(), 2);

        let declarative = template(DECLARATIVE_TEMPLATE);
        let refused = call(
            port,
            "luma_plugin_dev_start",
            declarative["submission"].clone(),
        );
        assert_eq!(legacy_tool_error(&refused), "plugin_unsupported");
        assert_eq!(backend.dev_starts(), 2);

        let generation = tauri::async_runtime::block_on(server.authorize(TOKEN)).unwrap();
        tauri::async_runtime::block_on(server.rotate()).unwrap();
        assert_eq!(backend.dev_stops(), vec!["dev-session", "dev-session"]);
        let stale = tauri::async_runtime::block_on(call_tool(
            &server,
            generation,
            "luma_plugin_dev_start",
            weather["submission"].clone(),
        ));
        assert_eq!(legacy_tool_error(&stale), "grant_revoked");
        assert_eq!(backend.dev_starts(), 2);
    }

    #[test]
    fn development_arguments_are_strict_and_require_declared_commands() {
        let backend = TestBackend::new();
        let (port, _) = serve(access(true), backend.clone());
        let weather = template(WEATHER_TEMPLATE);

        let missing_session = call(port, "luma_plugin_dev_status", json!({ "sessionId": "" }));
        assert_eq!(legacy_tool_error(&missing_session), "invalid_arguments");

        let unknown_field = call(
            port,
            "luma_plugin_dev_invoke",
            json!({ "sessionId": "dev-session", "command": "openForecast", "extra": true }),
        );
        assert_eq!(legacy_tool_error(&unknown_field), "invalid_arguments");

        let bad_command = call(
            port,
            "luma_plugin_dev_invoke",
            json!({ "sessionId": "dev-session", "command": "Not A Command" }),
        );
        assert_eq!(legacy_tool_error(&bad_command), "invalid_arguments");

        let missing_files = call(
            port,
            "luma_plugin_dev_start",
            json!({
                "manifest": weather["submission"]["manifest"].clone()
            }),
        );
        assert_eq!(missing_files["isError"], true);
        assert_eq!(backend.dev_starts(), 0);
    }

    #[test]
    fn development_command_ids_use_the_shared_local_id_limit() {
        assert!(valid_local_id("openForecast"));
        assert!(valid_local_id("next-track_2"));
        assert!(!valid_local_id(""));
        assert!(!valid_local_id("OpenForecast"));
        assert!(!valid_local_id("open Forecast"));
        assert!(!valid_local_id(&"a".repeat(33)));
        assert_eq!(
            dev_command_schema()["properties"]["command"]["maxLength"],
            plugin::limits()["localId"]
        );
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
            config.widgets.get_mut("spotify").unwrap().placement = placement.into();
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
