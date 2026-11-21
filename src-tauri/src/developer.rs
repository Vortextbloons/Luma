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
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;

const SESSION_ENV: &str = "LUMA_PLUGIN_DEV_SESSION";
const PACKAGE_ENV: &str = "LUMA_PLUGIN_DEV_PACKAGE";
const ID_ENV: &str = "LUMA_PLUGIN_DEV_ID";

#[derive(Clone)]
pub(crate) struct Profile {
    session: PathBuf,
    initial: Option<PathBuf>,
    expected_id: Option<String>,
    token: String,
}

impl Profile {
    pub(crate) fn from_environment() -> Result<Option<Self>, String> {
        let mut session = env::var_os(SESSION_ENV);
        let mut package = env::var_os(PACKAGE_ENV);
        let mut id = env::var_os(ID_ENV);
        let mut args = env::args_os().skip(1);
        while let Some(argument) = args.next() {
            let argument = argument.to_string_lossy();
            if argument == "--developer-session" {
                session = args.next();
            } else if argument == "--developer-package" {
                package = args.next();
            } else if argument == "--developer-id" {
                id = args.next();
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
            token: token.iter().map(|byte| format!("{byte:02x}")).collect(),
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
    review_required: Mutex<bool>,
    last_error: Mutex<Option<String>>,
}

impl State {
    pub(crate) fn new(profile: Option<Profile>) -> Self {
        Self {
            profile: profile.map(Arc::new),
            reload_lock: Mutex::new(()),
            active: Mutex::new(None),
            active_id: Mutex::new(None),
            active_name: Mutex::new(None),
            review_required: Mutex::new(false),
            last_error: Mutex::new(None),
        }
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
}

pub(crate) fn active(app: &AppHandle) -> bool {
    app.try_state::<State>()
        .is_some_and(|state| state.profile().is_some())
}

pub(crate) fn log(app: &AppHandle, message: impl ToString) {
    if active(app) {
        eprintln!("[luma-dev] {}", bounded(message));
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
            Ok(_) => {}
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
        .review_required
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = review_required && !enabled;
    *state
        .last_error
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = None;
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
    let record = plugin::state_for_app(app)
        .ok()
        .and_then(|state| state.plugins.into_iter().find(|plugin| plugin.id == id));
    Ok(json!({
        "plugin": record.map(|record| json!({
            "id": record.id,
            "name": record.name,
            "version": record.version,
            "runtime": record.runtime,
            "enabled": record.enabled,
            "reviewRequired": record.disabled_reason.as_deref().is_some_and(|reason| reason.contains("Review")),
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
    let review = *state
        .review_required
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let review_required = record.as_ref().is_some_and(|record| {
        !record.enabled
            && (review
                || record
                    .disabled_reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("Review")))
    });
    let last_error = state
        .last_error
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
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
                "grantedPermissions": record.granted_permissions,
                "grantedNetwork": record.granted_network,
                "runtimeStatus": record.runtime_status,
            })),
            "activeSnapshot": state.active().map(|path| path.display().to_string()),
            "lastError": last_error,
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
