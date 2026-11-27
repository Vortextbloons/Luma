mod net;

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use crate::{
    customization::{
        plugin::{self, CommandSpec, Grant, Permission, RuntimeKind, RuntimeStatus, Trigger},
        Store,
    },
    integrations::spotify::{auth::Auth, player},
    remote::server::{MusicState, Remote},
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::{
    http::{header, Response, StatusCode},
    ipc::{Invoke, InvokeBody},
    utils::config::BackgroundThrottlingPolicy,
    webview::NewWindowResponse,
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub(crate) const SCHEME: &str = "luma-extension";
const PROTOCOL_POOL: usize = 16;

const BRIDGE: &str = "extension_bridge";
const SETTINGS_EVENT: &str = "luma:settings";
const PLAYBACK_EVENT: &str = "luma:playback";
const MAX_TEXT: usize = 200;
const MAX_ARGS: usize = 128 * 1024;
const MAX_ORIGIN: usize = 128;
const MAX_PATH: usize = 256;
const MAX_QUERY_PAIRS: usize = 16;
const MAX_QUERY_KEY: usize = 64;
const MAX_QUERY_VALUE: usize = 512;
const MAX_QUERY_BYTES: usize = 2048;
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const READY_POLL: Duration = Duration::from_millis(50);
const DELIVERY_TIMEOUT: Duration = Duration::from_millis(1500);
const REMOVE_RETRIES: u32 = 10;
const REMOVE_RETRY_DELAY: Duration = Duration::from_millis(200);
const RESTART_WINDOW: Duration = Duration::from_secs(600);
const MAX_RESTARTS: usize = 3;
const RESTART_DELAY: Duration = Duration::from_millis(500);
const NOT_READY: &str = "The package did not become ready. Luma returned to the launcher.";
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src ipc: http://ipc.localhost; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lifecycle {
    Starting,
    Running,
    Failed,
}

impl Lifecycle {
    fn label(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone)]
struct PendingCommand {
    command: String,
    version: String,
    invocation_id: Option<String>,
}

#[derive(Clone)]
struct Session {
    plugin_id: String,
    version: String,
    label: String,
    token: String,
    slot: usize,
    generation: u64,
    permissions: BTreeSet<Permission>,
    cancel: tokio::sync::watch::Receiver<bool>,
    network_busy: Arc<AtomicBool>,
    control_busy: Arc<AtomicBool>,
    pending: Arc<Mutex<Option<PendingCommand>>>,
    delivered: Arc<tokio::sync::watch::Sender<u64>>,
}

impl Session {
    fn pending_lock(&self) -> std::sync::MutexGuard<'_, Option<PendingCommand>> {
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn revoke(&self) {
        self.pending_lock().take();
        self.delivered
            .send_modify(|value| *value = value.wrapping_add(1));
    }

    fn take_command(&self) -> Option<PendingCommand> {
        let command = self.pending_lock().take();
        if command.is_some() {
            self.delivered
                .send_modify(|value| *value = value.wrapping_add(1));
        }
        command
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Subscription {
    Settings,
    Playback,
}

impl Subscription {
    fn event(self) -> &'static str {
        match self {
            Self::Settings => SETTINGS_EVENT,
            Self::Playback => PLAYBACK_EVENT,
        }
    }
}

#[derive(Default)]
struct Subscriptions {
    settings: Option<String>,
    playback: Option<String>,
}

impl Subscriptions {
    fn slot(&mut self, subscription: Subscription) -> &mut Option<String> {
        match subscription {
            Subscription::Settings => &mut self.settings,
            Subscription::Playback => &mut self.playback,
        }
    }

    fn register(&mut self, subscription: Subscription, payload: &Value) -> bool {
        let serialized = serde_json::to_string(payload).ok();
        *self.slot(subscription) = serialized;
        true
    }

    fn accept(&mut self, subscription: Subscription, payload: &Value) -> bool {
        let serialized = serde_json::to_string(payload).ok();
        let slot = self.slot(subscription);
        if serialized.is_some() && *slot == serialized {
            return false;
        }
        *slot = serialized;
        true
    }
}

struct Runtime {
    session: Session,
    cancel: tokio::sync::watch::Sender<bool>,
    kind: RuntimeKind,
    trigger: Trigger,
    state: Lifecycle,
    reason: Option<String>,
    restarts: u32,
    crashes: Vec<Instant>,
    pending_visible: bool,
    surface_visible: bool,
    subscriptions: Subscriptions,
}

impl Runtime {
    fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            state: self.state.label().into(),
            reason: self.reason.clone(),
            restarts: self.restarts,
        }
    }

    fn live(&self) -> bool {
        self.state != Lifecycle::Failed && !*self.session.cancel.borrow()
    }

    fn accepts(&self, session: &Session) -> bool {
        self.live()
            && self.session.generation == session.generation
            && self.session.token == session.token
    }
}

#[derive(Default)]
pub(crate) struct Supervisor {
    runtimes: Mutex<BTreeMap<String, Runtime>>,
    epochs: Mutex<BTreeMap<String, u64>>,
    dispatching: Mutex<BTreeSet<String>>,
    closed: AtomicBool,
}

impl Supervisor {
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Runtime>> {
        self.runtimes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn begin_dispatch(&self, plugin_id: &str) -> bool {
        self.dispatching
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(plugin_id.to_string())
    }

    fn end_dispatch(&self, plugin_id: &str) {
        self.dispatching
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(plugin_id);
    }

    fn epoch(&self, plugin_id: &str) -> u64 {
        self.epochs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(plugin_id)
            .copied()
            .unwrap_or_default()
    }

    fn bump(&self, plugin_id: &str) {
        let mut epochs = self
            .epochs
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let epoch = epochs.entry(plugin_id.to_string()).or_default();
        *epoch = epoch.wrapping_add(1);
    }
}

fn supervisor_of(app: &AppHandle) -> Option<tauri::State<'_, Supervisor>> {
    app.try_state::<Supervisor>()
}

pub(crate) fn runtime_status(app: &AppHandle, plugin_id: &str) -> Option<RuntimeStatus> {
    let supervisor = supervisor_of(app)?;
    let runtimes = supervisor.lock();
    runtimes.get(plugin_id).map(Runtime::status)
}

#[cfg(debug_assertions)]
pub(crate) fn runtime_origin(app: &AppHandle, plugin_id: &str) -> Option<String> {
    let supervisor = supervisor_of(app)?;
    let runtimes = supervisor.lock();
    runtimes.get(plugin_id).map(|runtime| {
        let token = &runtime.session.token;
        format!("{}/{}", origin(runtime.session.slot), token)
    })
}

pub(crate) fn schemes() -> impl Iterator<Item = String> {
    (0..PROTOCOL_POOL).map(scheme)
}

fn scheme(slot: usize) -> String {
    format!("{SCHEME}.s{slot}")
}

fn token(plugin_id: &str, generation: u64) -> String {
    let hash = Sha256::digest(plugin_id.as_bytes());
    let prefix = hash[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}{generation:x}")
}

fn label(token: &str) -> String {
    format!("ext-{token}")
}

fn host(slot: usize) -> String {
    format!("{}.localhost", scheme(slot))
}

fn origin(slot: usize) -> String {
    format!("http://{}", host(slot))
}

fn allocate_slot(used: &[usize]) -> Option<usize> {
    (0..PROTOCOL_POOL).find(|slot| !used.contains(slot))
}

fn token_path(path: &str, token: &str) -> bool {
    let root = format!("/{token}");
    let prefix = format!("/{token}/");
    path == root || path.starts_with(&prefix)
}

fn permitted_url(url: &tauri::Url, slot: usize, token: &str) -> bool {
    if url.scheme() != "http"
        || url.host_str() != Some(host(slot).as_str())
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    token_path(url.path(), token)
}

fn permitted_request(uri: &tauri::http::Uri, slot: usize, token: &str) -> bool {
    if uri.scheme_str() != Some(scheme(slot).as_str())
        || uri.host() != Some("localhost")
        || uri.port().is_some()
        || uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'))
    {
        return false;
    }
    token_path(uri.path(), token)
}

fn response(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header("content-security-policy", CSP)
        .body(body)
        .unwrap_or_default()
}

fn not_found() -> Response<Vec<u8>> {
    response(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        Vec::new(),
    )
}

fn content_type(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn request_path(path: &str) -> Option<String> {
    let decoded = percent_encoding::percent_decode_str(path)
        .decode_utf8()
        .ok()?;
    Some(decoded.trim_start_matches('/').to_string())
}

fn serve_package(package: &plugin::RuntimePackage, path: &str) -> Response<Vec<u8>> {
    let Some(relative) = request_path(path) else {
        return not_found();
    };
    let Ok(body) = package.read(&relative) else {
        return not_found();
    };
    let content = if relative.is_empty() {
        package.entry.as_str()
    } else {
        relative.as_str()
    };
    response(StatusCode::OK, content_type(content), body)
}

pub(crate) fn protocol<R: tauri::Runtime>(
    context: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let Some(supervisor) = context.app_handle().try_state::<Supervisor>() else {
        return not_found();
    };
    let Some(session) = ({
        let runtimes = supervisor.lock();
        runtimes
            .values()
            .find(|runtime| runtime.session.label == context.webview_label() && runtime.live())
            .map(|runtime| runtime.session.clone())
    }) else {
        return not_found();
    };
    if !permitted_request(request.uri(), session.slot, &session.token) {
        return not_found();
    }
    let prefix = format!("/{}/", session.token);
    let path = request.uri().path();
    let relative = if path == format!("/{}", session.token) {
        ""
    } else if let Some(rest) = path.strip_prefix(&prefix) {
        rest
    } else {
        return not_found();
    };
    let Some(store) = context.app_handle().try_state::<Store>() else {
        return not_found();
    };
    let Ok(package) = plugin::runtime_package(&store, &session.plugin_id) else {
        return not_found();
    };
    serve_package(&package, relative)
}

fn launcher_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main").is_some_and(|window| {
        window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
    })
}

fn show_surface(app: &AppHandle, plugin_id: &str) {
    let Some(supervisor) = supervisor_of(app) else {
        return;
    };
    let label = {
        let mut runtimes = supervisor.lock();
        let Some(runtime) = runtimes.get_mut(plugin_id) else {
            return;
        };
        runtime.pending_visible = false;
        runtime.surface_visible = true;
        runtime.session.label.clone()
    };
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        let _ = window.set_focus();
    }
}

fn focus_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
}

fn destroy_window(app: &AppHandle, label: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.destroy();
        }
    });
}

fn take_runtime(app: &AppHandle, plugin_id: &str) -> Option<Runtime> {
    let supervisor = supervisor_of(app)?;
    let removed = {
        let mut runtimes = supervisor.lock();
        supervisor.bump(plugin_id);
        runtimes.remove(plugin_id)
    };
    if let Some(runtime) = &removed {
        runtime.session.revoke();
        runtime.cancel.send_replace(true);
        plugin::notify_plugins(app);
    }
    removed
}

fn stop_runtime(app: &AppHandle, plugin_id: &str) {
    crate::commands::sports::clear_plugin_badges(app, plugin_id);
    if let Some(runtime) = take_runtime(app, plugin_id) {
        destroy_window(app, runtime.session.label);
    }
}

fn close_surface(app: &AppHandle, plugin_id: &str) {
    let (surface, visible) = supervisor_of(app)
        .map(|supervisor| {
            let runtimes = supervisor.lock();
            runtimes
                .get(plugin_id)
                .map(|runtime| {
                    (
                        runtime.kind.has_surface(),
                        runtime.surface_visible || runtime.pending_visible,
                    )
                })
                .unwrap_or((false, false))
        })
        .unwrap_or((false, false));
    if surface {
        stop_runtime(app, plugin_id);
    }
    if visible {
        focus_main(app);
    }
}

pub(crate) fn invalidate(app: &AppHandle, plugin_id: &str) {
    stop_runtime(app, plugin_id);
}

pub(crate) fn sync(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(store) = app.try_state::<Store>() else {
            return;
        };
        let Ok(granted) = plugin::granted_plugins(&store) else {
            return;
        };
        let running = supervisor_of(&app)
            .map(|supervisor| supervisor.lock().keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for plugin_id in running {
            if !granted.iter().any(|(id, _)| id == &plugin_id) {
                stop_runtime(&app, &plugin_id);
            }
        }
        for (plugin_id, grant) in granted {
            if grant.trigger == Trigger::LauncherOpen && !launcher_visible(&app) {
                continue;
            }
            start(
                &app,
                &plugin_id,
                grant,
                Vec::new(),
                0,
                false,
                false,
                false,
                None,
            );
        }
    });
}

pub(crate) fn refresh(app: &AppHandle, plugin_id: &str) {
    let app = app.clone();
    let plugin_id = plugin_id.to_string();
    tauri::async_runtime::spawn(async move {
        let Some(store) = app.try_state::<Store>() else {
            return;
        };
        let Ok(granted) = plugin::granted_plugins(&store) else {
            return;
        };
        let Some((_, grant)) = granted.into_iter().find(|(id, _)| id == &plugin_id) else {
            stop_runtime(&app, &plugin_id);
            return;
        };
        if grant.trigger == Trigger::LauncherOpen && !launcher_visible(&app) {
            stop_runtime(&app, &plugin_id);
            return;
        }
        start(
            &app,
            &plugin_id,
            grant,
            Vec::new(),
            0,
            false,
            false,
            false,
            None,
        );
    });
}

fn data_directory(app: &AppHandle, plugin_id: &str) -> Option<std::path::PathBuf> {
    let directory = app
        .path()
        .app_data_dir()
        .ok()?
        .join("extension-runtime")
        .join(plugin_id);
    std::fs::create_dir_all(&directory).ok()?;
    Some(directory)
}

async fn destroy_window_owned(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.destroy();
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
}

async fn remove_directory(directory: std::path::PathBuf) -> Result<(), String> {
    for attempt in 0..REMOVE_RETRIES {
        let path = directory.clone();
        let outcome =
            tauri::async_runtime::spawn_blocking(move || match std::fs::remove_dir_all(&path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            })
            .await;
        match outcome {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(error)) if attempt + 1 == REMOVE_RETRIES => {
                return Err(format!(
                    "Luma could not remove the plugin runtime data: {error}"
                ))
            }
            Ok(Err(_)) => tokio::time::sleep(REMOVE_RETRY_DELAY).await,
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

pub(crate) async fn remove_data(app: &AppHandle, plugin_id: &str) -> Result<(), String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("extension-runtime")
        .join(plugin_id);
    if directory.file_name().and_then(|name| name.to_str()) != Some(plugin_id) {
        return Err("The plugin runtime data path is invalid.".into());
    }
    if let Some(runtime) = take_runtime(app, plugin_id) {
        destroy_window_owned(app, &runtime.session.label).await;
    }
    remove_directory(directory).await
}

fn prune_orphaned_data(app: &AppHandle) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let runtime_root = data_dir.join("extension-runtime");
    let plugins_root = data_dir.join("plugins");
    let Ok(entries) = std::fs::read_dir(&runtime_root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !plugins_root.join(&name).is_dir() {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn start(
    app: &AppHandle,
    plugin_id: &str,
    grant: Grant,
    crashes: Vec<Instant>,
    restarts: u32,
    show: bool,
    allow_hidden: bool,
    retry_failed: bool,
    expected_epoch: Option<u64>,
) {
    let Some(store) = app.try_state::<Store>() else {
        return;
    };
    let Some(supervisor) = supervisor_of(app) else {
        return;
    };
    if supervisor.closed.load(Ordering::Acquire) {
        return;
    }
    let epoch = expected_epoch.unwrap_or_else(|| supervisor.epoch(plugin_id));
    let package = match plugin::runtime_package(&store, plugin_id) {
        Ok(package) => package,
        Err(error) => {
            eprintln!("Extension runtime: {}", error.message);
            return;
        }
    };
    if !package.kind.is_executable() {
        return;
    }
    if package.kind == RuntimeKind::IsolatedUi && !show && !allow_hidden {
        return;
    }
    let show = show && package.kind.has_surface();
    let generation = NEXT_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    let token = token(plugin_id, generation);
    let window_label = label(&token);
    let (slot, session) = {
        let mut runtimes = supervisor.lock();
        if supervisor.closed.load(Ordering::Acquire) || supervisor.epoch(plugin_id) != epoch {
            return;
        }
        if let Some(runtime) = runtimes.get(plugin_id) {
            if runtime.state == Lifecycle::Running || runtime.state == Lifecycle::Starting {
                return;
            }
            if runtime.state == Lifecycle::Failed && !retry_failed {
                return;
            }
        }
        let used: Vec<usize> = runtimes
            .iter()
            .filter(|(id, runtime)| {
                id.as_str() != plugin_id
                    && (runtime.state != Lifecycle::Failed
                        || app.get_webview_window(&runtime.session.label).is_some())
            })
            .map(|(_, runtime)| runtime.session.slot)
            .collect();
        let Some(slot) = allocate_slot(&used) else {
            drop(runtimes);
            plugin::notice(
                app,
                "Luma has reached the limit of simultaneous plugin runtimes. Disable another plugin and try again.",
            );
            return;
        };
        let (cancel, receiver) = tokio::sync::watch::channel(false);
        let session = Session {
            plugin_id: plugin_id.to_string(),
            version: package.version.clone(),
            label: window_label.clone(),
            token: token.clone(),
            slot,
            generation,
            permissions: package.permissions.clone(),
            cancel: receiver,
            network_busy: Arc::new(AtomicBool::new(false)),
            control_busy: Arc::new(AtomicBool::new(false)),
            pending: Arc::new(Mutex::new(None)),
            delivered: Arc::new(tokio::sync::watch::channel(0).0),
        };
        if let Some(previous) = runtimes.remove(plugin_id) {
            previous.session.revoke();
            previous.cancel.send_replace(true);
            destroy_window(app, previous.session.label);
        }
        runtimes.insert(
            plugin_id.to_string(),
            Runtime {
                session: session.clone(),
                cancel,
                kind: package.kind,
                trigger: grant.trigger,
                state: Lifecycle::Starting,
                reason: None,
                restarts,
                crashes,
                pending_visible: show,
                surface_visible: false,
                subscriptions: Subscriptions::default(),
            },
        );
        (slot, session)
    };
    crate::developer::runtime_event(
        app,
        plugin_id,
        "starting",
        format!("{} {}", package.name, package.version),
    );
    #[cfg(debug_assertions)]
    {
        use tauri::Emitter;
        let msg = format!("start:{plugin_id}");
        crate::commands::sports::push_log(app, msg.clone());
        let _ = app.emit_to("main", "luma:plugin-log", msg);
    }
    let url = match format!("{}/{token}/{entry}", origin(slot), entry = package.entry)
        .parse::<tauri::Url>()
    {
        Ok(url) => url,
        Err(error) => {
            eprintln!("Extension runtime: invalid origin: {error}");
            return;
        }
    };
    let navigation_slot = slot;
    let navigation_token = token.clone();
    let mut builder =
        WebviewWindowBuilder::new(app, &window_label, WebviewUrl::CustomProtocol(url))
            .title(format!("{} · Luma", package.name))
            .inner_size(640.0, 540.0)
            .min_inner_size(520.0, 420.0)
            .incognito(true)
            .additional_browser_args("--disk-cache-size=33554432")
            .visible(false)
            .focused(false)
            .background_throttling(BackgroundThrottlingPolicy::Disabled)
            .on_navigation(move |url| permitted_url(url, navigation_slot, &navigation_token))
            .on_new_window(|_, _| NewWindowResponse::Deny);
    if let Some(directory) = data_directory(app, plugin_id) {
        builder = builder.data_directory(directory);
    }
    if let (Some(main), Ok(size), Ok(position), Ok(scale)) = (
        app.get_webview_window("main"),
        app.get_webview_window("main")
            .map(|window| window.inner_size())
            .unwrap_or(Ok(tauri::PhysicalSize::new(1280, 720))),
        app.get_webview_window("main")
            .map(|window| window.outer_position())
            .unwrap_or(Ok(tauri::PhysicalPosition::new(0, 0))),
        app.get_webview_window("main")
            .map(|window| window.scale_factor())
            .unwrap_or(Ok(1.0)),
    ) {
        let _ = main;
        builder = builder.position(
            position.x as f64 / scale + (size.width as f64 / scale - 640.0) / 2.0,
            position.y as f64 / scale + (size.height as f64 / scale - 540.0) / 2.0,
        );
    }
    let window = match builder.build() {
        Ok(window) => window,
        Err(error) => {
            eprintln!("Could not open the extension runtime: {error}");
            handle_crash(
                app,
                plugin_id,
                generation,
                "Luma could not open the package runtime.",
            );
            return;
        }
    };
    let current = supervisor.lock().get(plugin_id).is_some_and(|runtime| {
        runtime.session.generation == generation && runtime.session.token == token && runtime.live()
    });
    if !current {
        let _ = window.destroy();
        return;
    }
    plugin::notify_plugins(app);
    let handle = app.clone();
    let event_plugin = plugin_id.to_string();
    let event_label = window_label.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            close_surface(&handle, &event_plugin);
        }
        tauri::WindowEvent::Destroyed => {
            destroyed(&handle, &event_plugin, generation, &event_label);
        }
        _ => {}
    });
    monitor_process(app, &window, plugin_id, generation);
    watchdog(
        app.clone(),
        plugin_id.to_string(),
        generation,
        session.token,
    );
}

fn watchdog(app: AppHandle, plugin_id: String, generation: u64, token: String) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(READY_TIMEOUT).await;
        let ready = supervisor_of(&app).is_some_and(|supervisor| {
            supervisor.lock().get(&plugin_id).is_some_and(|runtime| {
                runtime.session.generation == generation
                    && runtime.session.token == token
                    && runtime.state == Lifecycle::Running
            })
        });
        if ready {
            return;
        }
        handle_crash(&app, &plugin_id, generation, NOT_READY);
    });
}

fn destroyed(app: &AppHandle, plugin_id: &str, generation: u64, label: &str) {
    let Some(supervisor) = supervisor_of(app) else {
        return;
    };
    let current = supervisor.lock().get(plugin_id).is_some_and(|runtime| {
        runtime.session.generation == generation && runtime.session.label == label
    });
    if current {
        handle_crash(
            app,
            plugin_id,
            generation,
            "The package surface closed unexpectedly.",
        );
    }
}

#[cfg(windows)]
fn surface_failed(
    kind: webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PROCESS_FAILED_KIND,
) -> bool {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    matches!(
        kind,
        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
            | COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED
            | COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE
    )
}

#[cfg(windows)]
fn monitor_process(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    plugin_id: &str,
    generation: u64,
) {
    use windows_core::Interface;

    let app = app.clone();
    let plugin_id = plugin_id.to_string();
    let _ = window.with_webview(move |webview| {
        let Ok(core) = (unsafe { webview.controller().CoreWebView2() }) else {
            return;
        };
        let Ok(core) = core.cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_4>()
        else {
            return;
        };
        let handler = webview2_com::ProcessFailedEventHandler::create(Box::new(
            move |_, args| {
                let failed = args
                    .and_then(|args| {
                        let mut kind = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PROCESS_FAILED_KIND::default();
                        unsafe { args.ProcessFailedKind(&mut kind).ok() }.map(|_| kind)
                    })
                    .is_some_and(surface_failed);
                if failed {
                    let app = app.clone();
                    let plugin_id = plugin_id.clone();
                    tauri::async_runtime::spawn(async move {
                        handle_crash(
                            &app,
                            &plugin_id,
                            generation,
                            "The package surface stopped responding.",
                        );
                    });
                }
                Ok(())
            },
        ));
        let mut token = 0;
        let _ = unsafe { core.add_ProcessFailed(&handler, &mut token) };
    });
}

#[cfg(not(windows))]
fn monitor_process(_: &AppHandle, _: &tauri::WebviewWindow, _: &str, _: u64) {}

fn handle_crash(app: &AppHandle, plugin_id: &str, generation: u64, reason: &str) {
    let Some(supervisor) = supervisor_of(app) else {
        return;
    };
    let prepared = {
        let mut runtimes = supervisor.lock();
        let Some(runtime) = runtimes.get_mut(plugin_id) else {
            return;
        };
        if runtime.session.generation != generation || runtime.state == Lifecycle::Failed {
            return;
        }
        runtime.state = Lifecycle::Failed;
        runtime.reason = Some(reason.to_string());
        runtime.session.revoke();
        runtime.cancel.send_replace(true);
        let restore_focus = runtime.surface_visible || runtime.pending_visible;
        runtime.pending_visible = false;
        runtime.surface_visible = false;
        runtime.crashes.push(Instant::now());
        runtime
            .crashes
            .retain(|crash| crash.elapsed() < RESTART_WINDOW);
        runtime.restarts = runtime.restarts.max(runtime.crashes.len() as u32);
        let epoch = supervisor.epoch(plugin_id);
        (
            runtime.session.label.clone(),
            runtime.trigger,
            runtime.crashes.clone(),
            runtime.restarts,
            restore_focus,
            epoch,
        )
    };
    let (runtime_label, trigger, crashes, restarts, restore_focus, epoch) = prepared;
    crate::developer::runtime_event(app, plugin_id, "failed", reason);
    destroy_window(app, runtime_label);
    plugin::notify_plugins(app);
    if restore_focus {
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
        }
        plugin::notice(app, reason);
    }
    if !restart_allowed(&crashes) {
        let message = format!("{reason} Luma stopped restarting it after {MAX_RESTARTS} attempts.");
        if let Some(supervisor) = supervisor_of(app) {
            let mut runtimes = supervisor.lock();
            if let Some(runtime) = runtimes.get_mut(plugin_id) {
                runtime.state = Lifecycle::Failed;
                runtime.reason = Some(message.clone());
                runtime.restarts = MAX_RESTARTS as u32;
            }
        }
        plugin::notify_plugins(app);
        if let Err(error) = plugin::disable_on_crash(app, plugin_id, &message) {
            eprintln!(
                "Extension runtime: could not disable {} after repeated failures: {}: {}",
                plugin_id, error.code, error.message
            );
            if let Some(supervisor) = supervisor_of(app) {
                let mut runtimes = supervisor.lock();
                if let Some(runtime) = runtimes.get_mut(plugin_id) {
                    runtime.reason = Some(format!(
                        "{message} Disabling the package failed: {}",
                        error.message
                    ));
                }
            }
            plugin::notify_plugins(app);
        }
        return;
    }
    let app = app.clone();
    let plugin_id = plugin_id.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(RESTART_DELAY).await;
        let Some(supervisor) = supervisor_of(&app) else {
            return;
        };
        if supervisor.closed.load(Ordering::Acquire) || supervisor.epoch(&plugin_id) != epoch {
            return;
        }
        let Some(store) = app.try_state::<Store>() else {
            return;
        };
        let Ok(granted) = plugin::granted_plugins(&store) else {
            return;
        };
        let Some((_, grant)) = granted.into_iter().find(|(id, _)| id == &plugin_id) else {
            return;
        };
        if grant.trigger != trigger {
            return;
        }
        start(
            &app,
            &plugin_id,
            grant,
            crashes,
            restarts,
            restore_focus,
            trigger == Trigger::OnDemand,
            true,
            Some(epoch),
        );
    });
}

fn restart_allowed(crashes: &[Instant]) -> bool {
    crashes.len() <= MAX_RESTARTS
}

pub(crate) fn open_devtools(app: &AppHandle) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        let Some(supervisor) = supervisor_of(app) else {
            return Err("No plugin runtime is active.".into());
        };
        let label = supervisor
            .lock()
            .values()
            .find(|runtime| runtime.surface_visible)
            .map(|runtime| runtime.session.label.clone())
            .ok_or("Open a plugin surface before opening its DevTools.")?;
        let window = app
            .get_webview_window(&label)
            .ok_or("The plugin surface is not available.")?;
        window.open_devtools();
        Ok(())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = app;
        Err("DevTools are available only in a debug build.".into())
    }
}

pub(crate) fn open(app: &AppHandle, plugin_id: &str) -> Result<(), String> {
    let store = app
        .try_state::<Store>()
        .ok_or("Plugin storage is unavailable.")?;
    let package = plugin::runtime_package(&store, plugin_id).map_err(|error| error.message)?;
    if package.kind != RuntimeKind::IsolatedUi {
        return Err("This plugin does not provide an openable surface.".into());
    }
    let granted = plugin::granted_plugins(&store).map_err(|error| error.message)?;
    let Some((_, grant)) = granted.into_iter().find(|(id, _)| id == plugin_id) else {
        return Err("This plugin does not have an approved runtime grant. Enable it first.".into());
    };
    let existing = supervisor_of(app).and_then(|supervisor| {
        supervisor
            .lock()
            .get(plugin_id)
            .map(|runtime| (runtime.state, runtime.crashes.clone(), runtime.restarts))
    });
    match &existing {
        Some((Lifecycle::Running, _, _)) => {
            show_surface(app, plugin_id);
            return Ok(());
        }
        Some((Lifecycle::Starting, _, _)) => {
            if let Some(supervisor) = supervisor_of(app) {
                if let Some(runtime) = supervisor.lock().get_mut(plugin_id) {
                    runtime.pending_visible = true;
                }
            }
            return Ok(());
        }
        _ => {}
    }
    let (crashes, restarts) = existing
        .map(|(_, crashes, restarts)| (crashes, restarts))
        .unwrap_or_default();
    start(
        app, plugin_id, grant, crashes, restarts, true, false, true, None,
    );
    let state = supervisor_of(app).and_then(|supervisor| {
        supervisor
            .lock()
            .get(plugin_id)
            .map(|runtime| runtime.state)
    });
    match state {
        Some(Lifecycle::Running) => {
            show_surface(app, plugin_id);
            Ok(())
        }
        Some(Lifecycle::Starting) => {
            if let Some(supervisor) = supervisor_of(app) {
                if let Some(runtime) = supervisor.lock().get_mut(plugin_id) {
                    runtime.pending_visible = true;
                }
            }
            Ok(())
        }
        Some(Lifecycle::Failed) => {
            let reason = supervisor_of(app)
                .and_then(|supervisor| {
                    supervisor
                        .lock()
                        .get(plugin_id)
                        .and_then(|runtime| runtime.reason.clone())
                })
                .unwrap_or_else(|| "This plugin is stopped after a runtime failure.".to_string());
            Err(reason)
        }
        None => Err("Luma could not start the plugin runtime.".into()),
    }
}

pub(crate) struct DispatchError {
    pub code: &'static str,
    pub message: String,
}

impl DispatchError {
    fn new(code: &'static str, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

fn stale() -> DispatchError {
    DispatchError::new(
        "stale",
        "The package changed before the command could run. Try again.",
    )
}

fn not_ready(plugin_name: &str) -> DispatchError {
    DispatchError::new(
        "plugin_not_ready",
        format!("{plugin_name} did not start in time. Try again."),
    )
}

fn ensure_current(app: &AppHandle, spec: &CommandSpec, surface: bool) -> Result<(), DispatchError> {
    let store = app
        .try_state::<Store>()
        .ok_or_else(|| DispatchError::new("storage", "Plugin storage is unavailable."))?;
    let current = if surface {
        plugin::command_spec(&store, &spec.plugin_id, &spec.command)
    } else {
        match plugin::remote_invocation(app, &spec.plugin_id, &spec.command) {
            Ok(plugin::RemoteInvocation::Command(current)) => Ok(current),
            Ok(plugin::RemoteInvocation::Playback(_)) => return Err(stale()),
            Err(error) => Err(error),
        }
    }
    .map_err(|error| DispatchError::new(error.code, error.message))?;
    if current.version != spec.version {
        return Err(stale());
    }
    Ok(())
}

async fn wait_ready(
    app: &AppHandle,
    session: &Session,
    plugin_name: &str,
) -> Result<(), DispatchError> {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if *session.cancel.borrow() {
            return Err(stale());
        }
        let state = {
            let Some(supervisor) = supervisor_of(app) else {
                return Err(DispatchError::new(
                    "storage",
                    "The plugin runtime is unavailable.",
                ));
            };
            let runtimes = supervisor.lock();
            match runtimes.get(&session.plugin_id) {
                Some(runtime)
                    if runtime.session.generation == session.generation
                        && runtime.session.token == session.token =>
                {
                    (runtime.state, runtime.reason.clone())
                }
                _ => return Err(stale()),
            }
        };
        match state {
            (Lifecycle::Running, _) => return Ok(()),
            (Lifecycle::Failed, reason) => {
                return Err(DispatchError::new(
                    "plugin_not_ready",
                    reason.unwrap_or_else(|| format!("{plugin_name} could not start. Try again.")),
                ))
            }
            (Lifecycle::Starting, _) => {}
        }
        if Instant::now() >= deadline {
            return Err(not_ready(plugin_name));
        }
        tokio::time::sleep(READY_POLL).await;
    }
}

pub(crate) async fn dispatch_command(
    app: &AppHandle,
    plugin_id: &str,
    command: &str,
    invocation_id: Option<String>,
) -> Result<(), DispatchError> {
    let store = app
        .try_state::<Store>()
        .ok_or_else(|| DispatchError::new("storage", "Plugin storage is unavailable."))?;
    let spec = plugin::command_spec(&store, plugin_id, command)
        .map_err(|error| DispatchError::new(error.code, error.message))?;
    dispatch(app, &spec, true, invocation_id).await
}

pub(crate) async fn dispatch_remote_command(
    app: &AppHandle,
    spec: CommandSpec,
) -> Result<(), DispatchError> {
    dispatch(app, &spec, false, None).await
}

async fn dispatch(
    app: &AppHandle,
    spec: &CommandSpec,
    surface: bool,
    invocation_id: Option<String>,
) -> Result<(), DispatchError> {
    ensure_current(app, spec, surface)?;
    let Some(supervisor) = supervisor_of(app) else {
        return Err(DispatchError::new(
            "storage",
            "The plugin runtime is unavailable.",
        ));
    };
    if !supervisor.begin_dispatch(&spec.plugin_id) {
        return Err(DispatchError::new(
            "plugin_busy",
            "That plugin is already starting a command. Try again when it finishes.",
        ));
    }
    let result = dispatch_command_inner(app, spec, surface, invocation_id).await;
    if let Some(supervisor) = supervisor_of(app) {
        supervisor.end_dispatch(&spec.plugin_id);
    }
    result
}

async fn dispatch_command_inner(
    app: &AppHandle,
    spec: &CommandSpec,
    surface: bool,
    invocation_id: Option<String>,
) -> Result<(), DispatchError> {
    let existing = {
        let Some(supervisor) = supervisor_of(app) else {
            return Err(DispatchError::new(
                "storage",
                "The plugin runtime is unavailable.",
            ));
        };
        let runtimes = supervisor.lock();
        runtimes.get(&spec.plugin_id).map(|runtime| {
            (
                runtime.state,
                runtime.session.clone(),
                runtime.crashes.clone(),
                runtime.restarts,
            )
        })
    };
    let session = match existing {
        Some((Lifecycle::Running, session, _, _)) => {
            if surface {
                show_surface(app, &spec.plugin_id);
            }
            session
        }
        Some((Lifecycle::Starting, session, _, _)) => {
            if surface {
                if let Some(supervisor) = supervisor_of(app) {
                    if let Some(runtime) = supervisor.lock().get_mut(&spec.plugin_id) {
                        runtime.pending_visible = true;
                    }
                }
            }
            wait_ready(app, &session, &spec.plugin_name).await?;
            session
        }
        retry => {
            let (crashes, restarts) = match retry {
                Some((_, _, crashes, restarts)) => (crashes, restarts),
                None => (Vec::new(), 0),
            };
            start(
                app,
                &spec.plugin_id,
                spec.grant.clone(),
                crashes,
                restarts,
                surface,
                !surface,
                true,
                None,
            );
            let session = {
                let Some(supervisor) = supervisor_of(app) else {
                    return Err(DispatchError::new(
                        "storage",
                        "The plugin runtime is unavailable.",
                    ));
                };
                let runtimes = supervisor.lock();
                runtimes
                    .get(&spec.plugin_id)
                    .map(|runtime| runtime.session.clone())
            }
            .ok_or_else(|| {
                DispatchError::new(
                    "plugin_not_ready",
                    format!("{} could not start. Try again.", spec.plugin_name),
                )
            })?;
            wait_ready(app, &session, &spec.plugin_name).await?;
            session
        }
    };
    if !is_current(app, &session) {
        return Err(stale());
    }
    ensure_current(app, spec, surface)?;
    let mut delivered = session.delivered.subscribe();
    let before = *delivered.borrow();
    *session.pending_lock() = Some(PendingCommand {
        command: spec.command.clone(),
        version: spec.version.clone(),
        invocation_id,
    });
    if !is_current(app, &session) {
        session.revoke();
        return Err(stale());
    }
    if let Err(error) = ensure_current(app, spec, surface) {
        session.revoke();
        return Err(error);
    }
    let Some(window) = app.get_webview_window(&session.label) else {
        session.revoke();
        return Err(not_ready(&spec.plugin_name));
    };
    if let Err(error) = window.eval(PULL_COMMAND) {
        let message = format!("{} could not accept the command: {error}", spec.plugin_name);
        handle_crash(app, &session.plugin_id, session.generation, &message);
        return Err(DispatchError::new("plugin_not_ready", message));
    }
    match tokio::time::timeout(DELIVERY_TIMEOUT, delivered.changed()).await {
        Ok(Ok(())) => {}
        _ => {
            let message = format!(
                "{} did not accept the command in time. Try again.",
                spec.plugin_name
            );
            handle_crash(app, &session.plugin_id, session.generation, &message);
            return Err(DispatchError::new("plugin_command_timeout", message));
        }
    }
    if *delivered.borrow() == before {
        session.revoke();
        return Err(DispatchError::new(
            "plugin_command_timeout",
            format!(
                "{} did not accept the command in time. Try again.",
                spec.plugin_name
            ),
        ));
    }
    if !is_current(app, &session) {
        return Err(stale());
    }
    ensure_current(app, spec, surface).map_err(|_| stale())?;
    Ok(())
}

pub(crate) fn autostart(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        prune_orphaned_data(&app);
        let Some(store) = app.try_state::<Store>() else {
            return;
        };
        let Ok(granted) = plugin::granted_plugins(&store) else {
            return;
        };
        for (plugin_id, grant) in granted {
            if !matches!(grant.trigger, Trigger::Startup | Trigger::Always) {
                continue;
            }
            start(
                &app,
                &plugin_id,
                grant,
                Vec::new(),
                0,
                false,
                false,
                false,
                None,
            );
        }
    });
}

pub(crate) fn launcher_presented(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(store) = app.try_state::<Store>() else {
            return;
        };
        let Ok(granted) = plugin::granted_plugins(&store) else {
            return;
        };
        for (plugin_id, grant) in granted {
            if grant.trigger != Trigger::LauncherOpen {
                continue;
            }
            start(
                &app,
                &plugin_id,
                grant,
                Vec::new(),
                0,
                true,
                false,
                false,
                None,
            );
        }
    });
}

pub(crate) fn launcher_hidden(app: &AppHandle) {
    let plugin_ids = supervisor_of(app)
        .map(|supervisor| {
            supervisor
                .lock()
                .values()
                .filter(|runtime| {
                    runtime.trigger == Trigger::LauncherOpen || runtime.kind.has_surface()
                })
                .map(|runtime| runtime.session.plugin_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for plugin_id in plugin_ids {
        stop_runtime(app, &plugin_id);
    }
}

pub(crate) fn hide_surfaces(app: &AppHandle) {
    let plugin_ids = supervisor_of(app)
        .map(|supervisor| {
            supervisor
                .lock()
                .values()
                .filter(|runtime| runtime.kind.has_surface())
                .map(|runtime| runtime.session.plugin_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for plugin_id in plugin_ids {
        stop_runtime(app, &plugin_id);
    }
}

pub(crate) fn dismiss(app: &AppHandle) {
    hide_surfaces(app);
    focus_main(app);
}

pub(crate) fn close(app: &AppHandle) {
    hide_surfaces(app);
    focus_main(app);
}

pub(crate) fn shutdown(app: &AppHandle) {
    let Some(supervisor) = supervisor_of(app) else {
        return;
    };
    supervisor.closed.store(true, Ordering::Release);
    let mut runtimes = supervisor.lock();
    let labels = runtimes
        .values()
        .map(|runtime| runtime.session.label.clone())
        .collect::<Vec<_>>();
    for (plugin_id, runtime) in runtimes.iter() {
        supervisor.bump(plugin_id);
        runtime.session.revoke();
        runtime.cancel.send_replace(true);
    }
    runtimes.clear();
    drop(runtimes);
    for label in labels {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.destroy();
        }
    }
}

pub(crate) fn active(app: &AppHandle) -> bool {
    supervisor_of(app).is_some_and(|supervisor| {
        supervisor.lock().values().any(|runtime| {
            runtime.surface_visible
                && app
                    .get_webview_window(&runtime.session.label)
                    .is_some_and(|window| {
                        window.is_visible().unwrap_or(false)
                            && !window.is_minimized().unwrap_or(false)
                    })
        })
    })
}

const FOCUS_UP: &str = r#"window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp' }));"#;
const FOCUS_DOWN: &str =
    r#"window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));"#;
const FOCUS_LEFT: &str =
    r#"window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));"#;
const FOCUS_RIGHT: &str =
    r#"window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }));"#;
const ACTIVATE: &str = r#"(() => { const target = document.activeElement; if (target instanceof HTMLElement && target.matches('button:not(:disabled)')) target.click(); })();"#;
const PULL_COMMAND: &str = r#"(() => { const internals = window.__TAURI_INTERNALS__; if (!internals || typeof internals.invoke !== 'function') return; internals.invoke('extension_bridge', { op: 'commands.pending' }).then(command => { if (command) window.dispatchEvent(new CustomEvent('luma:command', { detail: command })); }).catch(() => {}); })();"#;

fn relay(command: &str) -> Option<&'static str> {
    match command {
        "up" => Some(FOCUS_UP),
        "down" => Some(FOCUS_DOWN),
        "left" => Some(FOCUS_LEFT),
        "right" => Some(FOCUS_RIGHT),
        "activate" => Some(ACTIVATE),
        _ => None,
    }
}

pub(crate) fn forward(app: &AppHandle, command: &str) {
    let Some(script) = relay(command) else {
        return;
    };
    let Some(supervisor) = supervisor_of(app) else {
        return;
    };
    let label = supervisor
        .lock()
        .values()
        .find(|runtime| runtime.surface_visible)
        .map(|runtime| runtime.session.label.clone());
    if let Some(window) = label.and_then(|label| app.get_webview_window(&label)) {
        let _ = window.eval(script);
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BridgeRequest {
    op: Op,
    #[serde(default)]
    args: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
enum Op {
    #[serde(rename = "identity")]
    Identity,
    #[serde(rename = "settings.read")]
    SettingsRead,
    #[serde(rename = "settings.subscribe")]
    SettingsSubscribe,
    #[serde(rename = "net.get")]
    NetGet,
    #[serde(rename = "playback.read")]
    PlaybackRead,
    #[serde(rename = "playback.subscribe")]
    PlaybackSubscribe,
    #[serde(rename = "playback.toggle")]
    PlaybackToggle,
    #[serde(rename = "playback.next")]
    PlaybackNext,
    #[serde(rename = "playback.previous")]
    PlaybackPrevious,
    #[serde(rename = "surface.ready")]
    SurfaceReady,
    #[serde(rename = "surface.close")]
    SurfaceClose,
    #[serde(rename = "commands.pending")]
    CommandPending,
    #[serde(rename = "development.report")]
    DevelopmentReport,
    #[serde(rename = "sports.active.read")]
    SportsActiveRead,
    #[serde(rename = "sports.badges.publish")]
    SportsBadgesPublish,
}

impl Op {
    fn label(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::SettingsRead => "settings.read",
            Self::SettingsSubscribe => "settings.subscribe",
            Self::NetGet => "net.get",
            Self::PlaybackRead => "playback.read",
            Self::PlaybackSubscribe => "playback.subscribe",
            Self::PlaybackToggle => "playback.toggle",
            Self::PlaybackNext => "playback.next",
            Self::PlaybackPrevious => "playback.previous",
            Self::SurfaceReady => "surface.ready",
            Self::SurfaceClose => "surface.close",
            Self::CommandPending => "commands.pending",
            Self::DevelopmentReport => "development.report",
            Self::SportsActiveRead => "sports.active.read",
            Self::SportsBadgesPublish => "sports.badges.publish",
        }
    }

    fn permission(self) -> Option<Permission> {
        match self {
            Self::SettingsRead | Self::SettingsSubscribe => Some(Permission::SettingsRead),
            Self::NetGet => Some(Permission::NetworkGet),
            Self::PlaybackRead | Self::PlaybackSubscribe => Some(Permission::PlaybackRead),
            Self::PlaybackToggle | Self::PlaybackNext | Self::PlaybackPrevious => {
                Some(Permission::PlaybackControl)
            }
            Self::SportsActiveRead | Self::SportsBadgesPublish => Some(Permission::SportsRead),
            Self::Identity | Self::SurfaceReady | Self::SurfaceClose | Self::CommandPending
            | Self::DevelopmentReport => None,
        }
    }

    fn player_command(self) -> Option<&'static str> {
        match self {
            Self::PlaybackToggle => Some("toggle"),
            Self::PlaybackNext => Some("next"),
            Self::PlaybackPrevious => Some("previous"),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SportsBadgesArgs {
    badges: Vec<crate::commands::sports::SportsBadge>,
    #[serde(default = "default_true", rename = "showOnCards")]
    show_on_cards: bool,
    #[serde(default = "default_true", rename = "showInPlayer")]
    show_in_player: bool,
}

fn default_true() -> bool {
    true
}

fn sanitize_sports_badges(
    plugin_id: &str,
    args: Option<Value>,
) -> Result<Vec<crate::commands::sports::SportsBadge>, Value> {
    let Some(value) = args else {
        return Err(failure("invalid", "Sports badges require an arguments object."));
    };
    let parsed: SportsBadgesArgs = serde_json::from_value(value)
        .map_err(|_| failure("invalid", "Sports badges payload is malformed."))?;
    if parsed.badges.len() > 400 {
        return Err(failure("invalid", "At most 400 sports badges can be published at once."));
    }
    Ok(parsed
        .badges
        .into_iter()
        .filter_map(|mut badge| {
            badge.plugin_id = plugin_id.to_string();
            badge.league = bounded_text(Some(&badge.league))?;
            badge.label = bounded_text(Some(&badge.label))?;
            badge.teams = badge
                .teams
                .into_iter()
                .filter_map(|t| bounded_text(Some(&t)))
                .take(4)
                .collect();
            if badge.teams.len() < 2 {
                return None;
            }
            badge.sides = badge
                .sides
                .into_iter()
                .filter_map(|mut side| {
                    side.team = bounded_text(Some(&side.team))?;
                    side.value = bounded_text(Some(&side.value))?;
                    side.pct = side.pct.map(|p| p.min(100));
                    Some(side)
                })
                .take(4)
                .collect();
            badge.show_on_cards = parsed.show_on_cards;
            badge.show_in_player = parsed.show_in_player;
            Some(badge)
        })
        .collect())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NetArgs {
    origin: String,
    path: String,
    #[serde(default)]
    query: Vec<(String, String)>,
}

fn failure(code: &'static str, message: impl ToString) -> Value {
    json!({ "code": code, "message": message.to_string() })
}

fn log_bridge(app: &AppHandle, plugin_id: &str, operation: &str, error: &Value) {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("The bridge request was rejected.");
    #[cfg(debug_assertions)]
    {
        use tauri::Emitter;
        let msg = format!("{plugin_id}:{operation}: {message}");
        eprintln!("[plugin:{msg}]");
        crate::commands::sports::push_log(app, msg.clone());
        let _ = app.emit_to("main", "luma:plugin-log", msg);
    }
    crate::developer::bridge_error(app, plugin_id, operation, message);
}

fn no_args(args: Option<Value>) -> Result<(), Value> {
    match args {
        None | Some(Value::Null) => Ok(()),
        Some(_) => Err(failure("invalid", "This action does not accept arguments.")),
    }
}

fn bounded_text(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(MAX_TEXT).collect())
}

fn playback(connected: bool, music: Option<&MusicState>) -> Value {
    json!({
        "connected": connected,
        "title": music.and_then(|state| bounded_text(state.title.as_deref())),
        "artist": music.and_then(|state| bounded_text(state.artist.as_deref())),
        "playing": music.is_some_and(|state| state.playing),
    })
}

fn playback_snapshot(app: &AppHandle) -> Value {
    let connected = app
        .try_state::<Auth>()
        .is_some_and(|auth| auth.authenticated());
    let music = app.try_state::<Remote>().and_then(|remote| {
        let receiver = remote.subscribe_music();
        let current = receiver.borrow();
        current.clone()
    });
    playback(connected, music.as_ref())
}

fn subscribed(runtime: &Runtime, subscription: Subscription, permission: Permission) -> bool {
    runtime.live()
        && runtime.session.permissions.contains(&permission)
        && match subscription {
            Subscription::Settings => runtime.subscriptions.settings.is_some(),
            Subscription::Playback => runtime.subscriptions.playback.is_some(),
        }
}

fn subscribed_sessions(
    app: &AppHandle,
    subscription: Subscription,
    permission: Permission,
) -> Vec<Session> {
    let Some(supervisor) = supervisor_of(app) else {
        return Vec::new();
    };
    let runtimes = supervisor.lock();
    runtimes
        .values()
        .filter(|runtime| subscribed(runtime, subscription, permission))
        .map(|runtime| runtime.session.clone())
        .collect()
}

fn register_subscription(
    app: &AppHandle,
    session: &Session,
    subscription: Subscription,
    payload: &Value,
) -> bool {
    let Some(supervisor) = supervisor_of(app) else {
        return false;
    };
    let mut runtimes = supervisor.lock();
    let Some(runtime) = runtimes.get_mut(&session.plugin_id) else {
        return false;
    };
    if !runtime.accepts(session) {
        return false;
    }
    runtime.subscriptions.register(subscription, payload);
    true
}

fn push_subscription(
    app: &AppHandle,
    session: &Session,
    subscription: Subscription,
    payload: &Value,
) -> bool {
    let Some(supervisor) = supervisor_of(app) else {
        return false;
    };
    let changed = {
        let mut runtimes = supervisor.lock();
        let Some(runtime) = runtimes.get_mut(&session.plugin_id) else {
            return false;
        };
        runtime.accepts(session) && runtime.subscriptions.accept(subscription, payload)
    };
    if !changed {
        return false;
    }
    let Some(window) = app.get_webview_window(&session.label) else {
        return false;
    };
    let detail = payload.to_string();
    let script = format!(
        "window.dispatchEvent(new CustomEvent(\"{}\", {{ detail: {detail} }}));",
        subscription.event()
    );
    window.eval(script).is_ok()
}

pub(crate) fn settings_changed(app: &AppHandle) {
    for session in subscribed_sessions(app, Subscription::Settings, Permission::SettingsRead) {
        let Some(package) = selected_package(app, &session) else {
            continue;
        };
        if !package.allows(Permission::SettingsRead) {
            continue;
        }
        push_subscription(
            app,
            &session,
            Subscription::Settings,
            &json!({ "values": package.settings }),
        );
    }
}

pub(crate) fn playback_changed(app: &AppHandle) {
    let sessions = subscribed_sessions(app, Subscription::Playback, Permission::PlaybackRead);
    if sessions.is_empty() {
        return;
    }
    let payload = playback_snapshot(app);
    for session in sessions {
        push_subscription(app, &session, Subscription::Playback, &payload);
    }
}

fn check_net_args(network: &[String], args: Option<Value>) -> Result<NetArgs, Value> {
    let args = serde_json::from_value::<NetArgs>(
        args.ok_or_else(|| failure("invalid", "This action requires arguments."))?,
    )
    .map_err(|_| failure("invalid", "The network request arguments are invalid."))?;
    if args.origin.len() > MAX_ORIGIN {
        return Err(failure("invalid", "The network origin is too long."));
    }
    if !network.iter().any(|allowed| allowed == &args.origin) {
        return Err(failure(
            "denied",
            "This package is not allowed to reach that origin.",
        ));
    }
    if args.path.is_empty()
        || args.path.len() > MAX_PATH
        || !args.path.starts_with('/')
        || args.path.starts_with("//")
        || args.path.contains('?')
        || args.path.contains('#')
        || args.path.contains('\\')
        || args.path.chars().any(char::is_control)
        || args
            .path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
    {
        return Err(failure("invalid", "The network path is invalid."));
    }
    if args.query.len() > MAX_QUERY_PAIRS {
        return Err(failure(
            "invalid",
            "The network request has too many query values.",
        ));
    }
    let mut bytes = 0usize;
    for (key, value) in &args.query {
        if key.is_empty()
            || key.len() > MAX_QUERY_KEY
            || value.len() > MAX_QUERY_VALUE
            || key.chars().any(char::is_control)
            || value.chars().any(char::is_control)
        {
            return Err(failure("invalid", "A network query value is invalid."));
        }
        bytes += key.len() + value.len();
    }
    if bytes > MAX_QUERY_BYTES {
        return Err(failure("invalid", "The network query is too large."));
    }
    Ok(args)
}

fn session_for_label(app: &AppHandle, label: &str) -> Option<Session> {
    supervisor_of(app).and_then(|supervisor| {
        supervisor
            .lock()
            .values()
            .find(|runtime| runtime.session.label == label && runtime.live())
            .map(|runtime| runtime.session.clone())
    })
}

fn is_current(app: &AppHandle, session: &Session) -> bool {
    supervisor_of(app).is_some_and(|supervisor| {
        supervisor
            .lock()
            .get(&session.plugin_id)
            .is_some_and(|runtime| runtime.accepts(session))
    })
}

fn selected_package(app: &AppHandle, session: &Session) -> Option<plugin::RuntimePackage> {
    let store = app.try_state::<Store>()?;
    plugin::runtime_package(&store, &session.plugin_id)
        .ok()
        .filter(|package| package.version == session.version)
}

async fn apply_control(app: AppHandle, command: &'static str) -> Result<(), String> {
    let auth = app.state::<Auth>();
    player::control(&app, &auth, command, None).await
}

pub(crate) fn guard<F>(handler: F) -> impl Fn(Invoke) -> bool + Send + Sync + 'static
where
    F: Fn(Invoke) -> bool + Send + Sync + 'static,
{
    move |invoke| {
        if !invoke.message.webview_ref().label().starts_with("ext-") {
            return handler(invoke);
        }
        let Invoke {
            message, resolver, ..
        } = invoke;
        let label = message.webview_ref().label().to_string();
        let app = message.webview_ref().window_ref().app_handle().clone();
        let Some(session) = session_for_label(&app, &label) else {
            crate::developer::bridge_error(
                &app,
                "unknown",
                "request",
                "The package runtime is not active.",
            );
            resolver.reject(failure("revoked", "The package runtime is not active."));
            return true;
        };
        if message.command() != BRIDGE
            || !message
                .webview_ref()
                .url()
                .is_ok_and(|url| permitted_url(&url, session.slot, &session.token))
        {
            crate::developer::bridge_error(
                &app,
                &session.plugin_id,
                "request",
                "Extension bridge command denied.",
            );
            resolver.reject(failure("denied", "Extension bridge command denied."));
            return true;
        }
        let Some(package) = selected_package(&app, &session) else {
            crate::developer::bridge_error(
                &app,
                &session.plugin_id,
                "request",
                "The package runtime is no longer active.",
            );
            resolver.reject(failure(
                "revoked",
                "The package runtime is no longer active.",
            ));
            return true;
        };
        let request = match message.payload() {
            InvokeBody::Json(payload) => serde_json::from_value::<BridgeRequest>(payload.clone())
                .map_err(|_| failure("invalid", "Unknown or malformed extension request.")),
            InvokeBody::Raw(_) => Err(failure("invalid", "The request must be JSON.")),
        };
        let request = match request {
            Ok(request) => request,
            Err(error) => {
                crate::developer::bridge_error(
                    &app,
                    &session.plugin_id,
                    "request",
                    "Unknown or malformed extension request.",
                );
                resolver.reject(error);
                return true;
            }
        };
        let operation = request.op.label();
        if request
            .args
            .as_ref()
            .is_some_and(|args| args.to_string().len() > MAX_ARGS)
        {
            crate::developer::bridge_error(
                &app,
                &session.plugin_id,
                operation,
                "The request is too large.",
            );
            resolver.reject(failure("invalid", "The request is too large."));
            return true;
        }
        if let Some(permission) = request.op.permission() {
            if !package.allows(permission) {
                crate::developer::bridge_error(
                    &app,
                    &session.plugin_id,
                    operation,
                    format!(
                        "This package has not been granted the {} permission.",
                        permission.label()
                    ),
                );
                resolver.reject(failure(
                    "denied",
                    format!(
                        "This package has not been granted the {} permission.",
                        permission.label()
                    ),
                ));
                return true;
            }
        }
        match request.op {
            Op::Identity => match no_args(request.args) {
                Ok(()) => resolver.resolve(json!({
                    "id": package.id,
                    "name": package.name,
                    "version": package.version,
                    "apiVersion": package.api_version,
                    "runtime": package.kind.label(),
                    "surface": if package.kind.has_surface() { Value::String("desktop".into()) } else { Value::Null },
                    "origin": format!("{}/{}", origin(session.slot), session.token),
                })),
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::SettingsRead => match no_args(request.args) {
                Ok(()) => resolver.resolve(json!({ "values": package.settings })),
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::SettingsSubscribe => match no_args(request.args) {
                Ok(()) => {
                    let payload = json!({ "values": package.settings });
                    if !register_subscription(&app, &session, Subscription::Settings, &payload) {
                        let error = failure(
                            "revoked",
                            "The package runtime is no longer active.",
                        );
                        log_bridge(&app, &session.plugin_id, operation, &error);
                        resolver.reject(error);
                        return true;
                    }
                    resolver.resolve(payload);
                    if let Some(package) = selected_package(&app, &session) {
                        push_subscription(
                            &app,
                            &session,
                            Subscription::Settings,
                            &json!({ "values": package.settings }),
                        );
                    }
                }
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::SurfaceReady => match no_args(request.args) {
                Ok(()) => {
                    let shown = supervisor_of(&app).is_some_and(|supervisor| {
                        let mut runtimes = supervisor.lock();
                        let Some(runtime) = runtimes.get_mut(&session.plugin_id) else {
                            return false;
                        };
                        if runtime.session.generation != session.generation
                            || runtime.session.token != session.token
                            || runtime.state == Lifecycle::Failed
                            || *runtime.session.cancel.borrow()
                        {
                            return false;
                        }
                        runtime.state = Lifecycle::Running;
                        runtime.reason = None;
                        runtime.pending_visible
                    });
                    if shown {
                        show_surface(&app, &session.plugin_id);
                    }
                    plugin::notify_plugins(&app);
                    crate::developer::runtime_event(
                        &app,
                        &session.plugin_id,
                        "running",
                        "surface ready",
                    );
                    resolver.resolve(json!({ "ready": true }));
                }
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::SurfaceClose => match no_args(request.args) {
                Ok(()) => {
                    resolver.resolve(json!({ "closed": true }));
                    close_surface(&app, &session.plugin_id);
                }
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::CommandPending => match no_args(request.args) {
                Ok(()) => {
                    if package.api_version != plugin::HOME_API_VERSION {
                        resolver.reject(failure(
                            "denied",
                            "Command delivery is available to API version 3 packages.",
                        ));
                        return true;
                    }
                    let running = supervisor_of(&app).is_some_and(|supervisor| {
                        supervisor
                            .lock()
                            .get(&session.plugin_id)
                            .is_some_and(|runtime| {
                                runtime.accepts(&session)
                                    && runtime.state == Lifecycle::Running
                            })
                    });
                    if !running {
                        resolver.resolve(Value::Null);
                        return true;
                    }
                    match session.take_command() {
                        Some(command) if command.version != package.version => {
                            resolver.reject(failure(
                                "stale",
                                "The package changed before the command was delivered.",
                            ));
                        }
                        Some(command) => {
                            let mut delivery = json!({ "command": command.command });
                            if let Some(id) = command.invocation_id {
                                delivery["invocationId"] = json!(id);
                            }
                            resolver.resolve(delivery);
                        }
                        None => resolver.resolve(Value::Null),
                    }
                }
                Err(error) => resolver.reject(error),
            },
            Op::DevelopmentReport => {
                match crate::developer::report_command(&app, &session.plugin_id, request.args) {
                    Ok(()) => resolver.resolve(json!({ "accepted": true })),
                    Err(error) => resolver.reject(failure(error.code, error.message)),
                }
            }
            Op::PlaybackRead => match no_args(request.args) {
                Ok(()) => resolver.resolve(playback_snapshot(&app)),
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::SportsActiveRead => match no_args(request.args) {
                Ok(()) => resolver.resolve(
                    serde_json::to_value(crate::commands::sports::active_snapshot(&app))
                        .unwrap_or(Value::Null),
                ),
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::SportsBadgesPublish => {
                match sanitize_sports_badges(&session.plugin_id, request.args) {
                    Ok(badges) => {
                        let count = badges.len();
                        #[cfg(debug_assertions)]
                        {
                            use tauri::Emitter;
                            let msg = format!("{}:sports.badges.publish: published {count} badges", session.plugin_id);
                            eprintln!("[plugin:{msg}]");
                            crate::commands::sports::push_log(&app, msg.clone());
                            let _ = app.emit_to("main", "luma:plugin-log", msg);
                        }
                        crate::commands::sports::set_plugin_badges(
                            &app,
                            &session.plugin_id,
                            badges,
                        );
                        resolver.resolve(json!({ "published": count }))
                    }
                    Err(error) => {
                        log_bridge(&app, &session.plugin_id, operation, &error);
                        resolver.reject(error)
                    }
                }
            }
            Op::PlaybackSubscribe => match no_args(request.args) {
                Ok(()) => {
                    let payload = playback_snapshot(&app);
                    if !register_subscription(&app, &session, Subscription::Playback, &payload) {
                        let error = failure(
                            "revoked",
                            "The package runtime is no longer active.",
                        );
                        log_bridge(&app, &session.plugin_id, operation, &error);
                        resolver.reject(error);
                        return true;
                    }
                    resolver.resolve(payload);
                    let latest = playback_snapshot(&app);
                    push_subscription(&app, &session, Subscription::Playback, &latest);
                }
                Err(error) => {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error)
                }
            },
            Op::NetGet => {
                let args = match check_net_args(&package.network, request.args) {
                    Ok(args) => args,
                    Err(error) => {
                        log_bridge(&app, &session.plugin_id, operation, &error);
                        resolver.reject(error);
                        return true;
                    }
                };
                if session
                    .network_busy
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    let error = failure(
                        "busy",
                        "A network request is already running for this package.",
                    );
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error);
                    return true;
                }
                let busy = session.network_busy.clone();
                let session = session.clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let mut cancel = session.cancel.clone();
                    let cancelled = tokio::select! {
                        outcome = net::get(&args.origin, &args.path, &args.query) => outcome,
                        _ = cancel.changed() => Err(net::Error::cancelled()),
                    };
                    busy.store(false, Ordering::Release);
                    if !is_current(&app, &session) {
                        let error = failure(
                            "stale",
                            "The package runtime changed before the request finished.",
                        );
                        log_bridge(&app, &session.plugin_id, operation, &error);
                        resolver.reject(error);
                        return;
                    }
                    match cancelled {
                        Ok(value) => resolver.resolve(value),
                        Err(error) => {
                            let error = failure(error.code, error.message);
                            log_bridge(&app, &session.plugin_id, operation, &error);
                            resolver.reject(error)
                        }
                    }
                });
            }
            Op::PlaybackToggle | Op::PlaybackNext | Op::PlaybackPrevious => {
                if let Err(error) = no_args(request.args) {
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error);
                    return true;
                }
                let Some(command) = request.op.player_command() else {
                    let error = failure("invalid", "Unknown playback command.");
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error);
                    return true;
                };
                if session
                    .control_busy
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    let error = failure(
                        "busy",
                        "A playback control is already in flight. Try again once it finishes.",
                    );
                    log_bridge(&app, &session.plugin_id, operation, &error);
                    resolver.reject(error);
                    return true;
                }
                let busy = session.control_busy.clone();
                let session = session.clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let outcome = apply_control(app.clone(), command).await;
                    busy.store(false, Ordering::Release);
                    if !is_current(&app, &session) {
                        let error = failure(
                            "stale",
                            "The package runtime changed before the command finished.",
                        );
                        log_bridge(&app, &session.plugin_id, operation, &error);
                        resolver.reject(error);
                        return;
                    }
                    match outcome {
                        Ok(()) => resolver.resolve(json!({ "accepted": true })),
                        Err(error) => {
                            let error = failure("unavailable", error);
                            log_bridge(&app, &session.plugin_id, operation, &error);
                            resolver.reject(error)
                        }
                    }
                });
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        allocate_slot, check_net_args, content_type, host, label, no_args, origin,
        permitted_request, permitted_url, playback, relay, restart_allowed, scheme, subscribed,
        token, BridgeRequest, Lifecycle, Op, PendingCommand, Permission, Runtime, Session,
        Subscription, Subscriptions, Supervisor, Trigger, MAX_RESTARTS, MAX_TEXT, PROTOCOL_POOL,
        SCHEME,
    };
    use crate::remote::server::MusicState;
    use serde_json::{json, Value};
    use std::sync::{atomic::AtomicBool, Arc, Mutex};
    use std::time::Instant;

    fn parsed(payload: Value) -> Result<BridgeRequest, serde_json::Error> {
        serde_json::from_value(payload)
    }

    fn music(title: Option<&str>, artist: Option<&str>, playing: bool) -> MusicState {
        MusicState {
            playing,
            title: title.map(str::to_string),
            artist: artist.map(str::to_string),
            has_artwork: true,
            position_ms: Some(42_000),
            duration_ms: Some(200_000),
            updated_at: 1_700_000_000_000,
            can_toggle: true,
            can_skip_next: true,
            can_skip_previous: true,
        }
    }

    #[test]
    fn only_navigates_to_this_packages_slot_origin_and_prefix() {
        let token = "abcdef0123456789";
        let slot = 3;
        let origin_host = host(slot);
        let allowed = |url: &str| {
            url.parse::<tauri::Url>()
                .is_ok_and(|url| permitted_url(&url, slot, token))
        };
        assert!(allowed(&format!(
            "http://{origin_host}/{token}/ui/index.html"
        )));
        assert!(allowed(&format!("http://{origin_host}/{token}")));
        assert!(!allowed(&format!(
            "http://{}/{token}/ui/index.html",
            host(2)
        )));
        assert!(!allowed(&format!(
            "http://{origin_host}/0123456789abcdef/ui/index.html"
        )));
        assert!(!allowed(&format!(
            "http://{origin_host}/{token}x/ui/index.html"
        )));
        assert!(!allowed(&format!(
            "https://{origin_host}/{token}/ui/index.html"
        )));
        assert!(!allowed(&format!(
            "http://{origin_host}.evil.com/{token}/ui/index.html"
        )));
        assert!(!allowed(&format!(
            "http://{origin_host}:8080/{token}/ui/index.html"
        )));
        assert!(!allowed(&format!(
            "http://user@{origin_host}/{token}/ui/index.html"
        )));
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("file:///C:/Windows/System32/"));
    }

    #[test]
    fn slot_origins_are_distinct_and_tokens_still_separate_sessions() {
        let first = token("com.example.alpha", 1);
        let second = token("com.example.beta", 1);
        let later = token("com.example.alpha", 2);
        assert_ne!(origin(0), origin(1));
        assert_eq!(origin(0), format!("http://{}", host(0)));
        assert_eq!(scheme(0), "luma-extension.s0");
        assert_eq!(label(&first), format!("ext-{first}"));

        let own = format!("http://{}/{first}/ui/index.html", host(0))
            .parse::<tauri::Url>()
            .unwrap();
        let foreign_slot = format!("http://{}/{first}/ui/index.html", host(1))
            .parse::<tauri::Url>()
            .unwrap();
        let foreign_path = format!("http://{}/{second}/ui/index.html", host(0))
            .parse::<tauri::Url>()
            .unwrap();
        let next_generation = format!("http://{}/{later}/ui/index.html", host(0))
            .parse::<tauri::Url>()
            .unwrap();
        assert!(permitted_url(&own, 0, &first));
        assert!(!permitted_url(&foreign_slot, 0, &first));
        assert!(!permitted_url(&foreign_path, 0, &first));
        assert!(!permitted_url(&next_generation, 0, &first));
    }

    #[test]
    fn slot_pool_is_bounded_and_reuses_only_free_slots() {
        assert_eq!(allocate_slot(&[]), Some(0));
        assert_eq!(allocate_slot(&[0, 1, 2]), Some(3));
        let full: Vec<usize> = (0..PROTOCOL_POOL).collect();
        assert_eq!(allocate_slot(&full), None);
        assert_eq!(allocate_slot(&[1, 2]), Some(0));
    }

    #[test]
    fn protocol_requests_are_bound_to_session_scheme_and_path() {
        let first = token("com.example.alpha", 1);
        let second = token("com.example.beta", 1);
        let slot = 0;
        let uri = |value: String| value.parse::<tauri::http::Uri>().unwrap();
        assert!(permitted_request(
            &uri(format!(
                "{}://localhost/{first}/ui/index.html",
                scheme(slot)
            )),
            slot,
            &first
        ));
        assert!(permitted_request(
            &uri(format!("{}://localhost/{first}", scheme(slot))),
            slot,
            &first
        ));
        for rejected in [
            format!("{}://localhost/{first}/ui/index.html", scheme(1)),
            format!("{}://localhost/{second}/ui/index.html", scheme(slot)),
            format!("{SCHEME}://localhost/{first}/ui/index.html"),
            format!("http://localhost/{first}/ui/index.html"),
            format!("{}://localhost:8080/{first}/ui/index.html", scheme(slot)),
            format!("{}://user@localhost/{first}/ui/index.html", scheme(slot)),
        ] {
            let allowed = rejected
                .parse::<tauri::http::Uri>()
                .map(|uri| permitted_request(&uri, slot, &first))
                .unwrap_or(false);
            assert!(!allowed, "{rejected} must not be served");
        }
    }

    fn session(plugin_id: &str, generation: u64) -> (Session, tokio::sync::watch::Sender<bool>) {
        let token = token(plugin_id, generation);
        let (cancel, receiver) = tokio::sync::watch::channel(false);
        (
            Session {
                plugin_id: plugin_id.to_string(),
                version: "1.0.0".into(),
                label: label(&token),
                token,
                slot: 0,
                generation,
                permissions: Default::default(),
                cancel: receiver,
                network_busy: Arc::new(AtomicBool::new(false)),
                control_busy: Arc::new(AtomicBool::new(false)),
                pending: Arc::new(Mutex::new(None)),
                delivered: Arc::new(tokio::sync::watch::channel(0).0),
            },
            cancel,
        )
    }

    fn runtime(
        session: &Session,
        cancel: tokio::sync::watch::Sender<bool>,
        state: Lifecycle,
    ) -> Runtime {
        Runtime {
            session: session.clone(),
            cancel,
            kind: super::RuntimeKind::IsolatedUi,
            trigger: Trigger::LauncherOpen,
            state,
            reason: None,
            restarts: 0,
            crashes: Vec::new(),
            pending_visible: false,
            surface_visible: false,
            subscriptions: Subscriptions::default(),
        }
    }

    #[test]
    fn cancelled_or_failed_sessions_are_not_treated_as_current() {
        let (active, cancel) = session("com.example.alpha", 1);
        let mut current = runtime(&active, cancel.clone(), Lifecycle::Starting);
        assert!(current.accepts(&active));

        current.state = Lifecycle::Failed;
        assert!(!current.accepts(&active));

        current.state = Lifecycle::Starting;
        cancel.send_replace(true);
        assert!(!current.accepts(&active));

        let (replacement, _) = session("com.example.alpha", 2);
        assert!(!current.accepts(&replacement));
    }

    #[test]
    fn pending_commands_are_consumed_once_and_revoked_with_the_session() {
        let (session, _) = session("com.example.alpha", 1);
        assert!(session.take_command().is_none());
        *session.pending_lock() = Some(PendingCommand {
            command: "openForecast".into(),
            version: "1.0.0".into(),
            invocation_id: Some("dev-run".into()),
        });
        let command = session
            .take_command()
            .expect("the pending command is available");
        assert_eq!(command.command, "openForecast");
        assert_eq!(command.version, "1.0.0");
        assert_eq!(command.invocation_id.as_deref(), Some("dev-run"));
        assert!(session.take_command().is_none());
        *session.pending_lock() = Some(PendingCommand {
            command: "openForecast".into(),
            version: "1.0.0".into(),
            invocation_id: None,
        });
        session.revoke();
        assert!(session.pending_lock().is_none());
    }

    #[test]
    fn duplicate_command_dispatches_are_suppressed_while_pending() {
        let supervisor = Supervisor::default();
        assert!(supervisor.begin_dispatch("com.example.weather"));
        assert!(!supervisor.begin_dispatch("com.example.weather"));
        assert!(supervisor.begin_dispatch("com.example.other"));
        supervisor.end_dispatch("com.example.weather");
        assert!(supervisor.begin_dispatch("com.example.weather"));
    }

    #[test]
    fn subscriptions_register_once_and_suppress_duplicate_snapshots() {
        let mut subscriptions = Subscriptions::default();
        assert!(subscriptions.settings.is_none());
        subscriptions.register(
            Subscription::Settings,
            &json!({ "values": { "theme": "mint" } }),
        );
        assert!(subscriptions.settings.is_some());
        assert!(!subscriptions.accept(
            Subscription::Settings,
            &json!({ "values": { "theme": "mint" } })
        ));
        assert!(subscriptions.accept(
            Subscription::Settings,
            &json!({ "values": { "theme": "ember" } })
        ));
        subscriptions.register(
            Subscription::Playback,
            &json!({ "connected": true, "playing": false }),
        );
        assert!(!subscriptions.accept(
            Subscription::Playback,
            &json!({ "connected": true, "playing": false })
        ));
        assert!(subscriptions.accept(
            Subscription::Playback,
            &json!({ "connected": true, "playing": true })
        ));
    }

    #[test]
    fn only_live_sessions_with_the_read_grant_and_subscription_receive_events() {
        let (mut current, cancel) = session("com.example.alpha", 1);
        current.permissions.insert(Permission::SettingsRead);
        let mut runtime = runtime(&current, cancel.clone(), Lifecycle::Running);
        runtime.subscriptions.register(
            Subscription::Settings,
            &json!({ "values": { "theme": "mint" } }),
        );
        assert!(subscribed(
            &runtime,
            Subscription::Settings,
            Permission::SettingsRead
        ));
        assert!(!subscribed(
            &runtime,
            Subscription::Settings,
            Permission::PlaybackRead
        ));
        assert!(!subscribed(
            &runtime,
            Subscription::Playback,
            Permission::PlaybackRead
        ));

        runtime.state = Lifecycle::Failed;
        assert!(!subscribed(
            &runtime,
            Subscription::Settings,
            Permission::SettingsRead
        ));
        runtime.state = Lifecycle::Running;
        cancel.send_replace(true);
        assert!(!subscribed(
            &runtime,
            Subscription::Settings,
            Permission::SettingsRead
        ));
    }

    #[test]
    fn a_stop_invalidates_only_that_packages_in_flight_start() {
        let supervisor = Supervisor::default();
        let first = supervisor.epoch("com.example.alpha");
        let second = supervisor.epoch("com.example.beta");
        supervisor.bump("com.example.alpha");
        assert_ne!(supervisor.epoch("com.example.alpha"), first);
        assert_eq!(supervisor.epoch("com.example.beta"), second);
    }

    #[cfg(windows)]
    #[test]
    fn renderer_exits_and_hangs_are_surface_failures() {
        use webview2_com::Microsoft::Web::WebView2::Win32::*;
        for kind in [
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
        ] {
            assert!(super::surface_failed(kind));
        }
        for kind in [
            COREWEBVIEW2_PROCESS_FAILED_KIND_GPU_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_UTILITY_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_SANDBOX_HELPER_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_PPAPI_BROKER_PROCESS_EXITED,
        ] {
            assert!(!super::surface_failed(kind));
        }
    }

    #[test]
    fn serves_package_assets_with_explicit_content_types() {
        assert_eq!(content_type("ui/index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("ui/page.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("ui/page.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("ui/icon.svg"), "image/svg+xml");
        assert_eq!(content_type("ui/logo.PNG"), "image/png");
        assert_eq!(content_type("ui/data.json"), "application/json");
        assert_eq!(content_type("ui/unknown.bin"), "application/octet-stream");
    }

    #[test]
    fn parses_exact_bridge_operations() {
        for (payload, op) in [
            (json!({ "op": "identity" }), Op::Identity),
            (json!({ "op": "settings.read" }), Op::SettingsRead),
            (json!({ "op": "settings.subscribe" }), Op::SettingsSubscribe),
            (json!({ "op": "surface.ready" }), Op::SurfaceReady),
            (json!({ "op": "surface.close" }), Op::SurfaceClose),
            (json!({ "op": "commands.pending" }), Op::CommandPending),
            (json!({ "op": "playback.read" }), Op::PlaybackRead),
            (json!({ "op": "playback.subscribe" }), Op::PlaybackSubscribe),
            (json!({ "op": "playback.toggle" }), Op::PlaybackToggle),
            (json!({ "op": "playback.next" }), Op::PlaybackNext),
            (json!({ "op": "playback.previous" }), Op::PlaybackPrevious),
        ] {
            assert_eq!(parsed(payload).unwrap().op, op);
        }
        let net = parsed(json!({
            "op": "net.get",
            "args": {
                "origin": "https://api.open-meteo.com",
                "path": "/v1/forecast",
                "query": [["latitude", "47.6062"]]
            }
        }))
        .unwrap();
        assert_eq!(net.op, Op::NetGet);
        assert!(net.args.is_some());
        for payload in [
            json!({ "op": "weather.read" }),
            json!({ "op": "install" }),
            json!({ "op": "identity", "package": "com.example.evil" }),
            json!({}),
        ] {
            assert!(parsed(payload).is_err());
        }
    }

    #[test]
    fn bridge_operations_map_to_exact_permissions() {
        assert_eq!(Op::Identity.permission(), None);
        assert_eq!(Op::SurfaceReady.permission(), None);
        assert_eq!(Op::SurfaceClose.permission(), None);
        assert_eq!(Op::CommandPending.permission(), None);
        assert_eq!(
            Op::SettingsRead
                .permission()
                .map(|permission| permission.label()),
            Some("settings.read")
        );
        assert_eq!(
            Op::SettingsSubscribe
                .permission()
                .map(|permission| permission.label()),
            Some("settings.read")
        );
        assert_eq!(
            Op::PlaybackRead
                .permission()
                .map(|permission| permission.label()),
            Some("playback.read")
        );
        assert_eq!(
            Op::PlaybackSubscribe
                .permission()
                .map(|permission| permission.label()),
            Some("playback.read")
        );
        for op in [Op::PlaybackToggle, Op::PlaybackNext, Op::PlaybackPrevious] {
            assert_eq!(
                op.permission().map(|permission| permission.label()),
                Some("playback.control")
            );
        }
        assert_eq!(
            Op::NetGet.permission().map(|permission| permission.label()),
            Some("network.get")
        );
    }

    #[test]
    fn argument_free_operations_reject_arguments() {
        assert!(no_args(None).is_ok());
        assert!(no_args(Some(Value::Null)).is_ok());
        for args in [json!({}), json!([]), json!("value"), json!(true)] {
            assert!(no_args(Some(args)).is_err());
        }
    }

    #[test]
    fn net_requests_are_bound_to_declared_origins_and_bounded_paths() {
        let allowed = ["https://api.open-meteo.com".to_string()];
        let valid = check_net_args(
            &allowed,
            Some(json!({
                "origin": "https://api.open-meteo.com",
                "path": "/v1/forecast",
                "query": [["latitude", "47.6062"]]
            })),
        )
        .unwrap();
        assert_eq!(valid.origin, "https://api.open-meteo.com");
        assert_eq!(valid.path, "/v1/forecast");

        for (args, code) in [
            (
                json!({ "origin": "https://example.com", "path": "/v1/forecast" }),
                "denied",
            ),
            (
                json!({ "origin": "https://api.open-meteo.com", "path": "//evil.example" }),
                "invalid",
            ),
            (
                json!({ "origin": "https://api.open-meteo.com", "path": "/../Cargo.toml" }),
                "invalid",
            ),
            (
                json!({ "origin": "https://api.open-meteo.com", "path": "/v1/forecast?x=1" }),
                "invalid",
            ),
            (
                json!({
                    "origin": "https://api.open-meteo.com",
                    "path": "/v1/forecast",
                    "query": [["", "value"]]
                }),
                "invalid",
            ),
            (
                json!({
                    "origin": "https://api.open-meteo.com",
                    "path": "/v1/forecast",
                    "query": [["a", "x".repeat(513)]]
                }),
                "invalid",
            ),
            (
                json!({
                    "origin": "https://api.open-meteo.com",
                    "path": "/v1/forecast",
                    "query": [["a", "b"], ["c", "d"], ["e", "f"], ["g", "h"], ["i", "j"],
                              ["k", "l"], ["m", "n"], ["o", "p"], ["q", "r"], ["s", "t"],
                              ["u", "v"], ["w", "x"], ["y", "z"], ["aa", "bb"], ["cc", "dd"],
                              ["ee", "ff"], ["gg", "hh"]]
                }),
                "invalid",
            ),
        ] {
            assert_eq!(
                check_net_args(&allowed, Some(args))
                    .err()
                    .and_then(|error| {
                        error
                            .get("code")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    }),
                Some(code.to_string())
            );
        }
        assert!(check_net_args(&allowed, None).is_err());
        assert!(check_net_args(
            &allowed,
            Some(json!({
                "origin": "https://api.open-meteo.com",
                "path": "/v1/forecast",
                "headers": {}
            }))
        )
        .is_err());
    }

    #[test]
    fn relays_only_bounded_navigation_keys_to_the_surface() {
        for (command, key) in [
            ("up", "ArrowUp"),
            ("down", "ArrowDown"),
            ("left", "ArrowLeft"),
            ("right", "ArrowRight"),
        ] {
            let script = relay(command).expect("directions relay");
            assert!(script.contains(key));
        }
        let activate = relay("activate").expect("activate relays");
        assert!(activate.contains("document.activeElement"));
        assert!(activate.contains("button:not(:disabled)"));
        assert!(activate.contains(".click()"));
        for command in [
            "home",
            "back",
            "close",
            "identity",
            "playback",
            "up; alert(1)",
            "ArrowUp",
            "",
        ] {
            assert!(
                relay(command).is_none(),
                "{command} must not reach the surface"
            );
        }
    }

    #[test]
    fn playback_projection_returns_only_the_contract_fields() {
        let snapshot = playback(true, Some(&music(Some("Song"), Some("Artist"), true)));
        let mut keys: Vec<_> = snapshot
            .as_object()
            .expect("snapshot is an object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(keys, ["artist", "connected", "playing", "title"]);
        assert_eq!(
            snapshot,
            json!({ "connected": true, "title": "Song", "artist": "Artist", "playing": true })
        );
        let encoded = serde_json::to_string(&snapshot).unwrap();
        for private in [
            "hasArtwork",
            "position",
            "duration",
            "updatedAt",
            "canToggle",
            "canSkip",
            "artwork",
            "track",
            "device",
            "volume",
            "repeat",
            "token",
            "client",
            "path",
        ] {
            assert!(
                !encoded.contains(private),
                "{private} leaked from the projection"
            );
        }
    }

    #[test]
    fn playback_projection_handles_no_playback() {
        assert_eq!(
            playback(false, None),
            json!({ "connected": false, "title": null, "artist": null, "playing": false })
        );
        let blank = playback(true, Some(&music(Some("   "), None, false)));
        assert_eq!(blank["title"], Value::Null);
        assert_eq!(blank["artist"], Value::Null);
        assert_eq!(blank["connected"], true);
        assert_eq!(blank["playing"], false);
    }

    #[test]
    fn playback_projection_trims_and_bounds_text() {
        let long = "é".repeat(MAX_TEXT + 25);
        let snapshot = playback(true, Some(&music(Some(&long), Some("  Artist  "), true)));
        assert_eq!(
            snapshot["title"]
                .as_str()
                .map(|value| value.chars().count()),
            Some(MAX_TEXT)
        );
        assert_eq!(snapshot["artist"], "Artist");
    }

    #[test]
    fn crash_restarts_are_bounded_to_three_attempts() {
        let mut crashes = Vec::new();
        for _ in 0..MAX_RESTARTS {
            crashes.push(Instant::now());
            assert!(restart_allowed(&crashes));
        }
        crashes.push(Instant::now());
        assert!(!restart_allowed(&crashes));
    }
}
