mod net;

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use crate::{
    customization::{
        plugin::{self, Grant, Permission, RuntimeKind, RuntimeStatus, Trigger},
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
const MAX_TEXT: usize = 200;
const MAX_ARGS: usize = 16 * 1024;
const MAX_ORIGIN: usize = 128;
const MAX_PATH: usize = 256;
const MAX_QUERY_PAIRS: usize = 16;
const MAX_QUERY_KEY: usize = 64;
const MAX_QUERY_VALUE: usize = 512;
const MAX_QUERY_BYTES: usize = 2048;
const READY_TIMEOUT: Duration = Duration::from_secs(5);
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
struct Session {
    plugin_id: String,
    label: String,
    token: String,
    slot: usize,
    generation: u64,
    cancel: tokio::sync::watch::Receiver<bool>,
    network_busy: Arc<AtomicBool>,
    control_busy: Arc<AtomicBool>,
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
    closed: AtomicBool,
}

impl Supervisor {
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Runtime>> {
        self.runtimes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
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
        runtime.cancel.send_replace(true);
        plugin::notify_plugins(app);
    }
    removed
}

fn stop_runtime(app: &AppHandle, plugin_id: &str) {
    if let Some(runtime) = take_runtime(app, plugin_id) {
        destroy_window(app, runtime.session.label);
    }
}

fn close_surface(app: &AppHandle, plugin_id: &str) {
    let surface = supervisor_of(app).is_some_and(|supervisor| {
        supervisor
            .lock()
            .get(plugin_id)
            .is_some_and(|runtime| runtime.kind.has_surface())
    });
    if surface {
        stop_runtime(app, plugin_id);
    }
    focus_main(app);
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
            start(&app, &plugin_id, grant, Vec::new(), 0, false, false, None);
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
        start(&app, &plugin_id, grant, Vec::new(), 0, false, false, None);
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

fn start(
    app: &AppHandle,
    plugin_id: &str,
    grant: Grant,
    crashes: Vec<Instant>,
    restarts: u32,
    show: bool,
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
    if package.kind == RuntimeKind::IsolatedUi && !show {
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
            label: window_label.clone(),
            token: token.clone(),
            slot,
            generation,
            cancel: receiver,
            network_busy: Arc::new(AtomicBool::new(false)),
            control_busy: Arc::new(AtomicBool::new(false)),
        };
        if let Some(previous) = runtimes.remove(plugin_id) {
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
            },
        );
        (slot, session)
    };
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
            true,
            Some(epoch),
        );
    });
}

fn restart_allowed(crashes: &[Instant]) -> bool {
    crashes.len() <= MAX_RESTARTS
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
    start(app, plugin_id, grant, crashes, restarts, true, true, None);
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

pub(crate) fn autostart(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Some(store) = app.try_state::<Store>() else {
            return;
        };
        let Ok(granted) = plugin::granted_plugins(&store) else {
            return;
        };
        for (plugin_id, grant) in granted {
            if grant.trigger == Trigger::LauncherOpen {
                continue;
            }
            start(&app, &plugin_id, grant, Vec::new(), 0, false, false, None);
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
            start(&app, &plugin_id, grant, Vec::new(), 0, true, false, None);
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
    #[serde(rename = "net.get")]
    NetGet,
    #[serde(rename = "playback.read")]
    PlaybackRead,
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
}

impl Op {
    fn permission(self) -> Option<Permission> {
        match self {
            Self::SettingsRead => Some(Permission::SettingsRead),
            Self::NetGet => Some(Permission::NetworkGet),
            Self::PlaybackRead => Some(Permission::PlaybackRead),
            Self::PlaybackToggle | Self::PlaybackNext | Self::PlaybackPrevious => {
                Some(Permission::PlaybackControl)
            }
            Self::Identity | Self::SurfaceReady | Self::SurfaceClose => None,
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
struct NetArgs {
    origin: String,
    path: String,
    #[serde(default)]
    query: Vec<(String, String)>,
}

fn failure(code: &'static str, message: impl ToString) -> Value {
    json!({ "code": code, "message": message.to_string() })
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
    plugin::runtime_package(&store, &session.plugin_id).ok()
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
            resolver.reject(failure("revoked", "The package runtime is not active."));
            return true;
        };
        if message.command() != BRIDGE
            || !message
                .webview_ref()
                .url()
                .is_ok_and(|url| permitted_url(&url, session.slot, &session.token))
        {
            resolver.reject(failure("denied", "Extension bridge command denied."));
            return true;
        }
        let Some(package) = selected_package(&app, &session) else {
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
                resolver.reject(error);
                return true;
            }
        };
        if request
            .args
            .as_ref()
            .is_some_and(|args| args.to_string().len() > MAX_ARGS)
        {
            resolver.reject(failure("invalid", "The request is too large."));
            return true;
        }
        if let Some(permission) = request.op.permission() {
            if !package.allows(permission) {
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
                Err(error) => resolver.reject(error),
            },
            Op::SettingsRead => match no_args(request.args) {
                Ok(()) => resolver.resolve(json!({ "values": package.settings })),
                Err(error) => resolver.reject(error),
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
                    resolver.resolve(json!({ "ready": true }));
                }
                Err(error) => resolver.reject(error),
            },
            Op::SurfaceClose => match no_args(request.args) {
                Ok(()) => {
                    resolver.resolve(json!({ "closed": true }));
                    close_surface(&app, &session.plugin_id);
                }
                Err(error) => resolver.reject(error),
            },
            Op::PlaybackRead => match no_args(request.args) {
                Ok(()) => resolver.resolve(playback_snapshot(&app)),
                Err(error) => resolver.reject(error),
            },
            Op::NetGet => {
                let args = match check_net_args(&package.network, request.args) {
                    Ok(args) => args,
                    Err(error) => {
                        resolver.reject(error);
                        return true;
                    }
                };
                if session
                    .network_busy
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    resolver.reject(failure(
                        "busy",
                        "A network request is already running for this package.",
                    ));
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
                        resolver.reject(failure(
                            "stale",
                            "The package runtime changed before the request finished.",
                        ));
                        return;
                    }
                    match cancelled {
                        Ok(value) => resolver.resolve(value),
                        Err(error) => resolver.reject(failure(error.code, error.message)),
                    }
                });
            }
            Op::PlaybackToggle | Op::PlaybackNext | Op::PlaybackPrevious => {
                if let Err(error) = no_args(request.args) {
                    resolver.reject(error);
                    return true;
                }
                let Some(command) = request.op.player_command() else {
                    resolver.reject(failure("invalid", "Unknown playback command."));
                    return true;
                };
                if session
                    .control_busy
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    resolver.reject(failure(
                        "busy",
                        "A playback control is already in flight. Try again once it finishes.",
                    ));
                    return true;
                }
                let busy = session.control_busy.clone();
                let session = session.clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let outcome = apply_control(app.clone(), command).await;
                    busy.store(false, Ordering::Release);
                    if !is_current(&app, &session) {
                        resolver.reject(failure(
                            "stale",
                            "The package runtime changed before the command finished.",
                        ));
                        return;
                    }
                    match outcome {
                        Ok(()) => resolver.resolve(json!({ "accepted": true })),
                        Err(error) => resolver.reject(failure("unavailable", error)),
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
        permitted_request, permitted_url, playback, relay, restart_allowed, scheme, token,
        BridgeRequest, Lifecycle, Op, Runtime, Session, Supervisor, Trigger, MAX_RESTARTS,
        MAX_TEXT, PROTOCOL_POOL, SCHEME,
    };
    use crate::remote::server::MusicState;
    use serde_json::{json, Value};
    use std::sync::{atomic::AtomicBool, Arc};
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
        assert!(allowed(&format!("http://{origin_host}/{token}/ui/index.html")));
        assert!(allowed(&format!("http://{origin_host}/{token}")));
        assert!(!allowed(&format!(
            "http://{}/{token}/ui/index.html",
            host(2)
        )));
        assert!(!allowed(&format!(
            "http://{origin_host}/0123456789abcdef/ui/index.html"
        )));
        assert!(!allowed(&format!("http://{origin_host}/{token}x/ui/index.html")));
        assert!(!allowed(&format!("https://{origin_host}/{token}/ui/index.html")));
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
                label: label(&token),
                token,
                slot: 0,
                generation,
                cancel: receiver,
                network_busy: Arc::new(AtomicBool::new(false)),
                control_busy: Arc::new(AtomicBool::new(false)),
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
            (json!({ "op": "surface.ready" }), Op::SurfaceReady),
            (json!({ "op": "surface.close" }), Op::SurfaceClose),
            (json!({ "op": "playback.read" }), Op::PlaybackRead),
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
        assert_eq!(
            Op::SettingsRead
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
