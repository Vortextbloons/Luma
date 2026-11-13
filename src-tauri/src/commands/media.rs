use serde::Serialize;
use tauri::{Emitter, Manager};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    available: bool,
    title: String,
    artist: String,
    playing: bool,
    can_toggle: bool,
    can_previous: bool,
    can_next: bool,
    error: Option<String>,
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::sync::Mutex;
    use windows::{
        Foundation::TypedEventHandler,
        Media::Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as SessionManager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
        },
        Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
    };

    pub struct Apartment;
    impl Apartment {
        pub fn new() -> Result<Self, String> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|e| e.to_string())?;
            Ok(Self)
        }
    }
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() }
        }
    }

    #[derive(Default)]
    struct Watch {
        manager: Option<SessionManager>,
        manager_token: Option<i64>,
        session: Option<Session>,
        metadata_token: Option<i64>,
        playback_token: Option<i64>,
    }
    impl Watch {
        fn clear_session(&mut self) {
            if let Some(session) = self.session.take() {
                if let Some(token) = self.metadata_token.take() {
                    let _ = session.RemoveMediaPropertiesChanged(token);
                }
                if let Some(token) = self.playback_token.take() {
                    let _ = session.RemovePlaybackInfoChanged(token);
                }
            }
        }
    }
    impl Drop for Watch {
        fn drop(&mut self) {
            self.clear_session();
            if let (Some(manager), Some(token)) = (&self.manager, self.manager_token) {
                let _ = manager.RemoveSessionsChanged(token);
            }
        }
    }
    #[derive(Default)]
    pub struct Media(Mutex<Watch>);

    fn spotify(manager: &SessionManager) -> Result<Option<Session>, String> {
        let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
        for index in 0..sessions.Size().map_err(|e| e.to_string())? {
            let session = sessions.GetAt(index).map_err(|e| e.to_string())?;
            let id = session
                .SourceAppUserModelId()
                .map_err(|e| e.to_string())?
                .to_string()
                .to_ascii_lowercase();
            if id == "spotify.exe" || id == "spotify" || id.starts_with("spotifyab.spotifymusic_") {
                return Ok(Some(session));
            }
        }
        Ok(None)
    }

    fn changed(app: tauri::AppHandle) {
        tauri::async_runtime::spawn_blocking(move || {
            let state = snapshot(&app).unwrap_or_else(|error| MediaState {
                error: Some(error),
                ..Default::default()
            });
            let _ = app.emit("spotify-state", state);
        });
    }

    pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
        let _apartment = Apartment::new()?;
        let media = app.state::<Media>();
        let mut watch = media.0.lock().map_err(|e| e.to_string())?;
        if watch.manager.is_none() {
            let manager = SessionManager::RequestAsync()
                .and_then(|operation| operation.get())
                .map_err(|e| format!("Windows media sessions are unavailable: {e}"))?;
            let handle = app.clone();
            watch.manager_token = Some(
                manager
                    .SessionsChanged(&TypedEventHandler::new(move |_, _| {
                        changed(handle.clone());
                        Ok(())
                    }))
                    .map_err(|e| e.to_string())?,
            );
            watch.manager = Some(manager);
        }
        Ok(())
    }

    pub fn snapshot(app: &tauri::AppHandle) -> Result<MediaState, String> {
        let _apartment = Apartment::new()?;
        start(app)?;
        let media = app.state::<Media>();
        let session = {
            let mut watch = media.0.lock().map_err(|e| e.to_string())?;
            let session = spotify(
                watch
                    .manager
                    .as_ref()
                    .ok_or("Windows media manager is unavailable.")?,
            )?;
            if session != watch.session {
                watch.clear_session();
                if let Some(ref session) = session {
                    let handle = app.clone();
                    watch.metadata_token = Some(
                        session
                            .MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
                                changed(handle.clone());
                                Ok(())
                            }))
                            .map_err(|e| e.to_string())?,
                    );
                    watch.session = Some(session.clone());
                    let handle = app.clone();
                    watch.playback_token = Some(
                        session
                            .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                                changed(handle.clone());
                                Ok(())
                            }))
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
            session
        };
        let Some(session) = session else {
            return Ok(MediaState {
                error: Some(
                    "No compatible Spotify session. Open Spotify and start playback on this PC."
                        .into(),
                ),
                ..Default::default()
            });
        };
        let properties = session
            .TryGetMediaPropertiesAsync()
            .and_then(|operation| operation.get())
            .map_err(|e| e.to_string())?;
        let playback = session.GetPlaybackInfo().map_err(|e| e.to_string())?;
        let controls = playback.Controls().map_err(|e| e.to_string())?;
        Ok(MediaState {
            available: true,
            title: properties.Title().map_err(|e| e.to_string())?.to_string(),
            artist: properties.Artist().map_err(|e| e.to_string())?.to_string(),
            playing: playback.PlaybackStatus().map_err(|e| e.to_string())?
                == PlaybackStatus::Playing,
            can_toggle: controls.IsPlayPauseToggleEnabled().unwrap_or(false),
            can_previous: controls.IsPreviousEnabled().unwrap_or(false),
            can_next: controls.IsNextEnabled().unwrap_or(false),
            error: None,
        })
    }

    pub fn control(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
        let _apartment = Apartment::new()?;
        start(app)?;
        let media = app.state::<Media>();
        let session = {
            let watch = media.0.lock().map_err(|e| e.to_string())?;
            spotify(
                watch
                    .manager
                    .as_ref()
                    .ok_or("Windows media manager is unavailable.")?,
            )?
            .ok_or("No compatible Spotify session. Start playback in Spotify on this PC.")?
        };
        let controls = session
            .GetPlaybackInfo()
            .and_then(|p| p.Controls())
            .map_err(|e| e.to_string())?;
        let operation = match action {
            "toggle" if controls.IsPlayPauseToggleEnabled().unwrap_or(false) => {
                session.TryTogglePlayPauseAsync()
            }
            "previous" if controls.IsPreviousEnabled().unwrap_or(false) => {
                session.TrySkipPreviousAsync()
            }
            "next" if controls.IsNextEnabled().unwrap_or(false) => session.TrySkipNextAsync(),
            "toggle" | "previous" | "next" => {
                return Err("Spotify does not currently support this control.".into())
            }
            _ => return Err("Unknown Spotify action.".into()),
        };
        if !operation
            .and_then(|operation| operation.get())
            .map_err(|e| e.to_string())?
        {
            return Err("Spotify declined the media control request.".into());
        }
        Ok(())
    }

    pub fn stop(app: &tauri::AppHandle) {
        if let Ok(mut watch) = app.state::<Media>().0.lock() {
            *watch = Watch::default();
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    #[derive(Default)]
    pub struct Media;
    pub fn start(_: &tauri::AppHandle) -> Result<(), String> {
        Err("Spotify media controls require Windows 10 or later.".into())
    }
    pub fn snapshot(_: &tauri::AppHandle) -> Result<MediaState, String> {
        Ok(MediaState {
            error: Some("Windows media sessions are unavailable on this platform.".into()),
            ..Default::default()
        })
    }
    pub fn control(_: &tauri::AppHandle, _: &str) -> Result<(), String> {
        Err("Spotify media controls require Windows.".into())
    }
    pub fn stop(_: &tauri::AppHandle) {}
}

pub use platform::{stop, Media};

#[tauri::command]
pub async fn media_state(app: tauri::AppHandle) -> Result<MediaState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(platform::snapshot(&app).unwrap_or_else(|error| MediaState {
            error: Some(error),
            ..Default::default()
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn media_control(app: tauri::AppHandle, action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || platform::control(&app, &action))
        .await
        .map_err(|e| e.to_string())?
}
