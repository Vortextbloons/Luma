pub mod api;
pub mod auth;
pub mod models;
pub mod player;
pub mod process;
pub mod windows_media;

use auth::Auth;
use models::{ConnectionState, Device, HomeData, PlaybackState, Queue, SearchResults};
use tauri::{Emitter, State};

pub async fn initialize(app: tauri::AppHandle) {
    player::initialize(app).await;
}

pub fn shutdown(app: &tauri::AppHandle) {
    player::shutdown(app);
}

pub async fn control(
    app: &tauri::AppHandle,
    auth: &Auth,
    action: &str,
    value: Option<&str>,
) -> Result<(), String> {
    player::control(app, auth, action, value).await
}

#[tauri::command]
pub async fn spotify_status(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
) -> Result<ConnectionState, String> {
    Ok(player::connection(&app, &auth))
}

#[tauri::command]
pub async fn spotify_playback(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
) -> Result<PlaybackState, String> {
    player::current(&app, &auth).await
}

#[tauri::command]
pub async fn spotify_control(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
    action: String,
    value: Option<String>,
) -> Result<(), String> {
    player::control(&app, &auth, &action, value.as_deref()).await
}

#[tauri::command]
pub async fn spotify_play(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
    uri: String,
    device_id: Option<String>,
    context_uri: Option<String>,
) -> Result<(), String> {
    player::play_uri(
        &app,
        &auth,
        &uri,
        device_id.as_deref(),
        context_uri.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn spotify_transfer(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
    device_id: String,
) -> Result<(), String> {
    player::transfer(&app, &auth, &device_id).await
}

#[tauri::command]
pub async fn spotify_home(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
) -> Result<HomeData, String> {
    let data = api::home(&auth).await?;
    player::set_display_name(&app, data.display_name.clone());
    let _ = app.emit("spotify-connection", player::connection(&app, &auth));
    Ok(data)
}

#[tauri::command]
pub async fn spotify_search(auth: State<'_, Auth>, query: String) -> Result<SearchResults, String> {
    api::search(&auth, &query).await
}

#[tauri::command]
pub async fn spotify_queue(auth: State<'_, Auth>) -> Result<Queue, String> {
    api::queue(&auth).await
}

#[tauri::command]
pub async fn spotify_devices(auth: State<'_, Auth>) -> Result<Vec<Device>, String> {
    api::devices(&auth).await
}

#[tauri::command]
pub async fn spotify_client_id(auth: State<'_, Auth>) -> Result<String, String> {
    Ok(auth.client_id())
}

#[tauri::command]
pub async fn spotify_set_client_id(auth: State<'_, Auth>, client_id: String) -> Result<(), String> {
    auth.save_client_id(&client_id)
}

#[tauri::command]
pub async fn spotify_connect(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
) -> Result<ConnectionState, String> {
    auth.connect().await?;
    if let Ok(Some(name)) = api::profile(&auth).await {
        player::set_display_name(&app, Some(name));
    }
    player::set_error(&app, None);
    let _ = player::snapshot(&app, &auth).await;
    let state = player::connection(&app, &auth);
    let _ = app.emit("spotify-connection", state.clone());
    Ok(state)
}

#[tauri::command]
pub async fn spotify_disconnect(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
) -> Result<ConnectionState, String> {
    auth.disconnect()?;
    player::set_display_name(&app, None);
    player::clear_device_preference(&app);
    let _ = player::snapshot(&app, &auth).await;
    let state = player::connection(&app, &auth);
    let _ = app.emit("spotify-connection", state.clone());
    Ok(state)
}

#[tauri::command]
pub async fn spotify_start(app: tauri::AppHandle) -> Result<(), String> {
    player::start(app).await
}

#[tauri::command]
pub async fn spotify_browse(
    auth: State<'_, Auth>,
    kind: String,
    id: String,
    offset: u32,
) -> Result<models::BrowsePage, String> {
    api::browse(&auth, &kind, &id, offset).await
}
