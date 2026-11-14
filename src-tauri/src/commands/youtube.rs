use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const URL: &str = "https://www.youtube.com/tv";
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Cobalt/27.lts.3.1040976-gold (unlike Gecko) v8/13.8.258.54-jit gles Starboard/18, Luma_DESKTOP_x8664_2026/0.1.0 (Luma, Windows) com.luma.desktop/0.1.0";
const SCRIPT: &str = include_str!("../../resources/youtube/inject.js");

#[derive(Default)]
pub struct Bridge {
    url: tokio::sync::OnceCell<String>,
    window: tokio::sync::Mutex<()>,
}

#[derive(Deserialize)]
struct VideoQuery {
    #[serde(rename = "videoID")]
    video_id: String,
}

fn checked_id(value: &str) -> Result<&str, StatusCode> {
    if value.len() == 11
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Ok(value)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn origin_allowed(headers: &HeaderMap) -> bool {
    headers
        .get("origin")
        .and_then(|value| value.to_str().ok())
        == Some(YOUTUBE_ORIGIN)
}

fn reply(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header("access-control-allow-origin", YOUTUBE_ORIGIN)
        .header("access-control-allow-private-network", "true")
        .header("vary", "origin")
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default()
    })
}

async fn sponsorblock(
    headers: HeaderMap,
    Query(query): Query<VideoQuery>,
) -> Response {
    if !origin_allowed(&headers) {
        return reply(StatusCode::FORBIDDEN, "application/json", b"[]".to_vec());
    }
    let Ok(id) = checked_id(&query.video_id) else {
        return reply(StatusCode::BAD_REQUEST, "application/json", b"[]".to_vec());
    };
    let mut url = reqwest::Url::parse("https://sponsor.ajay.app/api/skipSegments")
        .expect("static sponsorblock URL");
    url.query_pairs_mut()
        .append_pair("videoID", id)
        .append_pair("service", "YouTube")
        .append_pair(
            "categories",
            r#"["sponsor","selfpromo","interaction","intro","outro","preview","hook","filler"]"#,
        );
    match client().get(url).send().await {
        Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
            reply(StatusCode::OK, "application/json", b"[]".to_vec())
        }
        Ok(response) if response.status().is_success() => match response.bytes().await {
            Ok(body) => reply(StatusCode::OK, "application/json", body.to_vec()),
            Err(_) => reply(StatusCode::BAD_GATEWAY, "application/json", b"[]".to_vec()),
        },
        _ => reply(StatusCode::BAD_GATEWAY, "application/json", b"[]".to_vec()),
    }
}

async fn dearrow(headers: HeaderMap, Query(query): Query<VideoQuery>) -> Response {
    if !origin_allowed(&headers) {
        return reply(StatusCode::FORBIDDEN, "application/json", b"null".to_vec());
    }
    let Ok(id) = checked_id(&query.video_id) else {
        return reply(StatusCode::BAD_REQUEST, "application/json", b"null".to_vec());
    };
    let mut url = reqwest::Url::parse("https://sponsor.ajay.app/api/branding")
        .expect("static dearrow URL");
    url.query_pairs_mut().append_pair("videoID", id);
    match client().get(url).timeout(std::time::Duration::from_secs(3)).send().await {
        Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
            reply(StatusCode::OK, "application/json", b"null".to_vec())
        }
        Ok(response) if response.status().is_success() => match response.bytes().await {
            Ok(body) => reply(StatusCode::OK, "application/json", body.to_vec()),
            Err(_) => reply(StatusCode::BAD_GATEWAY, "application/json", b"null".to_vec()),
        },
        _ => reply(StatusCode::BAD_GATEWAY, "application/json", b"null".to_vec()),
    }
}

async fn dearrow_thumbnail(headers: HeaderMap, Query(query): Query<VideoQuery>) -> Response {
    if !origin_allowed(&headers) {
        return reply(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    }
    let Ok(id) = checked_id(&query.video_id) else {
        return reply(StatusCode::BAD_REQUEST, "text/plain", b"Invalid video ID".to_vec());
    };
    let mut url = reqwest::Url::parse("https://dearrow-thumb.ajay.app/api/v1/getThumbnail")
        .expect("static dearrow thumbnail URL");
    url.query_pairs_mut().append_pair("videoID", id);
    match client().get(url).timeout(std::time::Duration::from_secs(3)).send().await {
        Ok(response) if response.status().is_success() => {
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();
            match response.bytes().await {
                Ok(body) => reply(StatusCode::OK, &content_type, body.to_vec()),
                Err(_) => reply(StatusCode::BAD_GATEWAY, "text/plain", Vec::new()),
            }
        }
        Ok(_) => reply(
            StatusCode::NO_CONTENT,
            "text/plain",
            Vec::new(),
        ),
        Err(_) => reply(StatusCode::NO_CONTENT, "text/plain", Vec::new()),
    }
}

async fn youtube_exit(State(app): State<AppHandle>, headers: HeaderMap) -> Response {
    if !origin_allowed(&headers) {
        return reply(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    }
    close(&app);
    reply(StatusCode::NO_CONTENT, "text/plain", Vec::new())
}

async fn playback(
    State(app): State<AppHandle>,
    headers: HeaderMap,
    Json(report): Json<crate::activity::models::PlaybackReport>,
) -> Response {
    if !origin_allowed(&headers) {
        return reply(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    }
    crate::activity::record_youtube(&app, &report);
    reply(StatusCode::NO_CONTENT, "text/plain", Vec::new())
}

async fn preflight(headers: HeaderMap) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    if origin_allowed(&headers) {
        let headers = response.headers_mut();
        headers.insert(
            "access-control-allow-origin",
            HeaderValue::from_static(YOUTUBE_ORIGIN),
        );
        headers.insert(
            "access-control-allow-methods",
            HeaderValue::from_static("GET, POST, OPTIONS"),
        );
        headers.insert(
            "access-control-allow-headers",
            HeaderValue::from_static("content-type"),
        );
        headers.insert(
            "access-control-allow-private-network",
            HeaderValue::from_static("true"),
        );
        headers.insert("access-control-max-age", HeaderValue::from_static("600"));
    }
    response
}

async fn start_bridge(app: AppHandle) -> Result<String, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("Cannot start the YouTube bridge: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("Cannot read the YouTube bridge address: {error}"))?
        .port();
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).map_err(|error| format!("Cannot secure the YouTube bridge: {error}"))?;
    let token: String = secret.iter().map(|byte| format!("{byte:02x}")).collect();
    let router = Router::new()
        .route(&format!("/{token}/sponsorblock"), get(sponsorblock))
        .route(&format!("/{token}/dearrow"), get(dearrow))
        .route(
            &format!("/{token}/dearrow-thumbnail"),
            get(dearrow_thumbnail),
        )
        .route(&format!("/{token}/exit"), get(youtube_exit))
        .route(&format!("/{token}/playback"), post(playback))
        .fallback(preflight)
        .layer(DefaultBodyLimit::max(8192))
        .with_state(app);
    tauri::async_runtime::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("YouTube bridge stopped: {error}");
        }
    });
    Ok(format!("http://127.0.0.1:{port}/{token}"))
}

async fn bridge(app: &AppHandle) -> Result<String, String> {
    app.state::<Bridge>()
        .url
        .get_or_try_init(|| start_bridge(app.clone()))
        .await
        .cloned()
}

fn allowed(host: Option<&str>) -> bool {
    let Some(host) = host else { return true };
    [
        "youtube.com",
        "youtu.be",
        "google.com",
        "gstatic.com",
        "ytimg.com",
        "ggpht.com",
        "googlevideo.com",
        "youtube-nocookie.com",
    ]
    .iter()
    .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

fn script(bridge: &str) -> Result<String, String> {
    Ok(format!(
        "window.__LUMA_UA = {}; window.__LUMA_BRIDGE = {};\n{SCRIPT}",
        serde_json::to_string(USER_AGENT).map_err(|e| e.to_string())?,
        serde_json::to_string(bridge).map_err(|e| e.to_string())?,
    ))
}

fn create(
    app: &AppHandle,
    bridge: &str,
    fullscreen: bool,
    visible: bool,
    video_id: Option<&str>,
) -> Result<WebviewWindow, String> {
    let address = match video_id {
        Some(video_id) => format!("{URL}#/watch?v={video_id}"),
        None => URL.to_string(),
    };
    let url: tauri::Url = address
        .parse()
        .map_err(|e| format!("Invalid YouTube URL: {e}"))?;
    let main = app.get_webview_window("main").ok_or("Luma window is unavailable")?;
    let size = main.inner_size().map_err(|e| e.to_string())?;
    let position = main.outer_position().map_err(|e| e.to_string())?;
    let scale = main.scale_factor().map_err(|e| e.to_string())?;
    let restore = main.clone();
    let window = WebviewWindowBuilder::new(app, "youtube", WebviewUrl::External(url))
        .title("Luma TV")
        .inner_size(size.width as f64 / scale, size.height as f64 / scale)
        .position(position.x as f64 / scale, position.y as f64 / scale)
        .min_inner_size(720.0, 405.0)
        .fullscreen(fullscreen)
        .visible(visible)
        .user_agent(USER_AGENT)
        .initialization_script(script(bridge)?)
        .on_navigation(|url| url.scheme() == "about" || allowed(url.host_str()))
        .build()
        .map_err(|e| format!("Could not open YouTube TV: {e}"))?;
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            let _ = restore.show();
            let _ = restore.unminimize();
            let _ = restore.set_focus();
        }
    });
    Ok(window)
}

pub async fn prepare(app: AppHandle) {
    let Ok(bridge) = bridge(&app).await else { return };
    let state = app.state::<Bridge>();
    let _guard = state.window.lock().await;
    if app.get_webview_window("youtube").is_some() {
        return;
    }
    if let Err(error) = create(&app, &bridge, false, false, None) {
        eprintln!("YouTube preload skipped: {error}");
    }
}

#[tauri::command]
pub async fn open_youtube(app: AppHandle, fullscreen: bool) -> Result<(), String> {
    let state = app.state::<Bridge>();
    let _guard = state.window.lock().await;
    let window = match app.get_webview_window("youtube") {
        Some(window) => window,
        None => create(&app, &bridge(&app).await?, fullscreen, true, None)?,
    };
    window.set_fullscreen(fullscreen).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    if let Some(main) = app.get_webview_window("main") {
        main.hide().map_err(|e| e.to_string())?;
    }
    crate::activity::record_launch(&app, "YouTube");
    Ok(())
}

pub async fn open_video(
    app: AppHandle,
    video_id: &str,
    fullscreen: bool,
) -> Result<(), String> {
    let id = checked_id(video_id).map_err(|_| "That YouTube video identifier is invalid.".to_string())?;
    let state = app.state::<Bridge>();
    let _guard = state.window.lock().await;
    let window = match app.get_webview_window("youtube") {
        Some(window) => {
            let hash = serde_json::to_string(&format!("#/watch?v={id}"))
                .map_err(|error| error.to_string())?;
            let _ = window.eval(format!("location.hash = {hash};"));
            window
        }
        None => create(&app, &bridge(&app).await?, fullscreen, true, Some(id))?,
    };
    window.set_fullscreen(fullscreen).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    if let Some(main) = app.get_webview_window("main") {
        main.hide().map_err(|e| e.to_string())?;
    }
    crate::activity::record_launch(&app, "YouTube");
    Ok(())
}

pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("youtube") {
        let _ = window.eval("document.querySelector('video')?.pause();");
        let _ = window.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

pub fn navigate(window: &WebviewWindow, command: &str) {
    let command = serde_json::to_string(command).unwrap_or_default();
    let _ = window.eval(format!(
        "window.__lumaNavigate && window.__lumaNavigate({command});"
    ));
}

pub fn forward(app: &AppHandle, command: &str) {
    if command == "home" {
        close(app);
        return;
    }
    if let Some(window) = app.get_webview_window("youtube") {
        navigate(&window, command);
    }
}
