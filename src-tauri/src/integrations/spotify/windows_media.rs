use super::models::{now_ms, Command, PlaybackState};
use base64_helper::encode;

#[cfg(windows)]
mod platform {
    use super::*;
    use std::sync::Mutex;
    use windows::{
        Foundation::TypedEventHandler,
        Media::{
            Control::{
                GlobalSystemMediaTransportControlsSession as Session,
                GlobalSystemMediaTransportControlsSessionManager as SessionManager,
                GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
            },
            MediaPlaybackAutoRepeatMode,
        },
        Storage::Streams::DataReader,
        Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
    };

    pub struct Apartment {
        initialized: bool,
    }
    impl Apartment {
        pub fn new() -> Result<Self, String> {
            match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
                Ok(()) => Ok(Self { initialized: true }),
                Err(error) if error.code() == windows::Win32::Foundation::RPC_E_CHANGED_MODE => {
                    Ok(Self { initialized: false })
                }
                Err(error) => Err(error.to_string()),
            }
        }
    }
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.initialized {
                unsafe { RoUninitialize() }
            }
        }
    }

    #[derive(Default)]
    struct Watch {
        manager: Option<SessionManager>,
        manager_token: Option<i64>,
        session: Option<Session>,
        metadata_token: Option<i64>,
        playback_token: Option<i64>,
        timeline_token: Option<i64>,
        artwork_key: String,
        artwork: Option<String>,
        artwork_raw: Option<(String, Vec<u8>)>,
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
                if let Some(token) = self.timeline_token.take() {
                    let _ = session.RemoveTimelinePropertiesChanged(token);
                }
            }
            self.artwork_key.clear();
            self.artwork = None;
            self.artwork_raw = None;
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
        let sessions = manager.GetSessions().map_err(|error| error.to_string())?;
        for index in 0..sessions.Size().map_err(|error| error.to_string())? {
            let session = sessions.GetAt(index).map_err(|error| error.to_string())?;
            let id = session
                .SourceAppUserModelId()
                .map_err(|error| error.to_string())?
                .to_string()
                .to_ascii_lowercase();
            if id == "spotify.exe"
                || id == "spotify"
                || id.ends_with(r"\spotify.exe")
                || id.ends_with("/spotify.exe")
                || id.starts_with("spotifyab.spotifymusic_")
                || id.starts_with("spotifyab.spotifymusic!")
            {
                return Ok(Some(session));
            }
        }
        Ok(None)
    }

    fn ensure(app: &tauri::AppHandle) -> Result<(), String> {
        use tauri::Manager;
        let media = app.state::<Media>();
        let mut watch = media.0.lock().map_err(|error| error.to_string())?;
        if watch.manager.is_none() {
            let manager = SessionManager::RequestAsync()
                .and_then(|operation| operation.get())
                .map_err(|error| format!("Windows media sessions are unavailable: {error}"))?;
            let handle = app.clone();
            watch.manager_token = Some(
                manager
                    .SessionsChanged(&TypedEventHandler::new(move |_, _| {
                        crate::integrations::spotify::player::publish_media_event(&handle);
                        Ok(())
                    }))
                    .map_err(|error| error.to_string())?,
            );
            watch.manager = Some(manager);
        }
        Ok(())
    }

    pub fn start(app: &tauri::AppHandle) -> Result<(), String> {
        let _apartment = Apartment::new()?;
        ensure(app)
    }

    pub fn available(app: &tauri::AppHandle) -> bool {
        let Ok(_apartment) = Apartment::new() else {
            return false;
        };
        if ensure(app).is_err() {
            return false;
        }
        use tauri::Manager;
        let media = app.state::<Media>();
        let Ok(watch) = media.0.lock() else {
            return false;
        };
        watch
            .manager
            .as_ref()
            .and_then(|manager| spotify(manager).ok().flatten())
            .is_some()
    }

    fn artwork(
        reference: &windows::Storage::Streams::IRandomAccessStreamReference,
    ) -> Option<(String, String, Vec<u8>)> {
        let stream = reference.OpenReadAsync().ok()?.get().ok()?;
        let size = stream.Size().ok()?;
        if size == 0 || size > 3_000_000 {
            return None;
        }
        let input = stream.GetInputStreamAt(0).ok()?;
        let reader = DataReader::CreateDataReader(&input).ok()?;
        reader.LoadAsync(size as u32).ok()?.get().ok()?;
        let mut bytes = vec![0u8; size as usize];
        reader.ReadBytes(&mut bytes).ok()?;
        let content_type = stream
            .ContentType()
            .map(|value| value.to_string())
            .unwrap_or_default();
        let content_type = if content_type.starts_with("image/") {
            content_type
        } else {
            match bytes.as_slice() {
                [0xFF, 0xD8, ..] => "image/jpeg".into(),
                [0x89, b'P', b'N', b'G', ..] => "image/png".into(),
                _ => "image/jpeg".into(),
            }
        };
        let data_url = format!("data:{content_type};base64,{}", encode(&bytes));
        Some((data_url, content_type, bytes))
    }

    fn milliseconds(value: i64) -> u64 {
        value.max(0) as u64 / 10_000
    }

    pub fn snapshot(app: &tauri::AppHandle) -> Result<PlaybackState, String> {
        let _apartment = Apartment::new()?;
        ensure(app)?;
        use tauri::Manager;
        let media = app.state::<Media>();
        let mut watch = media.0.lock().map_err(|error| error.to_string())?;
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
                            crate::integrations::spotify::player::publish_media_event(&handle);
                            Ok(())
                        }))
                        .map_err(|error| error.to_string())?,
                );
                let handle = app.clone();
                watch.playback_token = Some(
                    session
                        .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                            crate::integrations::spotify::player::publish_media_event(&handle);
                            Ok(())
                        }))
                        .map_err(|error| error.to_string())?,
                );
                let handle = app.clone();
                watch.timeline_token = Some(
                    session
                        .TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                            crate::integrations::spotify::player::publish_media_event(&handle);
                            Ok(())
                        }))
                        .map_err(|error| error.to_string())?,
                );
                watch.session = Some(session.clone());
            }
        }
        let Some(session) = session else {
            return Ok(PlaybackState {
                player_available: false,
                updated_at: now_ms(),
                error: Some(
                    "Spotify is not exposing a Windows media session. Start a track to connect."
                        .into(),
                ),
                ..Default::default()
            });
        };

        let properties = session
            .TryGetMediaPropertiesAsync()
            .and_then(|operation| operation.get());
        let title = properties
            .as_ref()
            .ok()
            .and_then(|properties| properties.Title().ok())
            .map(|value| value.to_string())
            .unwrap_or_default();
        let artist = properties
            .as_ref()
            .ok()
            .and_then(|properties| properties.Artist().ok())
            .map(|value| value.to_string())
            .unwrap_or_default();
        let album = properties
            .as_ref()
            .ok()
            .and_then(|properties| properties.AlbumTitle().ok())
            .map(|value| value.to_string())
            .unwrap_or_default();

        let key = format!("{title}\u{1f}{artist}\u{1f}{album}");
        if watch.artwork_key != key {
            watch.artwork_key = key;
            let art = properties
                .as_ref()
                .ok()
                .and_then(|properties| properties.Thumbnail().ok())
                .and_then(|reference| artwork(&reference));
            watch.artwork = art.as_ref().map(|(data_url, _, _)| data_url.clone());
            watch.artwork_raw = art.map(|(_, content_type, bytes)| (content_type, bytes));
        }

        let playback = session.GetPlaybackInfo().ok();
        let status = playback
            .as_ref()
            .and_then(|playback| playback.PlaybackStatus().ok())
            .unwrap_or(PlaybackStatus::Closed);
        let controls = playback
            .as_ref()
            .and_then(|playback| playback.Controls().ok());
        let timeline = session.GetTimelineProperties().ok();
        let duration_ms = timeline.as_ref().and_then(|timeline| {
            let start = timeline.StartTime().ok()?.Duration;
            let end = timeline.EndTime().ok()?.Duration;
            (end > start).then(|| ((end - start) / 10_000) as u64)
        });
        let mut position_ms = timeline
            .as_ref()
            .and_then(|timeline| timeline.Position().ok())
            .map(|position| milliseconds(position.Duration));
        let updated = timeline
            .as_ref()
            .and_then(|timeline| timeline.LastUpdatedTime().ok())
            .map(|value| value.UniversalTime)
            .filter(|value| *value > 0);
        if status == PlaybackStatus::Playing {
            if let Some(updated) = updated {
                let epoch_ms = (updated / 10_000).saturating_sub(11_644_473_600_000);
                let now = now_ms();
                if now > epoch_ms as u64 {
                    position_ms = position_ms.map(|position| position + (now - epoch_ms as u64));
                }
            }
        }
        if let (Some(position), Some(duration)) = (position_ms, duration_ms) {
            position_ms = Some(position.min(duration));
        }

        let repeat = playback
            .as_ref()
            .and_then(|playback| playback.AutoRepeatMode().ok())
            .and_then(|mode| mode.Value().ok())
            .map(|mode| {
                if mode == MediaPlaybackAutoRepeatMode::Track {
                    "track"
                } else if mode == MediaPlaybackAutoRepeatMode::List {
                    "context"
                } else {
                    "off"
                }
                .to_string()
            });
        let shuffle = playback
            .as_ref()
            .and_then(|playback| playback.IsShuffleActive().ok())
            .and_then(|value| value.Value().ok());

        Ok(PlaybackState {
            connected: false,
            player_available: true,
            playing: status == PlaybackStatus::Playing,
            track_id: None,
            title: (!title.trim().is_empty()).then_some(title),
            artist: (!artist.trim().is_empty()).then_some(artist),
            album: (!album.trim().is_empty()).then_some(album),
            artwork_url: watch.artwork.clone(),
            position_ms,
            duration_ms,
            device_id: None,
            device_name: Some(super::super::models::computer_name()),
            can_seek: controls
                .as_ref()
                .is_some_and(|controls| controls.IsPlaybackPositionEnabled().unwrap_or(false)),
            can_skip_next: controls
                .as_ref()
                .is_some_and(|controls| controls.IsNextEnabled().unwrap_or(false)),
            can_skip_previous: controls
                .as_ref()
                .is_some_and(|controls| controls.IsPreviousEnabled().unwrap_or(false)),
            can_toggle: controls
                .as_ref()
                .is_some_and(|controls| controls.IsPlayPauseToggleEnabled().unwrap_or(false)),
            shuffle,
            repeat,
            volume: None,
            updated_at: now_ms(),
            error: None,
        })
    }

    pub fn control(app: &tauri::AppHandle, command: Command) -> Result<(), String> {
        let _apartment = Apartment::new()?;
        ensure(app)?;
        use tauri::Manager;
        let media = app.state::<Media>();
        let session = {
            let watch = media.0.lock().map_err(|error| error.to_string())?;
            spotify(
                watch
                    .manager
                    .as_ref()
                    .ok_or("Windows media manager is unavailable.")?,
            )?
            .ok_or("No Spotify media session is available. Start playback in Spotify on this PC.")?
        };
        let controls = session
            .GetPlaybackInfo()
            .and_then(|playback| playback.Controls())
            .map_err(|error| error.to_string())?;
        let operation = match command {
            Command::Toggle if controls.IsPlayPauseToggleEnabled().unwrap_or(false) => {
                session.TryTogglePlayPauseAsync()
            }
            Command::Play => session.TryPlayAsync(),
            Command::Pause => session.TryPauseAsync(),
            Command::Previous if controls.IsPreviousEnabled().unwrap_or(false) => {
                session.TrySkipPreviousAsync()
            }
            Command::Next if controls.IsNextEnabled().unwrap_or(false) => {
                session.TrySkipNextAsync()
            }
            Command::Seek(position) if controls.IsPlaybackPositionEnabled().unwrap_or(false) => {
                session.TryChangePlaybackPositionAsync((position as i64) * 10_000)
            }
            Command::Shuffle(active) if controls.IsShuffleEnabled().unwrap_or(false) => {
                session.TryChangeShuffleActiveAsync(active)
            }
            Command::Repeat(mode) if controls.IsRepeatEnabled().unwrap_or(false) => {
                let mode = match mode.as_str() {
                    "track" => MediaPlaybackAutoRepeatMode::Track,
                    "context" => MediaPlaybackAutoRepeatMode::List,
                    _ => MediaPlaybackAutoRepeatMode::None,
                };
                session.TryChangeAutoRepeatModeAsync(mode)
            }
            Command::Toggle
            | Command::Previous
            | Command::Next
            | Command::Seek(_)
            | Command::Shuffle(_)
            | Command::Repeat(_) => {
                return Err("Spotify does not currently expose this control.".into())
            }
        };
        if !operation
            .and_then(|operation| operation.get())
            .map_err(|error| error.to_string())?
        {
            return Err("Spotify declined the media control request.".into());
        }
        Ok(())
    }

    pub fn stop(app: &tauri::AppHandle) {
        use tauri::Manager;
        if let Ok(mut watch) = app.state::<Media>().0.lock() {
            *watch = Watch::default();
        }
    }

    pub fn current_artwork(app: &tauri::AppHandle) -> Option<(String, Vec<u8>)> {
        use tauri::Manager;
        let media = app.state::<Media>();
        let watch = media.0.lock().ok()?;
        watch.artwork_raw.clone()
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
    pub fn available(_: &tauri::AppHandle) -> bool {
        false
    }
    pub fn snapshot(_: &tauri::AppHandle) -> Result<PlaybackState, String> {
        Ok(PlaybackState {
            updated_at: now_ms(),
            error: Some("Windows media sessions are unavailable on this platform.".into()),
            ..Default::default()
        })
    }
    pub fn control(_: &tauri::AppHandle, _: Command) -> Result<(), String> {
        Err("Spotify media controls require Windows.".into())
    }
    pub fn stop(_: &tauri::AppHandle) {}
    pub fn current_artwork(_: &tauri::AppHandle) -> Option<(String, Vec<u8>)> {
        None
    }
}

pub use platform::{available, control, current_artwork, snapshot, start, stop, Media};

mod base64_helper {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let first = chunk[0] as u32;
            let second = chunk.get(1).copied().unwrap_or(0) as u32;
            let third = chunk.get(2).copied().unwrap_or(0) as u32;
            let value = (first << 16) | (second << 8) | third;
            output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
            output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
            output.push(if chunk.len() > 1 {
                ALPHABET[((value >> 6) & 63) as usize] as char
            } else {
                '='
            });
            output.push(if chunk.len() > 2 {
                ALPHABET[(value & 63) as usize] as char
            } else {
                '='
            });
        }
        output
    }
}
