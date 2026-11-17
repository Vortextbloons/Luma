mod net;

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
use std::time::Duration;

use crate::{
    customization::{
        plugin::{self, Permission, RuntimePackage},
        Store,
    },
    integrations::spotify::{auth::Auth, player},
    remote::server::{MusicState, Remote},
};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{
    http::{header, Response, StatusCode},
    ipc::{Invoke, InvokeBody},
    webview::NewWindowResponse,
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub(crate) const SCHEME: &str = "luma-extension-spike";

const LABEL: &str = "extension-spike";
const HOST: &str = "luma-extension-spike.localhost";
const ORIGIN: &str = "http://luma-extension-spike.localhost";
const BRIDGE: &str = "extension_bridge";
const ENV: &str = "LUMA_EXTENSION_SPIKE";
const PACKAGE_ENV: &str = "LUMA_EXTENSION_PLUGIN";
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src ipc: http://ipc.localhost; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
const MAX_TEXT: usize = 200;
const MAX_ARGS: usize = 16 * 1024;
const MAX_ORIGIN: usize = 128;
const MAX_PATH: usize = 256;
const MAX_QUERY_PAIRS: usize = 16;
const MAX_QUERY_KEY: usize = 64;
const MAX_QUERY_VALUE: usize = 512;
const MAX_QUERY_BYTES: usize = 2048;
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const NOT_READY: &str = "The package surface did not become ready. Luma returned to the launcher.";

static SELECTED: Mutex<Option<String>> = Mutex::new(None);
static STARTED: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static READY: AtomicU64 = AtomicU64::new(0);
static CONTROL: AtomicBool = AtomicBool::new(false);

fn enabled() -> bool {
    cfg!(debug_assertions) && matches!(std::env::var(ENV).as_deref(), Ok("1"))
}

pub(crate) fn development_enabled() -> bool {
    enabled()
}

fn selected() -> Option<String> {
    SELECTED.lock().ok().and_then(|selected| selected.clone())
}

fn select(plugin_id: &str) {
    if let Ok(mut selected) = SELECTED.lock() {
        *selected = Some(plugin_id.to_string());
    }
}

fn clear_selected() {
    if let Ok(mut selected) = SELECTED.lock() {
        *selected = None;
    }
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
    let decoded = percent_encoding::percent_decode_str(path).decode_utf8().ok()?;
    Some(decoded.trim_start_matches('/').to_string())
}

fn serve_package(package: &RuntimePackage, path: &str) -> Response<Vec<u8>> {
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

fn permitted(url: &tauri::Url) -> bool {
    url.scheme() == "http"
        && url.host_str() == Some(HOST)
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

pub(crate) fn protocol<R: tauri::Runtime>(
    context: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if !enabled() || context.webview_label() != LABEL {
        return not_found();
    }
    let Some(plugin_id) = selected() else {
        return not_found();
    };
    let Some(store) = context.app_handle().try_state::<Store>() else {
        return not_found();
    };
    let Ok(package) = plugin::runtime_package(&store, &plugin_id) else {
        return not_found();
    };
    serve_package(&package, request.uri().path())
}

pub(crate) fn open(app: &AppHandle, plugin_id: &str) -> Result<(), String> {
    if !enabled() {
        return Err(format!(
            "The extension runtime harness requires a development build with {ENV}=1."
        ));
    }
    let package = {
        let store = app.state::<Store>();
        plugin::runtime_package(&store, plugin_id).map_err(|error| error.message)?
    };
    if let Some(window) = app.get_webview_window(LABEL) {
        if selected().as_deref() != Some(plugin_id) {
            return Err("Close the open development surface before opening another package.".into());
        }
        let _ = window.show();
        if READY.load(Ordering::Acquire) == GENERATION.load(Ordering::Acquire) {
            let _ = window.set_focus();
        }
        return Ok(());
    }
    let main = app
        .get_webview_window("main")
        .ok_or("Luma window is unavailable")?;
    let size = main.inner_size().map_err(|error| error.to_string())?;
    let position = main.outer_position().map_err(|error| error.to_string())?;
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let url = format!("{ORIGIN}/{}", package.entry)
        .parse()
        .map_err(|error| format!("Invalid extension origin: {error}"))?;
    let generation = GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    READY.store(0, Ordering::Release);
    select(&package.id);
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::CustomProtocol(url))
        .title(format!("{} · Luma extension", package.name))
        .inner_size(640.0, 540.0)
        .position(
            position.x as f64 / scale + (size.width as f64 / scale - 640.0) / 2.0,
            position.y as f64 / scale + (size.height as f64 / scale - 540.0) / 2.0,
        )
        .min_inner_size(520.0, 420.0)
        .incognito(true)
        .visible(true)
        .focused(false)
        .on_navigation(permitted)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
        .map_err(|error| {
            clear_selected();
            format!("Could not open the extension surface: {error}")
        })?;
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            destroy(&handle, true);
        }
        tauri::WindowEvent::Destroyed => {
            clear_selected();
            GENERATION.fetch_add(1, Ordering::AcqRel);
            READY.store(0, Ordering::Release);
        }
        _ => {}
    });
    watchdog(app, generation);
    Ok(())
}

fn watchdog(app: &AppHandle, generation: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(READY_TIMEOUT).await;
        if GENERATION.load(Ordering::Acquire) != generation
            || READY.load(Ordering::Acquire) == generation
        {
            return;
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.destroy();
        }
        clear_selected();
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
        }
        plugin::notice(&app, NOT_READY);
    });
}

fn show_ready(app: &AppHandle) -> Result<(), Value> {
    let generation = GENERATION.load(Ordering::Acquire);
    let Some(window) = app.get_webview_window(LABEL) else {
        return Err(failure(
            "unavailable",
            "The package surface is no longer open.",
        ));
    };
    window
        .show()
        .map_err(|_| failure("unavailable", "The package surface could not be shown."))?;
    window
        .set_focus()
        .map_err(|_| failure("unavailable", "The package surface could not be focused."))?;
    READY.store(generation, Ordering::Release);
    Ok(())
}

fn destroy(app: &AppHandle, restore_focus: bool) {
    clear_selected();
    GENERATION.fetch_add(1, Ordering::AcqRel);
    READY.store(0, Ordering::Release);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.destroy();
        }
        if restore_focus {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_focus();
            }
        }
    });
}

pub(crate) fn dismiss(app: &AppHandle) {
    destroy(app, false);
}

pub(crate) fn close(app: &AppHandle) {
    destroy(app, true);
}

pub(crate) fn invalidate(app: &AppHandle, plugin_id: &str) {
    if selected().as_deref() == Some(plugin_id) {
        close(app);
    }
}

pub(crate) fn active(app: &AppHandle) -> bool {
    selected().is_some()
        && app.get_webview_window(LABEL).is_some_and(|window| {
            window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
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
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.eval(script);
    }
}

pub(crate) fn autostart(app: AppHandle) {
    if !enabled() || STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let plugin_id = match discover(&app) {
        Ok(Some(plugin_id)) => plugin_id,
        Ok(None) => return,
        Err(error) => {
            eprintln!("Extension runtime: {error}");
            return;
        }
    };
    if let Err(error) = open(&app, &plugin_id) {
        eprintln!("Extension runtime: {error}");
    }
}

fn discover(app: &AppHandle) -> Result<Option<String>, String> {
    let store = app.try_state::<Store>().ok_or("Plugin storage is unavailable.")?;
    let ids = plugin::executable_package_ids(&store).map_err(|error| error.message)?;
    if let Ok(requested) = std::env::var(PACKAGE_ENV) {
        if ids.iter().any(|id| id == &requested) {
            return Ok(Some(requested));
        }
        return Err(format!(
            "{PACKAGE_ENV} names {requested}, but no installed isolated-ui package has that id."
        ));
    }
    match ids.as_slice() {
        [only] => Ok(Some(only.clone())),
        [] => Ok(None),
        _ => Err(format!(
            "Several isolated-ui packages are installed. Set {PACKAGE_ENV} to the package id to open."
        )),
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
        return Err(failure("invalid", "The network request has too many query values."));
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

fn selected_package(app: &AppHandle) -> Option<RuntimePackage> {
    let plugin_id = selected()?;
    let store = app.try_state::<Store>()?;
    plugin::runtime_package(&store, &plugin_id).ok()
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
        if invoke.message.webview_ref().label() != LABEL {
            return handler(invoke);
        }
        let Invoke {
            message, resolver, ..
        } = invoke;
        if !enabled()
            || !message.webview_ref().url().is_ok_and(|url| permitted(&url))
            || message.command() != BRIDGE
        {
            resolver.reject(failure("denied", "Extension bridge command denied."));
            return true;
        }
        let app = message.webview_ref().window_ref().app_handle().clone();
        let Some(package) = selected_package(&app) else {
            resolver.reject(failure(
                "revoked",
                "The package surface is no longer active.",
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
                        "This package has not declared the {} permission.",
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
                    "runtime": "isolated-ui",
                    "surface": "desktop",
                    "origin": ORIGIN,
                })),
                Err(error) => resolver.reject(error),
            },
            Op::SettingsRead => match no_args(request.args) {
                Ok(()) => resolver.resolve(json!({ "values": package.settings })),
                Err(error) => resolver.reject(error),
            },
            Op::SurfaceReady => match no_args(request.args) {
                Ok(()) => match show_ready(&app) {
                    Ok(()) => resolver.resolve(json!({ "ready": true })),
                    Err(error) => resolver.reject(error),
                },
                Err(error) => resolver.reject(error),
            },
            Op::SurfaceClose => match no_args(request.args) {
                Ok(()) => {
                    destroy(&app, true);
                    resolver.resolve(json!({ "closed": true }));
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
                tauri::async_runtime::spawn(async move {
                    match net::get(&args.origin, &args.path, &args.query).await {
                        Ok(value) => resolver.resolve(value),
                        Err(error) => resolver.reject(error),
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
                if CONTROL
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    resolver.reject(failure(
                        "busy",
                        "A playback control is already in flight. Try again once it finishes.",
                    ));
                    return true;
                }
                tauri::async_runtime::spawn(async move {
                    let outcome = apply_control(app, command).await;
                    CONTROL.store(false, Ordering::Release);
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
        check_net_args, content_type, no_args, playback, permitted, relay, BridgeRequest, Op,
        MAX_TEXT,
    };
    use crate::remote::server::MusicState;
    use serde_json::{json, Value};

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
    fn only_navigates_to_the_extension_origin() {
        let allowed = |url: &str| url.parse::<tauri::Url>().is_ok_and(|url| permitted(&url));
        assert!(allowed("http://luma-extension-spike.localhost/"));
        assert!(allowed("http://luma-extension-spike.localhost/ui/page.js"));
        assert!(!allowed("https://luma-extension-spike.localhost/"));
        assert!(!allowed("http://luma-extension-spike.localhost.evil.com/"));
        assert!(!allowed("http://luma-extension-spike.localhost:8080/"));
        assert!(!allowed("http://user@luma-extension-spike.localhost/"));
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("file:///C:/Windows/System32/"));
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
            Op::SettingsRead.permission().map(|permission| permission.label()),
            Some("settings.read")
        );
        assert_eq!(
            Op::NetGet.permission().map(|permission| permission.label()),
            Some("network.get")
        );
        assert_eq!(
            Op::PlaybackRead.permission().map(|permission| permission.label()),
            Some("playback.read")
        );
        for op in [Op::PlaybackToggle, Op::PlaybackNext, Op::PlaybackPrevious] {
            assert_eq!(
                op.permission().map(|permission| permission.label()),
                Some("playback.control")
            );
        }
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
            assert!(relay(command).is_none(), "{command} must not reach the surface");
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
            snapshot["title"].as_str().map(|value| value.chars().count()),
            Some(MAX_TEXT)
        );
        assert_eq!(snapshot["artist"], "Artist");
    }
}
