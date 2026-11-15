use super::{
    api,
    auth::Auth,
    models::{computer_name, parse_command, ConnectionState, Device, PlaybackState},
    process, windows_media,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct Player {
    cache: Mutex<Option<PlaybackState>>,
    artwork: Mutex<Option<String>>,
    starting: AtomicBool,
    error: Mutex<Option<String>>,
    display_name: Mutex<Option<String>>,
    recorded: Mutex<String>,
    preferred_device: Mutex<Option<String>>,
}

pub async fn initialize(app: tauri::AppHandle) {
    let starter = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || windows_media::start(&starter)).await;
    if let Some(auth) = app.try_state::<Auth>() {
        if auth.authenticated() {
            let _ = snapshot(&app, &auth).await;
        }
    }
    let handle = app;
    tauri::async_runtime::spawn(async move {
        loop {
            let active = handle
                .try_state::<Player>()
                .and_then(|player| {
                    player
                        .cache
                        .lock()
                        .ok()
                        .and_then(|cache| cache.as_ref().map(|state| state.playing))
                })
                .unwrap_or(false);
            tokio::time::sleep(if active {
                Duration::from_secs(15)
            } else {
                Duration::from_secs(60)
            })
            .await;
            let auth = handle.state::<Auth>();
            if auth.authenticated() {
                let _ = snapshot(&handle, &auth).await;
            }
        }
    });
}

pub fn publish_media_event(app: &tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut state = windows_media::snapshot(&handle).unwrap_or_else(|error| PlaybackState {
            updated_at: super::models::now_ms(),
            error: Some(error),
            ..Default::default()
        });
        let cached = handle
            .state::<Player>()
            .cache
            .lock()
            .ok()
            .and_then(|cache| cache.clone());
        if let Some(cached) = cached.as_ref() {
            merge(&mut state, cached);
        }
        restore_artwork(&handle, &mut state, cached.as_ref());
        if state.player_available && clear_error(&handle) {
            emit_connection(&handle);
        }
        publish(&handle, state);
    });
}

async fn windows_state(app: &tauri::AppHandle, auth: &Auth) -> Result<PlaybackState, String> {
    let handle = app.clone();
    let mut state = tauri::async_runtime::spawn_blocking(move || windows_media::snapshot(&handle))
        .await
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|error| PlaybackState {
            updated_at: super::models::now_ms(),
            error: Some(error),
            ..Default::default()
        });
    let cached = app
        .state::<Player>()
        .cache
        .lock()
        .ok()
        .and_then(|cache| cache.clone());
    if let Some(cached) = cached.as_ref() {
        merge(&mut state, cached);
    }
    restore_artwork(app, &mut state, cached.as_ref());
    if state.player_available && clear_error(app) {
        emit_connection(app);
    }
    state.connected = auth.authenticated();
    Ok(state)
}

pub async fn current(app: &tauri::AppHandle, auth: &Auth) -> Result<PlaybackState, String> {
    let preferred = app
        .state::<Player>()
        .preferred_device
        .lock()
        .ok()
        .and_then(|device| device.clone());
    if preferred.is_some() && auth.authenticated() {
        return snapshot(app, auth).await;
    }
    let state = windows_state(app, auth).await?;
    publish(app, state.clone());
    Ok(state)
}

pub async fn snapshot(app: &tauri::AppHandle, auth: &Auth) -> Result<PlaybackState, String> {
    let mut state = windows_state(app, auth).await?;
    if auth.authenticated() {
        match api::playback(auth).await {
            Ok(api_state) => {
                let preferred = app
                    .state::<Player>()
                    .preferred_device
                    .lock()
                    .ok()
                    .and_then(|device| device.clone());
                if preferred.is_some() && api_state.player_available {
                    state = api_state;
                } else if state.player_available {
                    merge_api(&mut state, &api_state);
                } else if api_state.player_available {
                    state = api_state;
                }
                state.connected = true;
                if clear_error(app) {
                    emit_connection(app);
                }
            }
            Err(error) => {
                if !state.player_available {
                    state.error = Some(error);
                }
            }
        }
    }
    state.connected = auth.authenticated();
    state.updated_at = super::models::now_ms();
    publish(app, state.clone());
    Ok(state)
}

fn merge(media: &mut PlaybackState, cached: &PlaybackState) {
    if media.title == cached.title && media.artist == cached.artist {
        if media.track_id.is_none() {
            media.track_id = cached.track_id.clone();
        }
        if media.album.is_none() {
            media.album = cached.album.clone();
        }
        if media.artwork_url.is_none() {
            media.artwork_url = cached.artwork_url.clone();
        }
        if media.duration_ms.is_none() {
            media.duration_ms = cached.duration_ms;
        }
    }
    if media.device_id.is_none() {
        media.device_id = cached.device_id.clone();
    }
    if media.device_name.is_none() {
        media.device_name = cached.device_name.clone();
    }
    if media.shuffle.is_none() {
        media.shuffle = cached.shuffle;
    }
    if media.repeat.is_none() {
        media.repeat = cached.repeat.clone();
    }
    if media.volume.is_none() {
        media.volume = cached.volume;
    }
    if media.position_ms.is_none() {
        media.position_ms = cached.position_ms;
    }
    media.connected = media.connected || cached.connected;
}

fn merge_api(media: &mut PlaybackState, api: &PlaybackState) {
    let same_track = media.title == api.title && media.artist == api.artist;
    let adopt = same_track || media.title.is_none();
    if adopt {
        if media.track_id.is_none() {
            media.track_id = api.track_id.clone();
        }
        if media.title.is_none() {
            media.title = api.title.clone();
        }
        if media.artist.is_none() {
            media.artist = api.artist.clone();
        }
        if media.album.is_none() {
            media.album = api.album.clone();
        }
        if let Some(artwork) = api.artwork_url.as_ref() {
            if media
                .artwork_url
                .as_deref()
                .map_or(true, |value| value.starts_with("data:"))
            {
                media.artwork_url = Some(artwork.clone());
            }
        }
        if media.duration_ms.is_none() {
            media.duration_ms = api.duration_ms;
        }
        if media.position_ms.is_none() {
            media.position_ms = api.position_ms;
        }
        if media.device_id.is_none() {
            media.device_id = api.device_id.clone();
        }
        if media.shuffle.is_none() {
            media.shuffle = api.shuffle;
        }
        if media.repeat.is_none() {
            media.repeat = api.repeat.clone();
        }
        if media.volume.is_none() {
            media.volume = api.volume;
        }
        media.can_seek = media.can_seek || api.can_seek;
        media.can_skip_next = media.can_skip_next || api.can_skip_next;
        media.can_skip_previous = media.can_skip_previous || api.can_skip_previous;
        media.can_toggle = media.can_toggle || api.can_toggle;
    }
    if media.device_name.is_none() {
        media.device_name = api.device_name.clone();
    }
}

fn restore_artwork(
    app: &tauri::AppHandle,
    state: &mut PlaybackState,
    cached: Option<&PlaybackState>,
) {
    if state.artwork_url.is_some() || state.title.is_none() {
        return;
    }
    if !cached.is_some_and(|cached| cached.title == state.title && cached.artist == state.artist) {
        return;
    }
    state.artwork_url = app
        .state::<Player>()
        .artwork
        .lock()
        .ok()
        .and_then(|artwork| artwork.clone());
}

fn publish(app: &tauri::AppHandle, mut state: PlaybackState) {
    state.connected = app
        .try_state::<Auth>()
        .is_some_and(|auth| auth.authenticated());
    crate::remote::server::sync_music(app, &state);
    let artwork = state.artwork_url.take();
    let mut artwork_changed = false;
    if let Some(player) = app.try_state::<Player>() {
        if let Ok(mut cache) = player.cache.lock() {
            *cache = Some(state.clone());
        }
        if let Ok(mut cached) = player.artwork.lock() {
            if *cached != artwork {
                *cached = artwork.clone();
                artwork_changed = true;
            }
        }
        if let Some(title) = state.title.as_deref() {
            if !title.trim().is_empty() {
                let key = format!("{title}\u{1f}{}", state.artist.as_deref().unwrap_or(""));
                let changed = player
                    .recorded
                    .lock()
                    .map(|mut recorded| {
                        if *recorded == key {
                            false
                        } else {
                            *recorded = key;
                            true
                        }
                    })
                    .unwrap_or(true);
                if changed {
                    crate::activity::record_spotify(
                        app,
                        title,
                        state.artist.as_deref().unwrap_or(""),
                        state.track_id.as_deref(),
                        artwork.as_deref(),
                    );
                }
            }
        }
    }
    let _ = app.emit("spotify-playback", state);
    if artwork_changed {
        let _ = app.emit("spotify-artwork", artwork.unwrap_or_default());
    }
}

pub async fn control(
    app: &tauri::AppHandle,
    auth: &Auth,
    action: &str,
    value: Option<&str>,
) -> Result<(), String> {
    if action == "volume" {
        let value = value
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or("A volume level is required.")?;
        if !auth.authenticated() {
            return Err("Connect Spotify to control volume.".into());
        }
        api::volume(auth, value).await?;
        refresh_later(app.clone());
        return Ok(());
    }
    let command = parse_command(action, value)?;
    let preferred = app
        .state::<Player>()
        .preferred_device
        .lock()
        .ok()
        .and_then(|device| device.clone());
    if preferred.is_some() && auth.authenticated() {
        api::control(auth, command).await?;
        refresh_later(app.clone());
        return Ok(());
    }
    let handle = app.clone();
    let windows_command = command.clone();
    let windows_result = tauri::async_runtime::spawn_blocking(move || {
        windows_media::control(&handle, windows_command)
    })
    .await
    .map_err(|error| error.to_string())?;
    let windows_error = match windows_result {
        Ok(()) => {
            refresh_later(app.clone());
            return Ok(());
        }
        Err(error) => error,
    };
    if !auth.authenticated() {
        return Err(windows_error);
    }
    api::control(auth, command).await?;
    refresh_later(app.clone());
    Ok(())
}

pub async fn play_uri(
    app: &tauri::AppHandle,
    auth: &Auth,
    uri: &str,
    device_id: Option<&str>,
    context_uri: Option<&str>,
) -> Result<(), String> {
    if !auth.authenticated() {
        return Err("Connect a Spotify account to start playlists and albums from Luma.".into());
    }
    let preferred = app
        .state::<Player>()
        .preferred_device
        .lock()
        .ok()
        .and_then(|device| device.clone());
    let devices = api::devices(auth).await.unwrap_or_default();
    let target = device_id
        .map(str::to_string)
        .or_else(|| {
            preferred
                .as_deref()
                .filter(|id| {
                    devices.is_empty()
                        || devices
                            .iter()
                            .any(|device| device.id.as_deref() == Some(*id))
                })
                .map(str::to_string)
        })
        .or_else(|| pc_device(&devices).and_then(|device| device.id.clone()))
        .ok_or(
            "No Spotify device on this PC is available. Open Spotify on this PC and try again.",
        )?;
    let active = devices
        .iter()
        .find(|device| device.active)
        .and_then(|device| device.id.as_deref());
    if active != Some(target.as_str()) {
        api::transfer(auth, &target).await?;
        tokio::time::sleep(Duration::from_millis(350)).await;
    }
    api::play(auth, uri, Some(target.as_str()), context_uri).await?;
    refresh_later(app.clone());
    Ok(())
}

fn is_local_pc(device: &Device) -> bool {
    !device.restricted
        && device.kind.eq_ignore_ascii_case("computer")
        && device.name.eq_ignore_ascii_case(&computer_name())
}

fn pc_device(devices: &[Device]) -> Option<&Device> {
    devices
        .iter()
        .find(|device| is_local_pc(device))
        .or_else(|| {
            devices
                .iter()
                .find(|device| !device.restricted && device.kind.eq_ignore_ascii_case("computer"))
        })
}

pub async fn transfer(app: &tauri::AppHandle, auth: &Auth, device_id: &str) -> Result<(), String> {
    if !auth.authenticated() {
        return Err("Connect a Spotify account to switch playback devices.".into());
    }
    api::transfer(auth, device_id).await?;
    let devices = api::devices(auth).await.unwrap_or_default();
    let local = devices
        .iter()
        .find(|device| device.id.as_deref() == Some(device_id))
        .is_some_and(is_local_pc);
    if let Ok(mut preferred) = app.state::<Player>().preferred_device.lock() {
        *preferred = (!local).then(|| device_id.to_string());
    }
    refresh_later(app.clone());
    Ok(())
}

fn refresh_later(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(450)).await;
        let auth = app.state::<Auth>();
        let _ = snapshot(&app, &auth).await;
    });
}

pub fn connection(app: &tauri::AppHandle, auth: &Auth) -> ConnectionState {
    let player = app.state::<Player>();
    let cached = player.cache.lock().ok().and_then(|cache| cache.clone());
    let live = windows_media::available(app);
    let running = process::running();
    let api_active =
        auth.authenticated() && cached.as_ref().is_some_and(|state| state.player_available);
    ConnectionState {
        authenticated: auth.authenticated(),
        process_running: running,
        installed: process::installed(app) || running,
        player_available: live || api_active,
        starting: player.starting.load(Ordering::SeqCst),
        client_id_configured: auth.client_id_configured(),
        display_name: player
            .display_name
            .lock()
            .ok()
            .and_then(|value| value.clone()),
        error: player.error.lock().ok().and_then(|value| value.clone()),
    }
}

pub fn set_display_name(app: &tauri::AppHandle, value: Option<String>) {
    if let Ok(mut display) = app.state::<Player>().display_name.lock() {
        *display = value;
    }
}

pub fn clear_device_preference(app: &tauri::AppHandle) {
    if let Ok(mut preferred) = app.state::<Player>().preferred_device.lock() {
        *preferred = None;
    }
}

pub fn set_error(app: &tauri::AppHandle, value: Option<String>) {
    if let Ok(mut error) = app.state::<Player>().error.lock() {
        *error = value;
    }
}

fn clear_error(app: &tauri::AppHandle) -> bool {
    if let Ok(mut error) = app.state::<Player>().error.lock() {
        let had_error = error.is_some();
        *error = None;
        had_error
    } else {
        false
    }
}

pub async fn start(app: tauri::AppHandle) -> Result<(), String> {
    if process::running() {
        set_error(&app, None);
        emit_connection(&app);
        return Ok(());
    }
    if !process::installed(&app) {
        return Err(
            "Spotify is not installed. Install the Spotify desktop app on this PC first.".into(),
        );
    }
    process::launch_minimized(&app)?;
    crate::activity::record_launch(&app, "Spotify");
    let player = app.state::<Player>();
    player.starting.store(true, Ordering::SeqCst);
    set_error(&app, None);
    emit_connection(&app);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ready = false;
        for _ in 0..25 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if windows_media::available(&handle) {
                ready = true;
                break;
            }
        }
        handle
            .state::<Player>()
            .starting
            .store(false, Ordering::SeqCst);
        set_error(
            &handle,
            (!ready).then(|| {
                "Spotify started, but no media session appeared. Start a track in Spotify.".into()
            }),
        );
        emit_connection(&handle);
        let auth = handle.state::<Auth>();
        let _ = snapshot(&handle, &auth).await;
    });
    Ok(())
}

pub fn shutdown(app: &tauri::AppHandle) {
    windows_media::stop(app);
}

fn emit_connection(app: &tauri::AppHandle) {
    let auth = app.state::<Auth>();
    let _ = app.emit("spotify-connection", connection(app, &auth));
}
