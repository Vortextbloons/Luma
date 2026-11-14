use crate::integrations::spotify::{self, auth::Auth, models::PlaybackState};
use serde::Serialize;
use tauri::{Manager, State};

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

impl From<PlaybackState> for MediaState {
    fn from(state: PlaybackState) -> Self {
        Self {
            available: state.player_available,
            title: state.title.unwrap_or_default(),
            artist: state.artist.unwrap_or_default(),
            playing: state.playing,
            can_toggle: state.can_toggle,
            can_previous: state.can_skip_previous,
            can_next: state.can_skip_next,
            error: state.error,
        }
    }
}

#[derive(Default)]
pub struct Media;

pub fn stop(app: &tauri::AppHandle) {
    spotify::shutdown(app);
}

pub async fn control(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    let auth = app.state::<Auth>();
    spotify::control(app, &auth, action, None).await
}

#[tauri::command]
pub async fn media_state(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
) -> Result<MediaState, String> {
    spotify::player::snapshot(&app, &auth)
        .await
        .map(MediaState::from)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn media_control(
    app: tauri::AppHandle,
    auth: State<'_, Auth>,
    action: String,
) -> Result<(), String> {
    spotify::control(&app, &auth, &action, None).await
}
