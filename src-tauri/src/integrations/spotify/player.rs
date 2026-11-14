use super::{
    api, auth::Auth,
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
    starting: AtomicBool,
    error: Mutex<Option<String>>,
    display_name: Mutex<Option<String>>,
    recorded: Mutex<String>,
    preferred_device: Mutex<Option<String>>,
}

pub async fn initialize(app: tauri::AppHandle) {
    let starter = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || windows_media::start(&starter)).await;
    let handle = app;
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.tick().await;
        loop {
            interval.tick().await;
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
        if state.player_available {
            clear_error(&handle);
        }
        publish(&handle, state);
    });
}

pub async fn snapshot(
    app: &tauri::AppHandle,
    auth: &Auth,
) -> Result<PlaybackState, String> {
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
    if state.player_available {
        clear_error(app);
    }
    state.connected = auth.authenticated();
    if auth.authenticated() {
        match api::playback(auth).await {
            Ok(api_state) => {
                if state.player_available {
                    merge_api(&mut state, &api_state);
                } else if api_state.player_available {
                    state = api_state;
                }
                state.connected = true;
                clear_error(app);
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
    if same_track {
        if media.track_id.is_none() {
            media.track_id = api.track_id.clone();
        }
        if media.album.is_none() {
            media.album = api.album.clone();
        }
        if media.artwork_url.is_none() {
            media.artwork_url = api.artwork_url.clone();
        }
        if media.duration_ms.is_none() {
            media.duration_ms = api.duration_ms;
        }
        if media.position_ms.is_none() {
            media.position_ms = api.position_ms;
        }
        if media.shuffle.is_none() {
            media.shuffle = api.shuffle;
        }
        if media.repeat.is_none() {
            media.repeat = api.repeat.clone();
        }
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
    if media.device_name.is_none() {
        media.device_name = api.device_name.clone();
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

fn publish(app: &tauri::AppHandle, mut state: PlaybackState) {
    state.connected = app
        .try_state::<Auth>()
        .is_some_and(|auth| auth.authenticated());
    if let Some(player) = app.try_state::<Player>() {
        if let Ok(mut cache) = player.cache.lock() {
            *cache = Some(state.clone());
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
                    );
                }
            }
        }
    }
    let _ = app.emit("spotify-playback", state);
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
    let handle = app.clone();
    let windows_command = command.clone();
    let windows_result =
        tauri::async_runtime::spawn_blocking(move || windows_media::control(&handle, windows_command))
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
    let devices = if device_id.is_none() && windows_media::available(app) {
        api::devices(auth).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let target = device_id
        .map(str::to_string)
        .or(preferred)
        .or_else(|| pc_device(&devices).and_then(|device| device.id.clone()));
    if let Some(target) = target.as_deref() {
        let active = devices
            .iter()
            .find(|device| device.active)
            .and_then(|device| device.id.as_deref());
        if active != Some(target) {
            api::transfer(auth, target).await?;
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
    }
    api::play(auth, uri, target.as_deref()).await?;
    refresh_later(app.clone());
    Ok(())
}

fn pc_device(devices: &[Device]) -> Option<&Device> {
    let name = computer_name();
    devices
        .iter()
        .find(|device| {
            device.kind.eq_ignore_ascii_case("computer")
                && device.name.eq_ignore_ascii_case(&name)
        })
        .or_else(|| {
            devices
                .iter()
                .find(|device| device.kind.eq_ignore_ascii_case("computer"))
        })
}

pub async fn transfer(
    app: &tauri::AppHandle,
    auth: &Auth,
    device_id: &str,
) -> Result<(), String> {
    if !auth.authenticated() {
        return Err("Connect a Spotify account to switch playback devices.".into());
    }
    api::transfer(auth, device_id).await?;
    if let Ok(mut preferred) = app.state::<Player>().preferred_device.lock() {
        *preferred = Some(device_id.to_string());
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
    let cached = player
        .cache
        .lock()
        .ok()
        .and_then(|cache| cache.clone());
    let live = windows_media::available(app);
    let api_active = auth.authenticated()
        && cached
            .as_ref()
            .is_some_and(|state| state.player_available);
    ConnectionState {
        authenticated: auth.authenticated(),
        process_running: process::running(),
        installed: process::installed(),
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

pub fn set_error(app: &tauri::AppHandle, value: Option<String>) {
    if let Ok(mut error) = app.state::<Player>().error.lock() {
        *error = value;
    }
}

fn clear_error(app: &tauri::AppHandle) {
    if let Ok(mut error) = app.state::<Player>().error.lock() {
        *error = None;
    }
}

pub async fn start(app: tauri::AppHandle) -> Result<(), String> {
    if process::running() {
        set_error(&app, None);
        emit_connection(&app);
        return Ok(());
    }
    if !process::installed() {
        return Err(
            "Spotify is not installed. Install the Spotify desktop app on this PC first.".into(),
        );
    }
    process::launch_minimized()?;
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
                "Spotify started, but no media session appeared. Start a track in Spotify."
                    .into()
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
