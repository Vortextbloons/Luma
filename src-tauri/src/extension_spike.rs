mod weather;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{
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
const ACTION: &str = "extension_spike_action";
const ENV: &str = "LUMA_EXTENSION_SPIKE";
const PACKAGE: &str = "dev.luma.spike.now-playing";
const VERSION: &str = "0.1.0";
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src ipc: http://ipc.localhost; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
const MAX_TEXT: usize = 200;

const PAGE: &str = include_str!("extension_spike/page.html");
const SCRIPT: &str = include_str!("extension_spike/page.js");
const STYLE: &str = include_str!("extension_spike/page.css");

static PONGS: AtomicU64 = AtomicU64::new(0);
static STARTED: AtomicBool = AtomicBool::new(false);
static CONTROL: AtomicBool = AtomicBool::new(false);

fn enabled() -> bool {
    cfg!(debug_assertions) && matches!(std::env::var(ENV).as_deref(), Ok("1"))
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

fn serve(label: &str, path: &str) -> Response<Vec<u8>> {
    if label != LABEL {
        return response(
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            Vec::new(),
        );
    }
    let (body, content_type) = match path {
        "/" | "/index.html" => (PAGE.as_bytes().to_vec(), "text/html; charset=utf-8"),
        "/page.js" => (SCRIPT.as_bytes().to_vec(), "text/javascript; charset=utf-8"),
        "/page.css" => (STYLE.as_bytes().to_vec(), "text/css; charset=utf-8"),
        _ => {
            return response(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                Vec::new(),
            );
        }
    };
    response(StatusCode::OK, content_type, body)
}

fn permitted(url: &tauri::Url) -> bool {
    url.scheme() == "http" && url.host_str() == Some(HOST) && url.port().is_none()
}

pub(crate) fn protocol<R: tauri::Runtime>(
    context: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if !enabled() {
        return response(
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            Vec::new(),
        );
    }
    serve(context.webview_label(), request.uri().path())
}

pub(crate) fn open(app: &AppHandle) -> Result<(), String> {
    if !enabled() {
        return Err(format!(
            "The extension runtime spike requires a development build with {ENV}=1."
        ));
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let main = app
        .get_webview_window("main")
        .ok_or("Luma window is unavailable")?;
    let size = main.inner_size().map_err(|error| error.to_string())?;
    let position = main.outer_position().map_err(|error| error.to_string())?;
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let url = ORIGIN
        .parse()
        .map_err(|error| format!("Invalid spike origin: {error}"))?;
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::CustomProtocol(url))
        .title("Luma extension spike")
        .inner_size(560.0, 460.0)
        .position(
            position.x as f64 / scale + (size.width as f64 / scale - 560.0) / 2.0,
            position.y as f64 / scale + (size.height as f64 / scale - 460.0) / 2.0,
        )
        .min_inner_size(480.0, 360.0)
        .incognito(true)
        .visible(false)
        .focused(false)
        .on_navigation(permitted)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
        .map_err(|error| format!("Could not open the extension runtime spike: {error}"))?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            destroy(&handle, true);
        }
    });
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

fn destroy(app: &AppHandle, restore_focus: bool) {
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

pub(crate) fn active(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL).is_some_and(|window| {
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
    if let Err(error) = open(&app) {
        eprintln!("Extension runtime spike: {error}");
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WeatherRequest {
    action: WeatherAction,
    location: String,
    units: String,
}

#[derive(Deserialize)]
enum WeatherAction {
    #[serde(rename = "weather-read")]
    Read,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionRequest {
    action: Action,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    Identity,
    Ping,
    Playback,
    PlaybackToggle,
    PlaybackNext,
    PlaybackPrevious,
    Close,
}

impl Action {
    fn player_command(&self) -> Option<&'static str> {
        match self {
            Self::PlaybackToggle => Some("toggle"),
            Self::PlaybackNext => Some("next"),
            Self::PlaybackPrevious => Some("previous"),
            _ => None,
        }
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

fn perform(app: &AppHandle, action: Action) -> Result<Value, String> {
    match action {
        Action::Identity => Ok(json!({
            "package": PACKAGE,
            "version": VERSION,
            "runtime": "isolated-ui",
            "surface": "desktop",
            "window": LABEL,
            "origin": ORIGIN,
        })),
        Action::Ping => Ok(json!({
            "pongs": PONGS.fetch_add(1, Ordering::Relaxed) + 1,
            "source": "host",
        })),
        Action::Playback => Ok(playback_snapshot(app)),
        Action::Close => {
            destroy(app, true);
            Ok(json!({ "closed": true }))
        }
        Action::PlaybackToggle | Action::PlaybackNext | Action::PlaybackPrevious => {
            Err("Playback controls run asynchronously.".into())
        }
    }
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
            || message.command() != ACTION
        {
            resolver.reject("Extension spike command denied.");
            return true;
        }
        if let InvokeBody::Json(payload) = message.payload() {
            if payload.get("action").and_then(Value::as_str) == Some("weather-read") {
                let request = serde_json::from_value::<WeatherRequest>(payload.clone());
                let request = match request {
                    Ok(WeatherRequest {
                        action: WeatherAction::Read,
                        location,
                        units,
                    }) => weather::Request::new(&location, &units),
                    Err(_) => {
                        resolver.reject(json!({ "code": "invalid", "message": "Choose a supported city and temperature unit." }));
                        return true;
                    }
                };
                match request {
                    Ok(request) => {
                        tauri::async_runtime::spawn(async move {
                            match weather::read(request).await {
                                Ok(reading) => resolver.resolve(reading),
                                Err(error) => resolver.reject(error),
                            }
                        });
                    }
                    Err(error) => resolver.reject(error),
                }
                return true;
            }
        }
        let app = message.webview_ref().window_ref().app_handle().clone();
        let action = match message.payload() {
            InvokeBody::Json(payload) => serde_json::from_value::<ActionRequest>(payload.clone())
                .map(|request| request.action)
                .map_err(|_| "Invalid extension spike action.".to_string()),
            InvokeBody::Raw(_) => Err("Invalid extension spike action.".to_string()),
        };
        let action = match action {
            Ok(action) => action,
            Err(error) => {
                resolver.reject(error);
                return true;
            }
        };
        let Some(command) = action.player_command() else {
            match perform(&app, action) {
                Ok(value) => resolver.resolve(value),
                Err(error) => resolver.reject(error),
            }
            return true;
        };
        if CONTROL
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            resolver.reject("A playback control is already in flight. Try again once it finishes.");
            return true;
        }
        tauri::async_runtime::spawn(async move {
            let outcome = apply_control(app, command).await;
            CONTROL.store(false, Ordering::Release);
            match outcome {
                Ok(()) => resolver.resolve(json!({ "accepted": true })),
                Err(error) => resolver.reject(error),
            }
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        permitted, playback, relay, serve, Action, ActionRequest, LABEL, MAX_TEXT, SCRIPT,
    };
    use crate::remote::server::MusicState;
    use serde_json::json;
    use tauri::http::{header, StatusCode};

    fn parsed(payload: serde_json::Value) -> Result<Action, serde_json::Error> {
        serde_json::from_value::<ActionRequest>(payload).map(|request| request.action)
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
    fn weather_requests_reject_extra_authority_and_missing_selection() {
        let valid = json!({ "action": "weather-read", "location": "seattle", "units": "c" });
        assert!(serde_json::from_value::<super::WeatherRequest>(valid.clone()).is_ok());
        for field in ["url", "latitude", "headers", "package", "token"] {
            let mut payload = valid.clone();
            payload[field] = json!("untrusted");
            assert!(serde_json::from_value::<super::WeatherRequest>(payload).is_err());
        }
        assert!(serde_json::from_value::<super::WeatherRequest>(
            json!({ "action": "weather-read" })
        )
        .is_err());
        assert!(serde_json::from_value::<super::WeatherRequest>(
            json!({ "action": "playback", "location": "seattle", "units": "c" })
        )
        .is_err());
    }

    #[test]
    fn serves_the_spike_page_only_to_the_spike_window() {
        assert_eq!(serve("main", "/").status(), StatusCode::NOT_FOUND);
        assert_eq!(serve("youtube", "/page.js").status(), StatusCode::NOT_FOUND);
        assert_eq!(serve(LABEL, "/").status(), StatusCode::OK);
        assert_eq!(serve(LABEL, "/page.js").status(), StatusCode::OK);
        assert_eq!(serve(LABEL, "/page.css").status(), StatusCode::OK);
        assert_eq!(
            serve(LABEL, "/../Cargo.toml").status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn serves_the_spike_page_with_a_restrictive_content_security_policy() {
        let response = serve(LABEL, "/");
        let policy = response
            .headers()
            .get("content-security-policy")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert!(policy.contains("default-src 'none'"));
        assert!(policy.contains("script-src 'self'"));
        assert!(policy.contains("connect-src ipc: http://ipc.localhost"));
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
    }

    #[test]
    fn only_navigates_to_the_spike_origin() {
        let allowed = |url: &str| url.parse::<tauri::Url>().is_ok_and(|url| permitted(&url));
        assert!(allowed("http://luma-extension-spike.localhost/"));
        assert!(allowed("http://luma-extension-spike.localhost/page.js"));
        assert!(!allowed("https://luma-extension-spike.localhost/"));
        assert!(!allowed("http://luma-extension-spike.localhost.evil.com/"));
        assert!(!allowed("http://luma-extension-spike.localhost:8080/"));
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("file:///C:/Windows/System32/"));
    }

    #[test]
    fn rejects_actions_the_page_may_not_perform() {
        assert!(matches!(
            parsed(serde_json::json!({ "action": "identity" })),
            Ok(Action::Identity)
        ));
        assert!(matches!(
            parsed(serde_json::json!({ "action": "ping" })),
            Ok(Action::Ping)
        ));
        assert!(matches!(
            parsed(serde_json::json!({ "action": "playback" })),
            Ok(Action::Playback)
        ));
        assert!(matches!(
            parsed(serde_json::json!({ "action": "close" })),
            Ok(Action::Close)
        ));
        assert!(matches!(
            parsed(serde_json::json!({ "action": "playback-toggle" })),
            Ok(Action::PlaybackToggle)
        ));
        assert!(matches!(
            parsed(serde_json::json!({ "action": "playback-next" })),
            Ok(Action::PlaybackNext)
        ));
        assert!(matches!(
            parsed(serde_json::json!({ "action": "playback-previous" })),
            Ok(Action::PlaybackPrevious)
        ));
        assert!(parsed(serde_json::json!({ "action": "install" })).is_err());
        assert!(
            parsed(serde_json::json!({ "action": "identity", "package": "com.example" })).is_err()
        );
        assert!(
            parsed(serde_json::json!({ "action": "playback", "device": "pc" })).is_err(),
            "playback actions must not carry arguments"
        );
        assert!(
            parsed(serde_json::json!({ "action": "playback-next", "value": "1" })).is_err(),
            "playback controls must not carry arguments"
        );
        assert!(parsed(serde_json::json!({ "action": true })).is_err());
        assert!(parsed(serde_json::json!({})).is_err());
    }

    #[test]
    fn playback_controls_map_only_to_the_existing_player_commands() {
        for (name, command) in [
            ("playback-toggle", "toggle"),
            ("playback-next", "next"),
            ("playback-previous", "previous"),
        ] {
            let action = parsed(json!({ "action": name })).expect("control action parses");
            assert_eq!(action.player_command(), Some(command));
            for extra in [
                json!({ "action": name, "command": "play" }),
                json!({ "action": name, "value": "next" }),
                json!({ "action": name, "device": "pc" }),
                json!({ "action": name, "uri": "spotify:track:1" }),
                json!({ "action": name, "args": {} }),
            ] {
                assert!(parsed(extra).is_err(), "{name} must not accept arguments");
            }
        }
        for name in ["identity", "ping", "playback", "close"] {
            let action = parsed(json!({ "action": name })).expect("local action parses");
            assert_eq!(
                action.player_command(),
                None,
                "{name} must not dispatch a player control"
            );
        }
    }

    #[test]
    fn rejects_playback_actions_outside_the_bounded_control_set() {
        for name in [
            "playback-play",
            "playback-pause",
            "playback-seek",
            "playback-shuffle",
            "playback-repeat",
            "playback-volume",
            "playback-transfer",
            "playback-play-uri",
            "playback-device",
            "playback-token",
            "toggle",
            "next",
            "previous",
            "spotify-control",
            "spotify-connect",
            "playback-toggle-now",
            "playback_toggle",
            "PlaybackToggle",
            "",
        ] {
            assert!(
                parsed(json!({ "action": name })).is_err(),
                "{name} must stay outside the bounded control set"
            );
        }
    }

    #[test]
    fn relays_only_bounded_navigation_keys_to_the_spike_page() {
        for (command, key) in [
            ("up", "ArrowUp"),
            ("down", "ArrowDown"),
            ("left", "ArrowLeft"),
            ("right", "ArrowRight"),
        ] {
            let script = relay(command).expect("directions relay");
            assert!(script.contains(key));
            assert!(
                SCRIPT.contains(key),
                "the spike page no longer handles {key}"
            );
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
                "{command} must not reach the spike page"
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
        assert_eq!(blank["title"], serde_json::Value::Null);
        assert_eq!(blank["artist"], serde_json::Value::Null);
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
}
