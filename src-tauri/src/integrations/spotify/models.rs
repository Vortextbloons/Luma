use serde::{Deserialize, Serialize};

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default()
}

pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "This PC".into())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: Option<String>,
    pub uri: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub connected: bool,
    pub player_available: bool,
    pub playing: bool,
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub position_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub can_seek: bool,
    pub can_skip_next: bool,
    pub can_skip_previous: bool,
    pub can_toggle: bool,
    pub shuffle: Option<bool>,
    pub repeat: Option<String>,
    pub volume: Option<u8>,
    pub updated_at: u64,
    pub error: Option<String>,
}

impl PlaybackState {
    pub fn track(&self) -> Option<Track> {
        let title = self.title.clone().filter(|value| !value.trim().is_empty())?;
        Some(Track {
            id: self.track_id.clone(),
            uri: self
                .track_id
                .as_ref()
                .map(|id| format!("spotify:track:{id}")),
            title,
            artist: self.artist.clone().unwrap_or_default(),
            album: self.album.clone().unwrap_or_default(),
            artwork_url: self.artwork_url.clone(),
            duration_ms: self.duration_ms,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionState {
    pub authenticated: bool,
    pub process_running: bool,
    pub installed: bool,
    pub player_available: bool,
    pub starting: bool,
    pub client_id_configured: bool,
    pub display_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub artwork_url: Option<String>,
    pub track_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub artwork_url: Option<String>,
    pub year: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: Option<String>,
    pub name: String,
    pub kind: String,
    pub active: bool,
    pub restricted: bool,
    pub volume: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeData {
    pub authenticated: bool,
    pub display_name: Option<String>,
    pub recently_played: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub albums: Vec<Album>,
    pub tracks: Vec<Track>,
    pub devices: Vec<Device>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub playlists: Vec<Playlist>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub currently_playing: Option<Track>,
    pub items: Vec<Track>,
}

#[derive(Clone, Debug)]
pub enum Command {
    Toggle,
    Play,
    Pause,
    Next,
    Previous,
    Seek(u64),
    Shuffle(bool),
    Repeat(String),
}

pub fn parse_command(action: &str, value: Option<&str>) -> Result<Command, String> {
    match action {
        "toggle" | "playpause" => Ok(Command::Toggle),
        "play" => Ok(Command::Play),
        "pause" => Ok(Command::Pause),
        "next" => Ok(Command::Next),
        "previous" => Ok(Command::Previous),
        "seek" => value
            .and_then(|value| value.parse::<u64>().ok())
            .map(Command::Seek)
            .ok_or_else(|| "A valid position is required to seek.".into()),
        "shuffle" => match value {
            Some("true") => Ok(Command::Shuffle(true)),
            Some("false") => Ok(Command::Shuffle(false)),
            _ => Err("A valid shuffle state is required.".into()),
        },
        "repeat" => match value {
            Some(value @ ("off" | "track" | "context")) => Ok(Command::Repeat(value.into())),
            _ => Err("A valid repeat mode is required.".into()),
        },
        _ => Err("Unknown Spotify action.".into()),
    }
}
