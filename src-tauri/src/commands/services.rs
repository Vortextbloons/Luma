use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as AsyncMutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

const SCRIPT: &str = include_str!("../../resources/services/inject.js");
const WINDOW_TIMEOUT: Duration = Duration::from_secs(8);
const CDP_TIMEOUT: Duration = Duration::from_secs(2);
const LOOP_WAIT: Duration = Duration::from_millis(140);

pub(crate) fn sync_cursor(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Services>();
        let mut connections = state.connections.lock().await;
        let script = if crate::remote::pointer::cursor_hidden(&app) {
            crate::remote::pointer::HIDE
        } else {
            crate::remote::pointer::SHOW
        };
        for cdp in connections.values_mut() {
            let _ = cdp.eval(script).await;
        }
    });
}

#[derive(Default)]
pub struct Services {
    inner: Mutex<Inner>,
    connections: AsyncMutex<HashMap<String, Cdp>>,
}

#[derive(Default)]
struct Inner {
    windows: HashMap<String, ServiceWindow>,
}

#[derive(Clone)]
struct ServiceWindow {
    id: isize,
    url: String,
}

enum Outcome {
    Handled,
    Return,
}

struct Cdp {
    socket: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    next: i64,
}

#[derive(Deserialize)]
struct Target {
    #[serde(rename = "type")]
    target_type: String,
    url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    socket: Option<String>,
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .no_proxy()
            .build()
            .unwrap_or_default()
    })
}

fn edge() -> Option<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
        .iter()
        .filter_map(|variable| {
            std::env::var_os(variable)
                .map(|root| PathBuf::from(root).join(r"Microsoft\Edge\Application\msedge.exe"))
        })
        .find(|candidate| candidate.is_file())
}

fn profile_slug(name: &str) -> String {
    name.to_lowercase().replace('+', "plus").replace(' ', "-")
}

fn profile_dir(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("services")
        .join(profile_slug(name)))
}

fn edge_arguments(profile: &Path, url: &str, fullscreen: bool) -> Vec<String> {
    let mut arguments = vec![
        format!("--app={url}"),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--disk-cache-size=67108864".into(),
        "--no-default-browser-check".into(),
        "--enable-spatial-navigation".into(),
        "--remote-debugging-address=127.0.0.1".into(),
        "--remote-debugging-port=0".into(),
    ];
    arguments.push(if fullscreen {
        "--start-fullscreen".into()
    } else {
        "--start-maximized".into()
    });
    arguments
}

fn normalized_path(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return None;
    }
    let path = parsed.path().trim_end_matches('/');
    Some(if path.is_empty() {
        "/".into()
    } else {
        path.into()
    })
}

pub(crate) fn at_home(name: &str, url: &str) -> bool {
    let Some(path) = normalized_path(url) else {
        return false;
    };
    match name {
        "Netflix" => path == "/browse" || path == "/",
        "Disney+" => path == "/" || path == "/home",
        "Prime Video" => path == "/" || path == "/home" || path == "/storefront",
        "Apple TV+" => path == "/" || path.ends_with("/home"),
        "Hulu" | "Max" | "Peacock" => path == "/" || path == "/home",
        _ => path == "/",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Back {
    Home,
    Back,
}

fn back_target(index: Option<i64>, length: i64, home: bool) -> Back {
    if home || index.is_some_and(|value| value <= 0) || (index.is_none() && length <= 1) {
        Back::Home
    } else {
        Back::Back
    }
}

fn target_host(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
}

fn same_site(candidate: &str, expected: &str) -> bool {
    candidate == expected
        || candidate.ends_with(&format!(".{expected}"))
        || expected.ends_with(&format!(".{candidate}"))
}

fn pick_target(targets: &[Target], host: Option<&str>) -> Option<String> {
    let page = |target: &Target| {
        target.target_type == "page"
            && target.socket.is_some()
            && (target.url.starts_with("http://") || target.url.starts_with("https://"))
    };
    targets
        .iter()
        .find(|target| {
            page(target)
                && host.is_some_and(|host| {
                    target_host(&target.url).is_some_and(|candidate| same_site(&candidate, host))
                })
        })
        .or_else(|| targets.iter().find(|target| page(target)))
        .and_then(|target| target.socket.clone())
}

fn remember(app: &AppHandle, name: &str, id: isize, url: &str) {
    if let Ok(mut inner) = app.state::<Services>().inner.lock() {
        inner.windows.insert(
            name.into(),
            ServiceWindow {
                id,
                url: url.into(),
            },
        );
    }
}

fn tracked(app: &AppHandle, name: &str) -> Option<ServiceWindow> {
    app.state::<Services>()
        .inner
        .lock()
        .ok()
        .and_then(|inner| inner.windows.get(name).cloned())
}

pub(crate) fn window_source(app: &AppHandle, id: isize) -> Option<String> {
    app.state::<Services>().inner.lock().ok()?.windows.iter().find(|(_, window)| window.id == id).map(|(name, _)| name.clone())
}

pub(crate) async fn evaluate(app: &AppHandle, name: &str, script: &str) -> Result<Value, String> {
    let url = tracked(app, name).ok_or("That service has closed.")?.url;
    ensure(app, name, &url).await?;
    let state = app.state::<Services>();
    let mut connections = state.connections.lock().await;
    let result = connections.get_mut(name).ok_or("Player unavailable")?.timeline(script).await;
    if result.is_err() { connections.remove(name); }
    result
}

pub(crate) fn reuse(app: &AppHandle, name: &str, url: &str) -> bool {
    let id = tracked(app, name)
        .map(|window| window.id)
        .filter(|id| {
            crate::commands::session::window_alive(*id)
                && crate::commands::session::window_browser(*id)
        })
        .or_else(|| {
            crate::commands::session::find_window(name)
                .filter(|id| crate::commands::session::window_browser(*id))
        });
    let Some(id) = id else {
        return false;
    };
    remember(app, name, id, url);
    let pid = crate::commands::session::window_pid(id).unwrap_or(0);
    crate::commands::session::track_web(app, name, pid);
    crate::commands::session::restore_window(id);
    let handle = app.clone();
    let service = name.to_string();
    let target = url.to_string();
    tauri::async_runtime::spawn(async move {
        let _ = ensure(&handle, &service, &target).await;
    });
    true
}

pub async fn launch(app: &AppHandle, name: &str, url: &str) -> Result<(), String> {
    crate::viewing::check_capacity(app, name)?;
    if reuse(app, name, url) {
        crate::activity::record_launch(app, name);
        return minimize_main(app);
    }
    let browser = edge().ok_or_else(|| {
        format!("Microsoft Edge is required to open {name}. Install Edge or choose a program under App locations in Settings.")
    })?;
    let profile = profile_dir(app, name)?;
    std::fs::create_dir_all(&profile)
        .map_err(|error| format!("Could not prepare {name}: {error}"))?;
    let _ = std::fs::remove_file(profile.join("DevToolsActivePort"));
    let fullscreen = app
        .get_webview_window("main")
        .and_then(|window| window.is_fullscreen().ok())
        .unwrap_or(false) && !crate::viewing::enabled(app);
    let mut command = std::process::Command::new(&browser);
    if let Some(parent) = browser.parent() {
        command.current_dir(parent);
    }
    let mut child = command
        .args(edge_arguments(&profile, url, fullscreen))
        .spawn()
        .map_err(|error| format!("Could not start {name}: {error}"))?;
    let id = match wait_window(child.id(), name).await {
        Ok(id) => id,
        Err(error) => {
            let _ = child.kill();
            return Err(error);
        }
    };
    let pid = crate::commands::session::window_pid(id).unwrap_or(child.id());
    remember(app, name, id, url);
    crate::commands::session::track_web(app, name, pid);
    crate::activity::record_launch(app, name);
    crate::commands::session::restore_window(id);
    let handle = app.clone();
    let service = name.to_string();
    let target = url.to_string();
    tauri::async_runtime::spawn(async move {
        let _ = ensure(&handle, &service, &target).await;
    });
    minimize_main(app)
}

fn minimize_main(app: &AppHandle) -> Result<(), String> {
    if crate::viewing::enabled(app) { return Ok(()); }
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|error| error.to_string())?;
        crate::background::publish_visibility(&window, false);
    }
    Ok(())
}

async fn wait_window(pid: u32, name: &str) -> Result<isize, String> {
    let deadline = tokio::time::Instant::now() + WINDOW_TIMEOUT;
    loop {
        if let Some(id) = crate::commands::session::find_window_for(pid, name) {
            return Ok(id);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{name} did not open. Check that Microsoft Edge is installed and try again."
            ));
        }
        tokio::time::sleep(LOOP_WAIT).await;
    }
}

async fn ensure(app: &AppHandle, name: &str, url: &str) -> Result<(), String> {
    let state = app.state::<Services>();
    let mut connections = state.connections.lock().await;
    if !connections.contains_key(name) {
        connections.insert(name.into(), connect(app, name, url).await?);
    }
    Ok(())
}

pub(crate) fn forward(app: &AppHandle, name: &str, command: &str) -> bool {
    let app = app.clone();
    let name = name.to_string();
    let command = command.to_string();
    tauri::async_runtime::spawn(async move {
        match dispatch(&app, &name, &command).await {
            Ok(Outcome::Return) => { if !crate::viewing::return_home(&app) { crate::commands::session::finish(&app, &name); } },
            Ok(Outcome::Handled) => {}
            Err(_) => {
                if crate::viewing::external_selected(&app) {
                    if let Some(window) = tracked(&app, &name) { let _ = crate::commands::session::press_window(window.id, &command); }
                } else { crate::commands::session::press(&app, &name, &command); }
            },
        }
    });
    true
}

async fn dispatch(app: &AppHandle, name: &str, command: &str) -> Result<Outcome, String> {
    let url = tracked(app, name)
        .map(|window| window.url)
        .unwrap_or_default();
    let state = app.state::<Services>();
    let mut connections = state.connections.lock().await;
    if !crate::commands::session::web_current(app, name) {
        return Ok(Outcome::Handled);
    }
    if !connections.contains_key(name) {
        connections.insert(name.into(), connect(app, name, &url).await?);
    }
    if !crate::commands::session::web_current(app, name) {
        connections.remove(name);
        return Ok(Outcome::Handled);
    }
    let cdp = connections
        .get_mut(name)
        .ok_or("The service window is unavailable.")?;
    let script = if crate::remote::pointer::cursor_hidden(app) {
        crate::remote::pointer::HIDE
    } else {
        crate::remote::pointer::SHOW
    };
    cdp.eval(script).await?;
    let result = handle(cdp, name, command).await;
    if result.is_err() {
        connections.remove(name);
    }
    result
}

async fn handle(cdp: &mut Cdp, name: &str, command: &str) -> Result<Outcome, String> {
    match command {
        "back" => {
            let state = cdp
                .eval("(() => { const state = window.__lumaBackState ? window.__lumaBackState() : { index: null, length: history.length }; return { ...state, href: location.href }; })()")
                .await?;
            let index = state.get("index").and_then(Value::as_i64);
            let length = state.get("length").and_then(Value::as_i64).unwrap_or(1);
            let href = state.get("href").and_then(Value::as_str).unwrap_or("");
            if back_target(index, length, at_home(name, href)) == Back::Home {
                return Ok(Outcome::Return);
            }
            cdp.eval("history.back(); true").await?;
            Ok(Outcome::Handled)
        }
        "up" | "down" | "left" | "right" | "activate" => {
            let serialized = serde_json::to_string(command).map_err(|error| error.to_string())?;
            let expression =
                format!("window.__lumaNavigate ? window.__lumaNavigate({serialized}) : false");
            if cdp.eval(&expression).await?.as_bool() == Some(true) {
                return Ok(Outcome::Handled);
            }
            cdp.press(command).await?;
            Ok(Outcome::Handled)
        }
        _ => Ok(Outcome::Handled),
    }
}

pub(crate) fn forget(app: &AppHandle, name: &str) {
    if let Ok(mut inner) = app.state::<Services>().inner.lock() {
        inner.windows.remove(name);
    }
    if let Ok(mut connections) = app.state::<Services>().connections.try_lock() {
        connections.remove(name);
    } else {
        let app = app.clone();
        let name = name.to_string();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<Services>();
            let mut connections = state.connections.lock().await;
            if !crate::commands::session::web_current(&app, &name) {
                connections.remove(&name);
            }
        });
    }
}

async fn connect(app: &AppHandle, name: &str, url: &str) -> Result<Cdp, String> {
    let profile = profile_dir(app, name)?;
    let port = read_port(&profile).await?;
    let host = target_host(url);
    let target = page_target(port, host.as_deref()).await?;
    let (socket, _) = tokio::time::timeout(CDP_TIMEOUT, connect_async(target.as_str()))
        .await
        .map_err(|_| "Timed out connecting to the service window.".to_string())?
        .map_err(|error| error.to_string())?;
    let mut cdp = Cdp { socket, next: 0 };
    cdp.call("Page.enable", json!({})).await?;
    cdp.call(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": SCRIPT }),
    )
    .await?;
    let _ = cdp.eval(SCRIPT).await;
    Ok(cdp)
}

async fn read_port(profile: &Path) -> Result<u16, String> {
    for _ in 0..6 {
        if let Ok(text) = std::fs::read_to_string(profile.join("DevToolsActivePort")) {
            if let Some(port) = text
                .lines()
                .next()
                .and_then(|line| line.trim().parse::<u16>().ok())
            {
                return Ok(port);
            }
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    Err("The service window did not enable remote focus control.".into())
}

async fn page_target(port: u16, host: Option<&str>) -> Result<String, String> {
    for _ in 0..10 {
        if let Ok(response) = client()
            .get(format!("http://127.0.0.1:{port}/json/list"))
            .send()
            .await
        {
            if let Ok(targets) = response.json::<Vec<Target>>().await {
                if let Some(socket) = pick_target(&targets, host) {
                    return Ok(socket);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(140)).await;
    }
    Err("The service page is not ready for remote focus control.".into())
}

impl Cdp {
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        let payload = json!({ "id": id, "method": method, "params": params }).to_string();
        tokio::time::timeout(CDP_TIMEOUT, self.socket.send(Message::Text(payload.into())))
            .await
            .map_err(|_| "The service window stopped responding.".to_string())?
            .map_err(|error| error.to_string())?;
        let deadline = tokio::time::Instant::now() + CDP_TIMEOUT;
        loop {
            let message = tokio::time::timeout_at(deadline, self.socket.next())
                .await
                .map_err(|_| "The service window stopped responding.".to_string())?;
            let Some(message) = message else {
                return Err("The service window closed.".into());
            };
            let message = message.map_err(|error| error.to_string())?;
            let text = match message {
                Message::Text(text) => text.as_str().to_string(),
                Message::Binary(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Message::Close(_) => return Err("The service window closed.".into()),
                _ => continue,
            };
            let Ok(value) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if value.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("The service page rejected the command.")
                    .to_string());
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn eval(&mut self, expression: &str) -> Result<Value, String> {
        let result = self
            .call(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                    "userGesture": true
                }),
            )
            .await?;
        if result.get("exceptionDetails").is_some() {
            return Err("The service page could not run Luma's navigation.".into());
        }
        Ok(result
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null))
    }

    async fn timeline(&mut self, expression: &str) -> Result<Value, String> {
        let result = self.eval(expression).await?;
        if !result.is_null() {
            return Ok(result);
        }
        let tree = self.call("Page.getFrameTree", json!({})).await?;
        let mut frames: Vec<&Value> = tree
            .pointer("/frameTree/childFrames")
            .and_then(Value::as_array)
            .map(|frames| frames.iter().collect())
            .unwrap_or_default();
        while let Some(frame) = frames.pop() {
            if let Some(children) = frame.get("childFrames").and_then(Value::as_array) {
                frames.extend(children);
            }
            let Some(id) = frame.pointer("/frame/id").and_then(Value::as_str) else {
                continue;
            };
            let world = self
                .call(
                    "Page.createIsolatedWorld",
                    json!({ "frameId": id, "worldName": "luma-playback" }),
                )
                .await?;
            let Some(context) = world.get("executionContextId") else {
                continue;
            };
            let result = self.call("Runtime.evaluate", json!({
                "expression": expression, "contextId": context, "returnByValue": true, "awaitPromise": true, "userGesture": true,
            })).await?;
            if result.get("exceptionDetails").is_some() {
                continue;
            }
            let value = result
                .pointer("/result/value")
                .cloned()
                .unwrap_or(Value::Null);
            if !value.is_null() {
                return Ok(value);
            }
        }
        Ok(Value::Null)
    }

    async fn press(&mut self, command: &str) -> Result<(), String> {
        let (key, code, key_code, arrow) = match command {
            "up" => ("ArrowUp", "ArrowUp", 38, true),
            "down" => ("ArrowDown", "ArrowDown", 40, true),
            "left" => ("ArrowLeft", "ArrowLeft", 37, true),
            "right" => ("ArrowRight", "ArrowRight", 39, true),
            "activate" => ("Enter", "Enter", 13, false),
            _ => return Ok(()),
        };
        let mut down = json!({
            "type": if arrow { "rawKeyDown" } else { "keyDown" },
            "key": key,
            "code": code,
            "windowsVirtualKeyCode": key_code,
            "nativeVirtualKeyCode": key_code
        });
        if !arrow {
            down["text"] = json!("\r");
        }
        self.call("Input.dispatchKeyEvent", down).await?;
        self.call(
            "Input.dispatchKeyEvent",
            json!({
                "type": "keyUp",
                "key": key,
                "code": code,
                "windowsVirtualKeyCode": key_code,
                "nativeVirtualKeyCode": key_code
            }),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn playback_reads_embedded_players_through_their_frame_context() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            for expected in [
                "Runtime.evaluate",
                "Page.getFrameTree",
                "Page.createIsolatedWorld",
                "Runtime.evaluate",
            ] {
                let text = socket.next().await.unwrap().unwrap().into_text().unwrap();
                let request: Value = serde_json::from_str(&text).unwrap();
                assert_eq!(request["method"], expected);
                if expected == "Runtime.evaluate" {
                    assert_eq!(request["params"]["awaitPromise"], true);
                }
                let result = match expected {
                    "Page.getFrameTree" => {
                        json!({ "frameTree": { "childFrames": [{ "frame": { "id": "movie" } }] } })
                    }
                    "Page.createIsolatedWorld" => {
                        assert_eq!(request["params"]["frameId"], "movie");
                        json!({ "executionContextId": 42 })
                    }
                    _ if request["params"].get("contextId").is_some() => {
                        assert_eq!(request["params"]["contextId"], 42);
                        assert_eq!(request["params"]["expression"], "seek(3600)");
                        json!({ "result": { "value": { "position": 3600, "duration": 7200 } } })
                    }
                    _ => json!({ "result": { "value": null } }),
                };
                socket
                    .send(Message::Text(
                        json!({ "id": request["id"], "result": result })
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
            }
        });
        let (socket, _) = connect_async(format!("ws://{address}")).await.unwrap();
        let mut cdp = Cdp { socket, next: 0 };
        let result = cdp.timeline("seek(3600)").await.unwrap();
        assert_eq!(result, json!({ "position": 3600, "duration": 7200 }));
        server.await.unwrap();
    }

    #[test]
    fn service_home_pages_are_recognized() {
        assert!(at_home("Netflix", "https://www.netflix.com/browse"));
        assert!(at_home("Netflix", "https://www.netflix.com/"));
        assert!(!at_home(
            "Netflix",
            "https://www.netflix.com/title/80100172"
        ));
        assert!(at_home("Disney+", "https://www.disneyplus.com/home"));
        assert!(at_home(
            "Prime Video",
            "https://www.primevideo.com/storefront"
        ));
        assert!(!at_home(
            "Prime Video",
            "https://www.primevideo.com/detail/0ABC"
        ));
        assert!(at_home("Apple TV+", "https://tv.apple.com/us/home"));
        assert!(at_home("Hulu", "https://www.hulu.com/home"));
        assert!(!at_home("Hulu", "https://www.hulu.com/series/example"));
        assert!(!at_home("Netflix", "edge://settings"));
    }

    #[test]
    fn back_returns_to_luma_only_at_the_start_of_the_service() {
        assert_eq!(back_target(Some(0), 5, false), Back::Home);
        assert_eq!(back_target(Some(2), 5, false), Back::Back);
        assert_eq!(back_target(Some(2), 5, true), Back::Home);
        assert_eq!(back_target(None, 1, false), Back::Home);
        assert_eq!(back_target(None, 2, false), Back::Back);
    }

    #[test]
    fn cdp_target_prefers_the_service_page() {
        let targets = vec![
            Target {
                target_type: "page".into(),
                url: "edge://newtab/".into(),
                socket: Some("ws://newtab".into()),
            },
            Target {
                target_type: "page".into(),
                url: "https://www.netflix.com/browse".into(),
                socket: Some("ws://netflix".into()),
            },
            Target {
                target_type: "service_worker".into(),
                url: "https://www.netflix.com/sw.js".into(),
                socket: Some("ws://worker".into()),
            },
        ];
        assert_eq!(
            pick_target(&targets, Some("www.netflix.com")).as_deref(),
            Some("ws://netflix")
        );
        assert_eq!(
            pick_target(&targets, Some("example.com")).as_deref(),
            Some("ws://netflix")
        );
        assert_eq!(pick_target(&targets[..1], Some("www.netflix.com")), None);
        assert_eq!(pick_target(&[], None), None);
    }

    #[test]
    fn service_profiles_keep_existing_sign_ins() {
        assert_eq!(profile_slug("Netflix"), "netflix");
        assert_eq!(profile_slug("Disney+"), "disneyplus");
        assert_eq!(profile_slug("Prime Video"), "prime-video");
        assert_eq!(profile_slug("Apple TV+"), "apple-tvplus");
    }

    #[test]
    fn edge_arguments_keep_each_service_isolated_and_tv_ready() {
        let profile =
            Path::new(r"C:\Users\viewer\AppData\Roaming\com.luma.desktop\services\netflix");
        let arguments = edge_arguments(profile, "https://www.netflix.com/browse", true);
        assert!(arguments
            .iter()
            .any(|argument| argument.as_str() == "--app=https://www.netflix.com/browse"));
        assert!(arguments
            .iter()
            .any(|argument| argument.as_str() == "--enable-spatial-navigation"));
        assert!(arguments.iter().any(|argument| argument == "--disk-cache-size=67108864"));
        assert!(arguments
            .iter()
            .any(|argument| argument.as_str() == "--remote-debugging-port=0"));
        assert!(arguments
            .iter()
            .any(|argument| argument.as_str() == "--start-fullscreen"));
        assert!(arguments
            .iter()
            .any(|argument| argument == &format!("--user-data-dir={}", profile.display())));
        assert!(!arguments
            .iter()
            .any(|argument| argument.contains("netflix.com/title")));
        assert!(
            edge_arguments(profile, "https://www.netflix.com/browse", false)
                .iter()
                .any(|argument| argument.as_str() == "--start-maximized")
        );
    }
}

pub(crate) fn current_source(app: &AppHandle) -> Option<String> {
    let state = app.state::<Services>();
    let names: Vec<String> = state.inner.lock().ok()?.windows.keys().cloned().collect();
    names.into_iter().find(|name| crate::commands::session::web_current(app, name))
}
