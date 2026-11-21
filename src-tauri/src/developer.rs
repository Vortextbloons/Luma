use crate::{
    background,
    customization::{
        self,
        document::Config,
        plugin::{self, Trigger},
        Error, Store,
    },
    extension_spike,
};
use axum::{
    extract::{DefaultBodyLimit, Json, State as AxumState},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    env, fs,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;

const SESSION_ENV: &str = "LUMA_PLUGIN_DEV_SESSION";
const PACKAGE_ENV: &str = "LUMA_PLUGIN_DEV_PACKAGE";
const ID_ENV: &str = "LUMA_PLUGIN_DEV_ID";
const REVIEW_ENV: &str = "LUMA_PLUGIN_DEV_REVIEW";
const HOST_START_TIMEOUT: Duration = Duration::from_secs(30);
const HOST_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const HOST_RELOAD_TIMEOUT: Duration = Duration::from_secs(30);
const HOST_INVOKE_TIMEOUT: Duration = Duration::from_secs(15);
const HOST_STOP_TIMEOUT: Duration = Duration::from_secs(4);
const HOST_STOP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const HOST_POLL: Duration = Duration::from_millis(120);
const MAX_MCP_DIAGNOSTICS: usize = 12;
const MAX_MCP_DIAGNOSTIC_CHARS: usize = 600;

#[derive(Clone)]
pub(crate) struct Profile {
    session: PathBuf,
    initial: Option<PathBuf>,
    expected_id: Option<String>,
    token: String,
    review_initial: bool,
}

impl Profile {
    pub(crate) fn from_environment() -> Result<Option<Self>, String> {
        let mut session = env::var_os(SESSION_ENV);
        let mut package = env::var_os(PACKAGE_ENV);
        let mut id = env::var_os(ID_ENV);
        let mut review_initial = env::var_os(REVIEW_ENV).is_some_and(|value| {
            let value = value.to_string_lossy();
            value == "1" || value.eq_ignore_ascii_case("true")
        });
        let mut args = env::args_os().skip(1);
        while let Some(argument) = args.next() {
            let argument = argument.to_string_lossy();
            if argument == "--developer-session" {
                session = args.next();
            } else if argument == "--developer-package" {
                package = args.next();
            } else if argument == "--developer-id" {
                id = args.next();
            } else if argument == "--developer-review" {
                review_initial = true;
            }
        }
        let Some(session) = session else {
            return Ok(None);
        };
        let session = PathBuf::from(session);
        if !session.is_absolute() {
            return Err(format!("{SESSION_ENV} must be an absolute path."));
        }
        fs::create_dir_all(session.join("snapshots"))
            .map_err(|error| format!("Cannot create the development session: {error}"))?;
        fs::create_dir_all(session.join("app"))
            .map_err(|error| format!("Cannot create the development profile: {error}"))?;
        let expected_id = id
            .map(|value| value.to_string_lossy().to_string())
            .filter(|value| !value.is_empty());
        let package = package.map(PathBuf::from).map(|path| {
            if path.is_absolute() {
                path
            } else {
                session.join(path)
            }
        });
        let mut token = [0u8; 32];
        getrandom::fill(&mut token)
            .map_err(|error| format!("Cannot create the development token: {error}"))?;
        Ok(Some(Self {
            session,
            initial: package,
            expected_id,
            token: hex(&token),
            review_initial,
        }))
    }

    pub(crate) fn app_root(&self) -> PathBuf {
        self.session.join("app")
    }

    fn snapshots(&self) -> PathBuf {
        self.session.join("snapshots")
    }
}

pub(crate) struct State {
    profile: Option<Arc<Profile>>,
    reload_lock: Mutex<()>,
    active: Mutex<Option<PathBuf>>,
    active_id: Mutex<Option<String>>,
    active_name: Mutex<Option<String>>,
    last_error: Mutex<Option<String>>,
    diagnostics: Mutex<VecDeque<String>>,
    command_run: Mutex<Option<CommandRun>>,
    mcp: tokio::sync::Mutex<Option<McpSession>>,
    mcp_process: Mutex<Option<McpProcess>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandRun {
    invocation_id: String,
    plugin_id: String,
    command: String,
    outcome: Option<CommandReport>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandReport {
    invocation_id: String,
    success: bool,
    message: Option<String>,
}

struct McpSession {
    id: String,
    directory: PathBuf,
    plugin_id: String,
    plugin_name: String,
    plugin_version: String,
    port: u16,
    token: String,
    app_data: String,
    snapshot: PathBuf,
    commands: Vec<String>,
    last_error: Option<String>,
}

struct McpProcess {
    id: String,
    child: Child,
    directory: PathBuf,
}

impl State {
    pub(crate) fn new(profile: Option<Profile>) -> Self {
        Self {
            profile: profile.map(Arc::new),
            reload_lock: Mutex::new(()),
            active: Mutex::new(None),
            active_id: Mutex::new(None),
            active_name: Mutex::new(None),
            last_error: Mutex::new(None),
            diagnostics: Mutex::new(VecDeque::new()),
            command_run: Mutex::new(None),
            mcp: tokio::sync::Mutex::new(None),
            mcp_process: Mutex::new(None),
        }
    }

    fn command_run(&self) -> std::sync::MutexGuard<'_, Option<CommandRun>> {
        self.command_run
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn profile(&self) -> Option<Arc<Profile>> {
        self.profile.clone()
    }

    fn active(&self) -> Option<PathBuf> {
        self.active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn record(&self, message: &str) {
        let mut diagnostics = self
            .diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if diagnostics.back().is_some_and(|last| last == message) {
            return;
        }
        if diagnostics.len() >= MAX_MCP_DIAGNOSTICS {
            diagnostics.pop_front();
        }
        diagnostics.push_back(message.to_string());
    }

    fn diagnostics(&self) -> Vec<String> {
        self.diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .cloned()
            .collect()
    }
}

pub(crate) fn report_command(
    app: &AppHandle,
    plugin_id: &str,
    args: Option<Value>,
) -> Result<(), Error> {
    let state = app
        .try_state::<State>()
        .filter(|state| state.profile().is_some())
        .ok_or_else(|| {
            Error::new("denied", "Command reports require a disposable development session.")
        })?;
    let mut report: CommandReport = serde_json::from_value(args.unwrap_or(Value::Null))
        .map_err(|_| Error::new("invalid", "Provide invocationId, success, and an optional message."))?;
    let mut current = state.command_run();
    let run = current
        .as_mut()
        .filter(|run| run.plugin_id == plugin_id && run.invocation_id == report.invocation_id)
        .ok_or_else(|| Error::new("stale", "This command is no longer the active invocation."))?;
    if run.outcome.is_some() {
        return Err(Error::new("stale", "This command already has an outcome."));
    }
    report.message = report.message.map(clamp);
    run.outcome = Some(report);
    Ok(())
}

pub(crate) fn active(app: &AppHandle) -> bool {
    app.try_state::<State>()
        .is_some_and(|state| state.profile().is_some())
}

pub(crate) fn log(app: &AppHandle, message: impl ToString) {
    if active(app) {
        let message = bounded(message);
        eprintln!("[luma-dev] {message}");
        if let Some(state) = app.try_state::<State>() {
            state.record(&message);
        }
    }
}

pub(crate) fn runtime_event(app: &AppHandle, plugin_id: &str, state: &str, detail: impl ToString) {
    log(
        app,
        format!("{plugin_id} runtime {state}: {}", detail.to_string()),
    );
}

pub(crate) fn bridge_error(
    app: &AppHandle,
    plugin_id: &str,
    operation: &str,
    message: impl ToString,
) {
    log(
        app,
        format!("{plugin_id} bridge {operation}: {}", message.to_string()),
    );
}

fn bounded(message: impl ToString) -> String {
    message
        .to_string()
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(240)
        .collect()
}

fn clamp(message: impl ToString) -> String {
    clean(message, MAX_MCP_DIAGNOSTIC_CHARS)
}

fn clean(message: impl ToString, limit: usize) -> String {
    message
        .to_string()
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(limit)
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_command(command: &str) -> bool {
    let mut characters = command.chars();
    matches!(characters.next(), Some(character) if character.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
        && command.len() <= 32
}

fn session_command(command: &str) -> Result<&str, Error> {
    let command = command.trim();
    if valid_command(command) {
        Ok(command)
    } else {
        Err(Error::new(
            "plugin_command",
            "The command must be a declared plugin command id.",
        ))
    }
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<State>();
    let Some(profile) = state.profile() else {
        return Ok(());
    };
    let store = app.state::<Store>();
    if store.read().map_err(|error| error.message)?.is_none() {
        store
            .initialize(Config::default())
            .map_err(|error| error.message)?;
    }
    let mut message = None;
    if let Some(path) = profile.initial.clone() {
        match load_path(app, &path) {
            Ok(value) => {
                message = value
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            Err(error) => {
                let text = match &error.path {
                    Some(path) => format!("{path}: {}", error.message),
                    None => error.message,
                };
                set_error(app, text.clone());
                eprintln!("[luma-dev] {text}");
                message = Some(text);
            }
        }
    }
    start_api(app.clone(), profile.clone());
    present(app.clone(), message);
    Ok(())
}

pub(crate) fn cleanup(app: &AppHandle) {
    if let Some(state) = app.try_state::<State>() {
        if let Some(profile) = state.profile() {
            let _ = fs::remove_file(profile.session.join("host.json"));
        }
        if let Some(mut process) = state
            .mcp_process
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = process.child.kill();
            let _ = fs::remove_dir_all(&process.directory);
        }
        if let Ok(mut session) = state.mcp.try_lock() {
            if let Some(session) = session.take() {
                let _ = fs::remove_dir_all(&session.directory);
            }
        }
    }
}

fn set_error(app: &AppHandle, message: impl ToString) {
    if let Some(state) = app.try_state::<State>() {
        *state
            .last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(message.to_string());
    }
}

fn present(app: AppHandle, message: Option<String>) {
    tauri::async_runtime::spawn(async move {
        let window = match background::main_window(&app).await {
            Ok(window) => window,
            Err(error) => {
                log(
                    &app,
                    format!("The development window could not open: {error}"),
                );
                return;
            }
        };
        let name = app.try_state::<State>().and_then(|state| {
            state
                .active_name
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
        });
        let title = match name.as_deref() {
            Some(name) => format!("Luma Development · {name}"),
            None => "Luma Development".to_string(),
        };
        let _ = window.set_title(&title);
        let _ = window.show();
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        let _ = window.set_focus();
        background::publish_visibility(&window, true);
        extension_spike::launcher_presented(&app);
        if let Some(message) = message {
            crate::customization::plugin::notice(&app, message);
        }
        if let Some(plugin_id) = app
            .try_state::<State>()
            .and_then(|state| state.active_id.lock().ok().and_then(|id| id.clone()))
        {
            activate(&app, &plugin_id);
        }
    });
}

fn update_title(app: &AppHandle) {
    let name = app.try_state::<State>().and_then(|state| {
        state
            .active_name
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    });
    if let Some(window) = app.get_webview_window("main") {
        let title = match name.as_deref() {
            Some(name) => format!("Luma Development · {name}"),
            None => "Luma Development".to_string(),
        };
        let _ = window.set_title(&title);
    }
}

fn activate_if_present(app: &AppHandle, plugin_id: &str) {
    if app.get_webview_window("main").is_some() {
        activate(app, plugin_id);
    }
}

fn activate(app: &AppHandle, plugin_id: &str) {
    let record = plugin::state_for_app(app).ok().and_then(|state| {
        state
            .plugins
            .into_iter()
            .find(|plugin| plugin.id == plugin_id)
    });
    let Some(record) = record else {
        return;
    };
    if !record.enabled {
        return;
    }
    let outcome = if record.runtime == "isolated-ui" {
        extension_spike::open(app, plugin_id)
    } else {
        extension_spike::refresh(app, plugin_id);
        Ok(())
    };
    if let Err(error) = outcome {
        log(app, format!("{plugin_id} runtime: {error}"));
    }
}

fn load_path(app: &AppHandle, path: &Path) -> Result<Value, Error> {
    let store = app.state::<Store>();
    let state = app.state::<State>();
    let _reload = state.reload_lock.lock().unwrap_or_else(|error| error.into_inner());
    let profile = state
        .profile()
        .ok_or_else(|| Error::new("developer", "The development profile is not active."))?;
    let package = plugin::read_package_dir(path).map_err(|error| Error {
        path: error.path.or_else(|| Some(path.display().to_string())),
        ..error
    })?;
    if !matches!(
        package.manifest.api_version,
        plugin::EXECUTABLE_API_VERSION | plugin::HOME_API_VERSION
    ) || !package.manifest.runtime.kind.is_executable()
    {
        return Err(Error::new(
            "incompatible",
            "The development host requires an executable isolated-ui or background package.",
        ));
    }
    if let Some(expected) = &profile.expected_id {
        if &package.manifest.id != expected {
            return Err(Error::new(
                "validation",
                format!(
                    "The package id is {}, but this development session expects {expected}.",
                    package.manifest.id
                ),
            )
            .at("id"));
        }
    }
    let active_id = state
        .active_id
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let id = package.manifest.id.clone();
    let identity_changed = active_id.as_deref().is_some_and(|active| active != id);
    let name = package.manifest.name.clone();
    let version = package.manifest.version.clone();
    let kind = package.manifest.runtime.kind;
    let api_version = package.manifest.api_version;
    let permissions: Vec<String> = package
        .manifest
        .permissions
        .iter()
        .map(|permission| permission.label().to_string())
        .collect();
    let network = package
        .manifest
        .network
        .as_ref()
        .map(|network| network.allow.clone())
        .unwrap_or_default();
    let previous_id = active_id.as_deref().filter(|previous| *previous != id);
    if active_id.as_deref() == Some(id.as_str()) {
        extension_spike::invalidate(app, &id);
    }
    let change = match plugin::developer_install(&store, package, "development", previous_id) {
        Ok(change) => change,
        Err(error) => {
            if active_id.as_deref() == Some(id.as_str()) {
                activate_if_present(app, &id);
            }
            return Err(error);
        }
    };
    let mut enabled = change.enabled;
    let mut review_required = change.review_required;
    let mut message = if review_required {
        "The package requested different access. Review it in Settings > Plugins.".to_string()
    } else if enabled {
        format!("Reloaded {name} {version}.")
    } else {
        format!("Reloaded {name} {version}. The package is disabled.")
    };
    if identity_changed {
        message = "The package id changed. Review it in Settings > Plugins.".to_string();
        if let Some(previous_id) = previous_id {
            extension_spike::invalidate(app, previous_id);
        }
    } else if change.first_install && profile.review_initial {
        review_required = true;
        message = format!(
            "Loaded {name} {version}. Review its access in Settings > Plugins to enable it."
        );
    } else if change.first_install {
        let revision = store
            .read()
            .map_err(|error| error)?
            .ok_or_else(|| Error::new("not_initialized", "Customization is not initialized."))?
            .revision;
        let trigger = if api_version == plugin::HOME_API_VERSION {
            Trigger::OnDemand
        } else {
            Trigger::LauncherOpen
        };
        match plugin::command_enable_runtime(&store, &id, revision, permissions, network, trigger) {
            Ok(()) => {
                enabled = true;
                review_required = false;
                message = format!("Loaded {name} {version}.");
            }
            Err(error) => {
                review_required = true;
                message = format!(
                    "Loaded {name} {version}, but access could not be approved: {}",
                    error.message
                );
            }
        }
    }
    let document = store
        .read()
        .map_err(|error| error)?
        .ok_or_else(|| Error::new("not_initialized", "Customization is not initialized."))?;
    customization::publish(app, &document);
    let record = plugin::state_for_app(app)
        .ok()
        .and_then(|state| state.plugins.into_iter().find(|plugin| plugin.id == id));
    if let Some(reason) = record
        .as_ref()
        .and_then(|record| record.execution_reason.as_deref())
    {
        review_required = false;
        message = format!("Loaded {name} {version}, but it cannot run in this Luma: {reason}");
    } else if !enabled
        && record
            .as_ref()
            .is_some_and(|record| record.executable && record.can_enable)
        && !message.contains("Review")
        && !message.contains("could not be approved")
    {
        review_required = true;
        message = format!(
            "{} {name} {version}. Review its access in Settings > Plugins to enable it.",
            if change.first_install {
                "Loaded"
            } else {
                "Reloaded"
            }
        );
    }
    *state
        .active
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(path.to_path_buf());
    *state
        .active_id
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(id.clone());
    *state
        .active_name
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(name.clone());
    *state
        .last_error
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = None;
    state.command_run().take();
    activate_if_present(app, &id);
    update_title(app);
    log(
        app,
        format!(
            "{id} {version} {} ({})",
            if enabled { "enabled" } else { "disabled" },
            kind.label()
        ),
    );
    Ok(json!({
        "plugin": record.map(|record| json!({
            "id": record.id,
            "name": record.name,
            "version": record.version,
            "runtime": record.runtime,
            "enabled": record.enabled,
            "reviewRequired": review_required && !enabled,
            "executionReason": record.execution_reason,
            "canEnable": record.can_enable,
            "grantedPermissions": record.granted_permissions,
            "grantedNetwork": record.granted_network,
            "runtimeStatus": record.runtime_status,
        })),
        "message": message,
        "reviewRequired": review_required && !enabled,
        "enabled": enabled,
    }))
}

fn snapshot_path(profile: &Profile, value: &str) -> Result<PathBuf, String> {
    let path = fs::canonicalize(value)
        .map_err(|error| format!("Cannot read the package snapshot: {error}"))?;
    let root = fs::canonicalize(profile.snapshots())
        .map_err(|error| format!("Cannot read the session snapshots: {error}"))?;
    if !path.starts_with(&root) || !path.is_dir() {
        return Err("The package snapshot must be inside this development session.".into());
    }
    Ok(path)
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {token}"))
}

fn reply(status: StatusCode, value: Value) -> Response {
    (status, Json(value)).into_response()
}

fn unauthorized() -> Response {
    reply(
        StatusCode::UNAUTHORIZED,
        json!({
            "ok": false,
            "diagnostics": [{ "code": "denied", "message": "The development session token is invalid." }]
        }),
    )
}

fn diagnostic(error: &Error) -> Value {
    json!({
        "code": error.code,
        "message": error.message,
        "path": error.path,
    })
}

fn start_api(app: AppHandle, profile: Arc<Profile>) {
    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
            Ok(listener) => listener,
            Err(error) => {
                log(
                    &app,
                    format!("The development control server could not start: {error}"),
                );
                app.exit(1);
                return;
            }
        };
        let port = match listener.local_addr() {
            Ok(address) => address.port(),
            Err(error) => {
                log(
                    &app,
                    format!("The development control server has no address: {error}"),
                );
                app.exit(1);
                return;
            }
        };
        let app_data = app
            .path()
            .app_data_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let active_id = app
            .try_state::<State>()
            .and_then(|state| state.active_id.lock().ok().and_then(|id| id.clone()));
        let host = json!({
            "pid": std::process::id(),
            "port": port,
            "token": profile.token.clone(),
            "pluginId": active_id,
            "session": profile.session.display().to_string(),
            "appDataDir": app_data,
        });
        let temporary = profile.session.join("host.json.tmp");
        if let Err(error) = fs::write(&temporary, host.to_string()) {
            log(
                &app,
                format!("The development session handshake could not be written: {error}"),
            );
            app.exit(1);
            return;
        }
        if let Err(error) = fs::rename(&temporary, profile.session.join("host.json")) {
            log(
                &app,
                format!("The development session handshake could not be published: {error}"),
            );
            app.exit(1);
            return;
        }
        log(
            &app,
            format!(
                "Session on 127.0.0.1:{port}; app data {}",
                profile.app_root().display()
            ),
        );
        let router = Router::new()
            .route("/status", get(status))
            .route("/reload", post(reload))
            .route("/invoke", post(invoke))
            .route("/devtools", post(devtools))
            .route("/stop", post(stop))
            .layer(DefaultBodyLimit::max(8 * 1024))
            .with_state(app.clone());
        if let Err(error) = axum::serve(listener, router).await {
            log(
                &app,
                format!("The development control server stopped: {error}"),
            );
        }
    });
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReloadRequest {
    path: String,
}

async fn status(AxumState(app): AxumState<AppHandle>, headers: HeaderMap) -> Response {
    let Some(state) = app.try_state::<State>() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    let Some(profile) = state.profile() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    if !authorized(&headers, &profile.token) {
        return unauthorized();
    }
    let active_id = state
        .active_id
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let record = active_id.as_ref().and_then(|id| {
        plugin::state_for_app(&app)
            .ok()
            .and_then(|state| state.plugins.into_iter().find(|plugin| &plugin.id == id))
    });
    let review_required = record.as_ref().is_some_and(|record| {
        !record.enabled && record.execution_reason.is_none() && record.can_enable
    });
    let last_error = state
        .last_error
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .map(clamp);
    reply(
        StatusCode::OK,
        json!({
            "ok": true,
            "session": profile.session.display().to_string(),
            "appDataDir": app.path().app_data_dir().map(|path| path.display().to_string()).unwrap_or_default(),
            "plugin": record.map(|record| json!({
                "id": record.id,
                "name": record.name,
                "version": record.version,
                "runtime": record.runtime,
                "enabled": record.enabled,
                "reviewRequired": review_required,
                "disabledReason": record.disabled_reason,
                "executionReason": record.execution_reason,
                "canEnable": record.can_enable,
                "grantedPermissions": record.granted_permissions,
                "grantedNetwork": record.granted_network,
                "runtimeStatus": record.runtime_status,
            })),
            "activeSnapshot": state.active().map(|path| path.display().to_string()),
            "lastError": last_error,
            "diagnostics": state.diagnostics(),
            "commandRun": state.command_run().as_ref(),
        }),
    )
}

async fn reload(
    AxumState(app): AxumState<AppHandle>,
    headers: HeaderMap,
    Json(request): Json<ReloadRequest>,
) -> Response {
    let Some(state) = app.try_state::<State>() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    let Some(profile) = state.profile() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    if !authorized(&headers, &profile.token) {
        return unauthorized();
    }
    let path = match snapshot_path(&profile, &request.path) {
        Ok(path) => path,
        Err(error) => {
            return reply(
                StatusCode::BAD_REQUEST,
                json!({
                    "ok": false,
                    "diagnostics": [{ "code": "validation", "message": error }]
                }),
            )
        }
    };
    let handle = app.clone();
    let outcome =
        tauri::async_runtime::spawn_blocking(move || load_path(&handle, &path)).await;
    match outcome {
        Ok(Ok(value)) => reply(StatusCode::OK, json!({ "ok": true, "reload": value })),
        Ok(Err(error)) => {
            set_error(&app, &error.message);
            log(
                &app,
                format!(
                    "Reload failed{}: {}",
                    error
                        .path
                        .as_ref()
                        .map(|path| format!(" at {path}"))
                        .unwrap_or_default(),
                    error.message
                ),
            );
            reply(
                StatusCode::BAD_REQUEST,
                json!({ "ok": false, "diagnostics": [diagnostic(&error)] }),
            )
        }
        Err(error) => reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "ok": false,
                "diagnostics": [{ "code": "runtime", "message": error.to_string() }]
            }),
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InvokeRequest {
    command: String,
}

async fn invoke(
    AxumState(app): AxumState<AppHandle>,
    headers: HeaderMap,
    Json(request): Json<InvokeRequest>,
) -> Response {
    let Some(state) = app.try_state::<State>() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    let Some(profile) = state.profile() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    if !authorized(&headers, &profile.token) {
        return unauthorized();
    }
    let plugin_id = state
        .active_id
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let Some(plugin_id) = plugin_id else {
        return reply(
            StatusCode::BAD_REQUEST,
            json!({
                "ok": false,
                "diagnostics": [{
                    "code": "plugin_missing",
                    "message": "No plugin is loaded in this development session."
                }]
            }),
        );
    };
    let command = request.command.trim();
    if !valid_command(command) {
        return reply(
            StatusCode::BAD_REQUEST,
            json!({
                "ok": false,
                "diagnostics": [{
                    "code": "plugin_command",
                    "message": "The command must be a declared plugin command id."
                }]
            }),
        );
    }
    let invocation_id = match random_hex(16) {
        Ok(id) => id,
        Err(error) => {
            return reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "ok": false, "diagnostics": [diagnostic(&error)] }),
            );
        }
    };
    *state.command_run() = Some(CommandRun {
        invocation_id: invocation_id.clone(),
        plugin_id: plugin_id.clone(),
        command: command.to_string(),
        outcome: None,
    });
    match extension_spike::dispatch_command(&app, &plugin_id, command, Some(invocation_id.clone()))
        .await
    {
        Ok(()) => {
            let runtime_status = plugin::state_for_app(&app).ok().and_then(|state| {
                state
                    .plugins
                    .into_iter()
                    .find(|plugin| plugin.id == plugin_id)
                    .and_then(|record| record.runtime_status)
            });
            reply(
                StatusCode::OK,
                json!({
                    "ok": true,
                    "delivered": true,
                    "completed": false,
                    "invocationId": invocation_id,
                    "pluginId": plugin_id,
                    "command": command,
                    "runtimeStatus": runtime_status,
                }),
            )
        }
        Err(error) => {
            let mut run = state.command_run();
            if run.as_ref().is_some_and(|run| run.invocation_id == invocation_id) {
                run.take();
            }
            reply(StatusCode::BAD_REQUEST, json!({
                "ok": false,
                "diagnostics": [{
                    "code": error.code,
                    "message": clamp(error.message),
                }]
            }))
        }
    }
}

async fn devtools(AxumState(app): AxumState<AppHandle>, headers: HeaderMap) -> Response {
    let Some(state) = app.try_state::<State>() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    let Some(profile) = state.profile() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    if !authorized(&headers, &profile.token) {
        return unauthorized();
    }
    match extension_spike::open_devtools(&app) {
        Ok(()) => reply(StatusCode::OK, json!({ "ok": true })),
        Err(error) => reply(
            StatusCode::BAD_REQUEST,
            json!({ "ok": false, "diagnostics": [{ "code": "runtime", "message": error }] }),
        ),
    }
}

async fn stop(AxumState(app): AxumState<AppHandle>, headers: HeaderMap) -> Response {
    let Some(state) = app.try_state::<State>() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    let Some(profile) = state.profile() else {
        return reply(StatusCode::NOT_FOUND, json!({ "ok": false }));
    };
    if !authorized(&headers, &profile.token) {
        return unauthorized();
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        handle.exit(0);
    });
    reply(StatusCode::OK, json!({ "ok": true }))
}

fn mcp_state(app: &AppHandle) -> Result<tauri::State<'_, State>, Error> {
    app.try_state::<State>()
        .ok_or_else(|| Error::new("storage", "Developer session state is unavailable."))
}

fn random_hex(bytes: usize) -> Result<String, Error> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|error| Error::new("storage", error))?;
    Ok(hex(&buffer))
}

fn mcp_session_root() -> Result<PathBuf, Error> {
    Ok(env::temp_dir()
        .join("luma-plugin-dev")
        .join(format!("mcp-{}", random_hex(10)?)))
}

fn mcp_snapshot_path(directory: &Path) -> Result<PathBuf, Error> {
    Ok(directory.join("snapshots").join(random_hex(8)?))
}

fn ensure_development_package(package: &plugin::PreparedPackage) -> Result<(), Error> {
    if !matches!(
        package.manifest.api_version,
        plugin::EXECUTABLE_API_VERSION | plugin::HOME_API_VERSION
    ) || !package.manifest.runtime.kind.is_executable()
    {
        return Err(Error::new(
            "incompatible",
            "The development host requires an executable isolated-ui or background package.",
        ));
    }
    Ok(())
}

async fn prepare_package(submission: Value) -> Result<plugin::PreparedPackage, Error> {
    tauri::async_runtime::spawn_blocking(move || plugin::validate_submission(&submission))
        .await
        .map_err(|error| Error::new("runtime", error))?
}

fn write_snapshot(package: &plugin::PreparedPackage, target: &Path) -> Result<(), Error> {
    fs::create_dir_all(target).map_err(|error| Error::new("storage", error))?;
    fs::write(target.join("luma-extension.json"), &package.manifest_json)
        .map_err(|error| Error::new("storage", error))?;
    for (path, bytes) in &package.files {
        let file = target.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::new("storage", error))?;
        }
        fs::write(file, bytes).map_err(|error| Error::new("storage", error))?;
    }
    Ok(())
}

fn spawn_host(directory: &Path, snapshot: &Path) -> Result<Child, Error> {
    let executable = env::current_exe().map_err(|error| {
        Error::new(
            "developer_start",
            format!("Luma could not find its executable: {error}"),
        )
    })?;
    let log = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(directory.join("host.log"))
        .map_err(|error| Error::new("storage", error))?;
    let errors = log
        .try_clone()
        .map_err(|error| Error::new("storage", error))?;
    let mut command = Command::new(executable);
    command
        .arg("--developer-session")
        .arg(directory)
        .arg("--developer-package")
        .arg(snapshot)
        .arg("--developer-review")
        .current_dir(directory)
        .env_remove(SESSION_ENV)
        .env_remove(PACKAGE_ENV)
        .env_remove(ID_ENV)
        .env_remove(REVIEW_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(errors));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn().map_err(|error| {
        Error::new(
            "developer_start",
            format!("The development host could not start: {error}"),
        )
    })
}

fn host_log_tail(directory: &Path) -> Option<String> {
    const LIMIT: u64 = 4096;
    let mut file = fs::File::open(directory.join("host.log")).ok()?;
    let length = file.metadata().ok()?.len();
    if length > LIMIT {
        file.seek(SeekFrom::End(-(LIMIT as i64))).ok()?;
    }
    let mut bytes = Vec::with_capacity(LIMIT as usize);
    file.take(LIMIT).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let tail: String = text
        .chars()
        .rev()
        .take(MAX_MCP_DIAGNOSTIC_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    Some(clamp(tail))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostHandshake {
    pid: u32,
    port: u16,
    token: String,
    #[serde(default)]
    plugin_id: Option<String>,
    session: String,
    #[serde(default)]
    app_data_dir: String,
}

async fn wait_for_handshake(
    state: &State,
    session_id: &str,
    directory: &Path,
) -> Result<HostHandshake, Error> {
    let file = directory.join("host.json");
    let deadline = Instant::now() + HOST_START_TIMEOUT;
    loop {
        match host_process(state, session_id)? {
            HostProcess::Exited(status) => {
                let detail = host_log_tail(directory)
                    .map(|text| format!(" {text}"))
                    .unwrap_or_default();
                return Err(Error::new(
                    "developer_start",
                    format!(
                        "The development host exited with {status} before it was ready.{detail}"
                    ),
                ));
            }
            HostProcess::Missing => {
                return Err(Error::new(
                    "developer_start",
                    "The development host is not running.",
                ));
            }
            HostProcess::Running => {}
        }
        match fs::read(&file) {
            Ok(bytes) => {
                if let Ok(handshake) = serde_json::from_slice::<HostHandshake>(&bytes) {
                    if handshake.port == 0
                        || handshake.token.len() != 64
                        || !handshake
                            .token
                            .chars()
                            .all(|value| value.is_ascii_hexdigit())
                        || Path::new(&handshake.session) != directory
                    {
                        return Err(Error::new(
                            "developer_start",
                            "The development host returned an invalid handshake.",
                        ));
                    }
                    return Ok(handshake);
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::new(
                    "storage",
                    format!("The development session handshake could not be read: {error}"),
                ))
            }
        }
        if Instant::now() >= deadline {
            let detail = host_log_tail(directory)
                .map(|text| format!(" {text}"))
                .unwrap_or_default();
            return Err(Error::new(
                "developer_start",
                format!("The development host did not start in time.{detail}"),
            ));
        }
        tokio::time::sleep(HOST_POLL).await;
    }
}

async fn host_request(
    session: &McpSession,
    method: reqwest::Method,
    route: &str,
    body: Option<Value>,
    timeout: Duration,
) -> Result<Value, Error> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .no_proxy()
        .build()
        .map_err(|error| Error::new("runtime", error))?;
    let url = format!("http://127.0.0.1:{}{route}", session.port);
    let mut request = client.request(method, url).bearer_auth(&session.token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|error| {
        Error::new(
            "developer_session",
            format!("The development host did not respond: {error}"),
        )
    })?;
    let status = response.status();
    let value = response.json::<Value>().await.map_err(|error| {
        Error::new(
            "developer_session",
            format!("The development host returned an invalid response: {error}"),
        )
    })?;
    if !value.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::new(
                "developer_session",
                "The development session credential is no longer valid.",
            ));
        }
        return Err(host_failure(&value));
    }
    Ok(value)
}

fn host_failure(value: &Value) -> Error {
    let diagnostic = value
        .get("diagnostics")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    let code = diagnostic
        .and_then(|item| item.get("code"))
        .and_then(Value::as_str);
    let message = diagnostic
        .and_then(|item| item.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("The development host rejected the request.");
    let mut error = Error::new(host_code(code), clamp(message));
    if let Some(path) = diagnostic
        .and_then(|item| item.get("path"))
        .and_then(Value::as_str)
    {
        error = error.at(clamp(path));
    }
    error
}

fn host_code(code: Option<&str>) -> &'static str {
    match code {
        Some("validation") | Some("invalid") => "validation",
        Some("incompatible") => "incompatible",
        Some("plugin_disabled") => "plugin_disabled",
        Some("plugin_denied") => "plugin_denied",
        Some("plugin_command") => "plugin_command",
        Some("plugin_unsupported") => "plugin_unsupported",
        Some("plugin_missing") | Some("not_found") => "plugin_missing",
        Some("plugin_busy") => "plugin_busy",
        Some("plugin_not_ready") => "plugin_not_ready",
        Some("plugin_command_timeout") => "plugin_command_timeout",
        Some("stale") => "stale",
        Some("denied") => "denied",
        Some("storage") => "storage",
        _ => "developer",
    }
}

enum HostProcess {
    Missing,
    Running,
    Exited(std::process::ExitStatus),
}

fn host_process(state: &State, session_id: &str) -> Result<HostProcess, Error> {
    let mut process = state
        .mcp_process
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(process) = process.as_mut().filter(|process| process.id == session_id) else {
        return Ok(HostProcess::Missing);
    };
    process
        .child
        .try_wait()
        .map(|status| match status {
            Some(status) => HostProcess::Exited(status),
            None => HostProcess::Running,
        })
        .map_err(|error| {
            Error::new(
                "developer_session",
                format!("The development host status could not be read: {error}"),
            )
        })
}

fn process_id(state: &State, session_id: &str) -> Option<u32> {
    let process = state
        .mcp_process
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    process
        .as_ref()
        .filter(|process| process.id == session_id)
        .map(|process| process.child.id())
}

fn process_running(state: &State, session_id: &str) -> Result<bool, Error> {
    Ok(matches!(
        host_process(state, session_id)?,
        HostProcess::Running
    ))
}

fn take_process(state: &State, session_id: &str) -> Option<McpProcess> {
    let mut process = state
        .mcp_process
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if process
        .as_ref()
        .is_some_and(|process| process.id == session_id)
    {
        process.take()
    } else {
        None
    }
}

fn discard_process(state: &State, session_id: &str) {
    if let Some(mut process) = take_process(state, session_id) {
        let _ = process.child.kill();
    }
}

fn session_or_error<'a>(
    sessions: &'a Option<McpSession>,
    session_id: &str,
) -> Result<&'a McpSession, Error> {
    sessions
        .as_ref()
        .filter(|session| session.id == session_id)
        .ok_or_else(|| {
            Error::new(
                "developer_session",
                "That development session is not active. Start one with luma_plugin_dev_start.",
            )
        })
}

async fn remove_session_directory(directory: &Path) -> Result<(), std::io::Error> {
    let mut last_error = None;
    for attempt in 0..10 {
        let path = directory.to_path_buf();
        let outcome = tauri::async_runtime::spawn_blocking(move || fs::remove_dir_all(path)).await;
        match outcome {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(error)) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Ok(Err(error)) => {
                last_error = Some(error);
                if attempt < 9 {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
            Err(error) => {
                return Err(std::io::Error::new(ErrorKind::Other, error.to_string()));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            ErrorKind::Other,
            "The development session files could not be removed.",
        )
    }))
}

pub(crate) async fn mcp_start(app: &AppHandle, submission: Value) -> Result<Value, Error> {
    if active(app) {
        return Err(Error::new(
            "developer_session",
            "Start an MCP development session from the normal Luma instance.",
        ));
    }
    let package = prepare_package(submission).await?;
    ensure_development_package(&package)?;
    let plugin_id = package.manifest.id.clone();
    let plugin_name = package.manifest.name.clone();
    let plugin_version = package.manifest.version.clone();
    let commands = package.manifest.contributes.commands.clone();
    let state = mcp_state(app)?;
    let mut sessions = state.mcp.lock().await;
    if let Some(existing) = sessions.take() {
        if let Some(mut process) = take_process(&state, &existing.id) {
            let _ = process.child.kill();
        }
        let _ = remove_session_directory(&existing.directory).await;
    }
    let stale = state
        .mcp_process
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    if let Some(mut process) = stale {
        let _ = process.child.kill();
        let _ = remove_session_directory(&process.directory).await;
    }
    let id = random_hex(16)?;
    let directory = mcp_session_root()?;
    let snapshots = directory.join("snapshots");
    fs::create_dir_all(&snapshots).map_err(|error| Error::new("storage", error))?;
    let snapshot = snapshots.join("initial");
    let target = snapshot.clone();
    let write =
        tauri::async_runtime::spawn_blocking(move || write_snapshot(&package, &target)).await;
    match write {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            let _ = remove_session_directory(&directory).await;
            return Err(error);
        }
        Err(error) => {
            let _ = remove_session_directory(&directory).await;
            return Err(Error::new("runtime", error));
        }
    }
    let child = match spawn_host(&directory, &snapshot) {
        Ok(child) => child,
        Err(error) => {
            let _ = remove_session_directory(&directory).await;
            return Err(error);
        }
    };
    *state
        .mcp_process
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(McpProcess {
        id: id.clone(),
        child,
        directory: directory.clone(),
    });
    let handshake = match wait_for_handshake(&state, &id, &directory).await {
        Ok(handshake) => handshake,
        Err(error) => {
            discard_process(&state, &id);
            let _ = remove_session_directory(&directory).await;
            return Err(error);
        }
    };
    if process_id(&state, &id) != Some(handshake.pid) {
        discard_process(&state, &id);
        let _ = remove_session_directory(&directory).await;
        return Err(Error::new(
            "developer_start",
            "The development host returned an invalid handshake.",
        ));
    }
    if handshake
        .plugin_id
        .as_deref()
        .is_some_and(|loaded| loaded != plugin_id.as_str())
    {
        discard_process(&state, &id);
        let _ = remove_session_directory(&directory).await;
        return Err(Error::new(
            "developer_start",
            "The development host loaded an unexpected package.",
        ));
    }
    let session = McpSession {
        id: id.clone(),
        directory: directory.clone(),
        plugin_id: plugin_id.clone(),
        plugin_name: plugin_name.clone(),
        plugin_version: plugin_version.clone(),
        port: handshake.port,
        token: handshake.token,
        app_data: handshake.app_data_dir.clone(),
        snapshot,
        commands,
        last_error: None,
    };
    let status = match host_request(
        &session,
        reqwest::Method::GET,
        "/status",
        None,
        HOST_REQUEST_TIMEOUT,
    )
    .await
    {
        Ok(status) => status,
        Err(error) => {
            discard_process(&state, &id);
            let _ = remove_session_directory(&directory).await;
            return Err(error);
        }
    };
    let plugin = status
        .get("plugin")
        .filter(|plugin| !plugin.is_null())
        .cloned();
    let Some(plugin) = plugin else {
        let message = status
            .get("lastError")
            .and_then(Value::as_str)
            .unwrap_or("The development host did not load the package.");
        discard_process(&state, &id);
        let _ = remove_session_directory(&directory).await;
        return Err(Error::new("developer_start", clamp(message)));
    };
    let loaded = plugin.get("id").and_then(Value::as_str);
    if loaded != Some(plugin_id.as_str()) {
        discard_process(&state, &id);
        let _ = remove_session_directory(&directory).await;
        return Err(Error::new(
            "developer_start",
            "The development host loaded an unexpected package.",
        ));
    }
    let enabled = plugin
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let execution_reason = plugin
        .get("executionReason")
        .and_then(Value::as_str)
        .map(clamp);
    let review_required = execution_reason.is_none()
        && plugin
            .get("reviewRequired")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let status = if execution_reason.is_some() {
        "unsupported"
    } else if review_required {
        "waiting_for_user_review"
    } else if enabled {
        "enabled"
    } else {
        "disabled"
    };
    *sessions = Some(session);
    Ok(json!({
        "sessionId": id,
        "pluginId": plugin_id,
        "plugin": plugin,
        "enabled": enabled,
        "reviewRequired": review_required,
        "executionReason": plugin.get("executionReason").cloned(),
        "status": status,
        "developmentWindow": format!("Luma Development · {plugin_name}"),
        "appDataDir": handshake.app_data_dir,
        "message": if let Some(reason) = execution_reason {
            format!("Loaded {plugin_name} {plugin_version}, but it cannot run in this Luma: {reason}")
        } else {
            format!("Loaded {plugin_name} {plugin_version}. Review its access in Settings > Plugins in the Luma Development window to enable it.")
        },
        "next": "Ask the user to review and enable the plugin in the Luma Development window under Settings > Plugins. No MCP tool can approve access."
    }))
}

pub(crate) async fn mcp_reload(
    app: &AppHandle,
    session_id: &str,
    submission: Value,
) -> Result<Value, Error> {
    let state = mcp_state(app)?;
    let mut sessions = state.mcp.lock().await;
    session_or_error(&sessions, session_id)?;
    if !process_running(&state, session_id)? {
        return Err(Error::new(
            "developer_session",
            "The development host is not running. Start a new development session.",
        ));
    }
    let package = match prepare_package(submission).await {
        Ok(package) => package,
        Err(error) => {
            if let Some(session) = sessions.as_mut() {
                session.last_error = Some(clamp(&error.message));
            }
            return Err(error);
        }
    };
    if let Err(error) = ensure_development_package(&package) {
        if let Some(session) = sessions.as_mut() {
            session.last_error = Some(clamp(&error.message));
        }
        return Err(error);
    }
    let plugin_id = package.manifest.id.clone();
    let plugin_name = package.manifest.name.clone();
    let plugin_version = package.manifest.version.clone();
    let commands = package.manifest.contributes.commands.clone();
    let session = session_or_error(&sessions, session_id)?;
    let directory = session.directory.clone();
    let previous = session.snapshot.clone();
    let snapshot = mcp_snapshot_path(&directory)?;
    let target = snapshot.clone();
    let write =
        tauri::async_runtime::spawn_blocking(move || write_snapshot(&package, &target)).await;
    match write {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if let Some(session) = sessions.as_mut() {
                session.last_error = Some(clamp(&error.message));
            }
            let _ = fs::remove_dir_all(&snapshot);
            return Err(error);
        }
        Err(error) => {
            let error = Error::new("runtime", error);
            if let Some(session) = sessions.as_mut() {
                session.last_error = Some(clamp(&error.message));
            }
            let _ = fs::remove_dir_all(&snapshot);
            return Err(error);
        }
    }
    let response = host_request(
        session,
        reqwest::Method::POST,
        "/reload",
        Some(json!({ "path": snapshot.display().to_string() })),
        HOST_RELOAD_TIMEOUT,
    )
    .await;
    match response {
        Ok(value) => {
            let reload = value.get("reload").cloned().unwrap_or_else(|| json!({}));
            let plugin = reload.get("plugin").cloned();
            let loaded = plugin
                .as_ref()
                .and_then(|plugin| plugin.get("id"))
                .and_then(Value::as_str);
            if loaded != Some(plugin_id.as_str()) {
                let _ = fs::remove_dir_all(&snapshot);
                return Err(Error::new(
                    "developer_reload",
                    "The development host loaded an unexpected package.",
                ));
            }
            let Some(session) = sessions.as_mut() else {
                let _ = fs::remove_dir_all(&snapshot);
                return Err(Error::new(
                    "developer_session",
                    "That development session is not active.",
                ));
            };
            session.snapshot = snapshot.clone();
            session.plugin_id = plugin_id.clone();
            session.plugin_name = plugin_name;
            session.plugin_version = plugin_version;
            session.commands = commands;
            session.last_error = None;
            let _ = fs::remove_dir_all(&previous);
            let enabled = reload
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let execution_reason = plugin
                .as_ref()
                .and_then(|plugin| plugin.get("executionReason"))
                .and_then(Value::as_str)
                .map(clamp);
            let review_required = execution_reason.is_none()
                && reload
                    .get("reviewRequired")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let status = if execution_reason.is_some() {
                "unsupported"
            } else if review_required {
                "waiting_for_user_review"
            } else if enabled {
                "reloaded"
            } else {
                "disabled"
            };
            Ok(json!({
                "sessionId": session_id,
                "pluginId": plugin_id,
                "plugin": plugin,
                "enabled": enabled,
                "reviewRequired": review_required,
                "executionReason": execution_reason.clone(),
                "status": status,
                "message": reload.get("message").cloned(),
                "next": if execution_reason.is_some() {
                    "The reloaded package cannot run in this Luma version. Check the plugin's execution reason and update the host or package."
                } else if review_required {
                    "Ask the user to review the changed access in Settings > Plugins in the Luma Development window. No MCP tool can approve access."
                } else if enabled {
                    "The reloaded package kept its existing approval and is active."
                } else {
                    "The reloaded package is disabled. The user can review it in Settings > Plugins."
                }
            }))
        }
        Err(error) => {
            if let Some(session) = sessions.as_mut() {
                session.last_error = Some(clamp(&error.message));
            }
            let _ = fs::remove_dir_all(&snapshot);
            Err(error)
        }
    }
}

pub(crate) async fn mcp_invoke(
    app: &AppHandle,
    session_id: &str,
    command: &str,
) -> Result<Value, Error> {
    let state = mcp_state(app)?;
    let sessions = state.mcp.lock().await;
    let session = session_or_error(&sessions, session_id)?;
    if !process_running(&state, session_id)? {
        return Err(Error::new(
            "developer_session",
            "The development host is not running. Start a new development session.",
        ));
    }
    let command = session_command(command)?;
    if !session.commands.iter().any(|declared| declared == command) {
        return Err(Error::new(
            "plugin_command",
            "That package did not declare this command.",
        ));
    }
    let response = host_request(
        session,
        reqwest::Method::POST,
        "/invoke",
        Some(json!({ "command": command })),
        HOST_INVOKE_TIMEOUT,
    )
    .await?;
    Ok(json!({
        "sessionId": session_id,
        "pluginId": session.plugin_id,
        "command": command,
        "delivered": true,
        "completed": false,
        "invocationId": response.get("invocationId").cloned(),
        "runtimeStatus": response.get("runtimeStatus").cloned(),
        "message": "The plugin accepted the command. Delivery does not confirm that the feature finished; check the plugin surface and status."
    }))
}

pub(crate) async fn mcp_status(app: &AppHandle, session_id: &str) -> Result<Value, Error> {
    let state = mcp_state(app)?;
    let sessions = state.mcp.lock().await;
    let session = session_or_error(&sessions, session_id)?;
    if !process_running(&state, session_id)? {
        return Ok(json!({
            "sessionId": session.id,
            "pluginId": session.plugin_id,
            "pluginVersion": session.plugin_version,
            "plugin": null,
            "enabled": false,
            "reviewRequired": false,
            "running": false,
            "state": "exited",
            "lastError": session
                .last_error
                .clone()
                .or_else(|| host_log_tail(&session.directory)),
            "appDataDir": session.app_data.clone(),
            "next": "Start a new development session."
        }));
    }
    match host_request(
        session,
        reqwest::Method::GET,
        "/status",
        None,
        HOST_REQUEST_TIMEOUT,
    )
    .await
    {
        Ok(value) => {
            let plugin = value
                .get("plugin")
                .filter(|plugin| !plugin.is_null())
                .cloned();
            let plugin_id = plugin
                .as_ref()
                .and_then(|plugin| plugin.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(&session.plugin_id);
            let enabled = plugin
                .as_ref()
                .and_then(|plugin| plugin.get("enabled"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let review_required = plugin
                .as_ref()
                .and_then(|plugin| plugin.get("reviewRequired"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let last_error = session
                .last_error
                .clone()
                .or_else(|| value.get("lastError").and_then(Value::as_str).map(clamp));
            Ok(json!({
                "sessionId": session.id,
                "pluginId": plugin_id,
                "running": true,
                "state": "running",
                "plugin": plugin,
                "enabled": enabled,
                "reviewRequired": review_required,
                "lastError": last_error,
                "diagnostics": value.get("diagnostics").cloned().unwrap_or_else(|| json!([])),
                "commandRun": value.get("commandRun").cloned(),
                "appDataDir": value.get("appDataDir").cloned(),
                "developmentWindow": format!("Luma Development · {}", session.plugin_name),
            }))
        }
        Err(error) => Ok(json!({
            "sessionId": session.id,
            "pluginId": session.plugin_id,
            "running": true,
            "state": "unresponsive",
            "lastError": clamp(error.message),
            "appDataDir": session.app_data.clone(),
            "next": "Ask the user to check the Luma Development window, or stop and restart the session."
        })),
    }
}

pub(crate) async fn mcp_stop(app: &AppHandle, session_id: &str) -> Result<Value, Error> {
    let state = mcp_state(app)?;
    let mut sessions = state.mcp.lock().await;
    let session = session_or_error(&sessions, session_id)?;
    let plugin_id = session.plugin_id.clone();
    let _ = host_request(
        session,
        reqwest::Method::POST,
        "/stop",
        None,
        HOST_STOP_REQUEST_TIMEOUT,
    )
    .await;
    let deadline = Instant::now() + HOST_STOP_TIMEOUT;
    let mut running = process_running(&state, session_id)?;
    while running && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
        running = process_running(&state, session_id)?;
    }
    let mut warnings = Vec::new();
    if let Some(mut process) = take_process(&state, session_id) {
        if running {
            if let Err(error) = process.child.kill() {
                let exited = process
                    .child
                    .try_wait()
                    .map(|status| status.is_some())
                    .unwrap_or(false);
                if !exited {
                    warnings.push(clamp(format!(
                        "The development host could not be stopped: {error}"
                    )));
                }
            }
        }
    }
    let Some(session) = sessions.take() else {
        return Err(Error::new(
            "developer_session",
            "That development session is not active.",
        ));
    };
    if let Err(error) = remove_session_directory(&session.directory).await {
        warnings.push(clamp(format!(
            "The development session files could not be removed: {error}"
        )));
    }
    Ok(json!({
        "sessionId": session.id,
        "pluginId": plugin_id,
        "stopped": true,
        "warnings": warnings,
        "message": "The development session is stopped."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "luma-developer-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn executable_submission() -> Value {
        json!({
            "manifest": {
                "id": "com.example.devtest",
                "name": "Dev Test",
                "version": "1.0.0",
                "apiVersion": plugin::EXECUTABLE_API_VERSION,
                "runtime": { "kind": "isolated-ui", "entry": "ui/index.html" },
                "permissions": ["settings.read"]
            },
            "files": [{
                "path": "ui/index.html",
                "text": "<!doctype html><title>Dev</title>"
            }]
        })
    }

    #[test]
    fn snapshot_round_trips_through_the_package_validator() {
        let root = root();
        let package = plugin::validate_submission(&executable_submission()).unwrap();
        let hash = package.hash.clone();
        let target = root.join("snapshots").join("initial");
        write_snapshot(&package, &target).unwrap();
        let loaded = plugin::read_package_dir(&target).unwrap();
        assert_eq!(loaded.manifest.id, "com.example.devtest");
        assert_eq!(loaded.hash, hash);
        assert_eq!(
            loaded.files.get("ui/index.html").unwrap(),
            b"<!doctype html><title>Dev</title>"
        );
        assert_eq!(
            fs::read_to_string(target.join("luma-extension.json")).unwrap(),
            package.manifest_json
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn development_packages_must_be_executable() {
        let submission = json!({
            "manifest": {
                "id": "com.example.declarative",
                "name": "Declarative",
                "version": "1.0.0",
                "apiVersion": plugin::API_VERSION,
                "contributes": { "theme": "ember" }
            },
            "files": []
        });
        let package = plugin::validate_submission(&submission).unwrap();
        assert_eq!(
            ensure_development_package(&package).unwrap_err().code,
            "incompatible"
        );
    }

    #[test]
    fn plugin_commands_are_identifiers_not_paths_or_shell_arguments() {
        assert!(valid_command("openForecast"));
        assert!(valid_command("open-forecast_2"));
        assert!(valid_command("a"));
        assert!(!valid_command(""));
        assert!(!valid_command("OpenForecast"));
        assert!(!valid_command("open forecast"));
        assert!(!valid_command("open/forecast"));
        assert!(!valid_command("..\\forecast"));
        assert!(!valid_command(&"a".repeat(33)));
        assert_eq!(
            session_command("open forecast").unwrap_err().code,
            "plugin_command"
        );
    }

    #[test]
    fn host_failures_keep_the_code_and_bound_their_diagnostics() {
        let error = host_failure(&json!({
            "ok": false,
            "diagnostics": [{
                "code": "plugin_disabled",
                "message": "x".repeat(700),
                "path": "p".repeat(700)
            }]
        }));
        assert_eq!(error.code, "plugin_disabled");
        assert_eq!(error.message.chars().count(), MAX_MCP_DIAGNOSTIC_CHARS);
        assert_eq!(
            error.path.unwrap().chars().count(),
            MAX_MCP_DIAGNOSTIC_CHARS
        );
    }

    #[test]
    fn host_log_tails_are_read_without_loading_the_whole_file() {
        let root = root();
        let mut text = "x".repeat(10_000);
        text.push_str("\nfinal development error");
        fs::write(root.join("host.log"), text).unwrap();
        let tail = host_log_tail(&root).unwrap();
        assert!(tail.ends_with("final development error"));
        assert!(tail.chars().count() <= MAX_MCP_DIAGNOSTIC_CHARS);
        let _ = fs::remove_dir_all(root);
    }
}
