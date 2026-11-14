use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityType {
    Movie,
    Episode,
    Video,
    Track,
    Application,
}

impl ActivityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Episode => "episode",
            Self::Video => "video",
            Self::Track => "track",
            Self::Application => "application",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "movie" => Some(Self::Movie),
            "episode" => Some(Self::Episode),
            "video" => Some(Self::Video),
            "track" => Some(Self::Track),
            "application" => Some(Self::Application),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressSource {
    Provider,
    Luma,
    Import,
    Unknown,
}

impl ProgressSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Luma => "luma",
            Self::Import => "import",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "provider" => Some(Self::Provider),
            "luma" => Some(Self::Luma),
            "import" => Some(Self::Import),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResumeKind {
    Direct,
    Content,
    Service,
    #[serde(rename = "none")]
    Unavailable,
}

impl ResumeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Content => "content",
            Self::Service => "service",
            Self::Unavailable => "none",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "direct" => Some(Self::Direct),
            "content" => Some(Self::Content),
            "service" => Some(Self::Service),
            "none" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub id: String,
    pub provider: String,
    pub provider_item_id: Option<String>,
    pub profile_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: ActivityType,
    pub title: String,
    pub subtitle: Option<String>,
    pub artwork: Option<String>,
    pub last_activity_at: String,
    pub progress_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub progress_source: ProgressSource,
    pub completed: Option<bool>,
    pub can_resume: bool,
    pub launch_target: Option<String>,
    pub resume_kind: ResumeKind,
    pub launch_count: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderState {
    Available,
    NeedsSetup,
    NotValidated,
    Connected,
    Disabled,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub id: String,
    pub name: String,
    pub state: ProviderState,
    pub detail: String,
    pub enabled: bool,
    pub hidden: bool,
    pub configured: bool,
    pub validated: bool,
    pub requires_config: bool,
    pub capabilities: Vec<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySnapshot {
    pub items: Vec<ActivityItem>,
    pub providers: Vec<ProviderStatus>,
    pub tracking_enabled: bool,
    pub generated_at: String,
}

#[derive(Clone, Debug, Default)]
pub struct ProviderConfig {
    pub server_url: Option<String>,
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderConfigInput {
    pub server_url: Option<String>,
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackReport {
    pub video_id: String,
    pub title: String,
    pub channel: Option<String>,
    pub artwork: Option<String>,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub completed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub snapshot: ActivitySnapshot,
    pub updated: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOutcome {
    pub kind: ResumeKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Local,
    YouTube,
    Spotify,
    Plex,
    Jellyfin,
    Trakt,
    Launcher,
}

#[derive(Clone, Copy, Debug)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ProviderKind,
    pub capabilities: &'static [&'static str],
}

pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "local",
        name: "This PC",
        kind: ProviderKind::Local,
        capabilities: &["appLaunches"],
    },
    ProviderInfo {
        id: "youtube",
        name: "YouTube",
        kind: ProviderKind::YouTube,
        capabilities: &["playbackTracking", "progress"],
    },
    ProviderInfo {
        id: "spotify",
        name: "Spotify",
        kind: ProviderKind::Spotify,
        capabilities: &["currentMediaSession"],
    },
    ProviderInfo {
        id: "plex",
        name: "Plex",
        kind: ProviderKind::Plex,
        capabilities: &["onDeck", "progress", "contentLinks"],
    },
    ProviderInfo {
        id: "jellyfin",
        name: "Jellyfin",
        kind: ProviderKind::Jellyfin,
        capabilities: &["resume", "progress", "contentLinks"],
    },
    ProviderInfo {
        id: "trakt",
        name: "Trakt",
        kind: ProviderKind::Trakt,
        capabilities: &["watchHistory"],
    },
    ProviderInfo {
        id: "stremio",
        name: "Stremio",
        kind: ProviderKind::Launcher,
        capabilities: &["appLaunches"],
    },
    ProviderInfo {
        id: "netflix",
        name: "Netflix",
        kind: ProviderKind::Launcher,
        capabilities: &["appLaunches"],
    },
    ProviderInfo {
        id: "disneyplus",
        name: "Disney+",
        kind: ProviderKind::Launcher,
        capabilities: &["appLaunches"],
    },
    ProviderInfo {
        id: "primevideo",
        name: "Prime Video",
        kind: ProviderKind::Launcher,
        capabilities: &["appLaunches"],
    },
];

pub fn provider_info(id: &str) -> Option<&'static ProviderInfo> {
    PROVIDERS.iter().find(|provider| provider.id == id)
}

pub fn app_provider(app: &str) -> &'static str {
    match app {
        "YouTube" => "youtube",
        "Spotify" => "spotify",
        "Stremio" => "stremio",
        "Netflix" => "netflix",
        "Disney+" => "disneyplus",
        "Prime Video" => "primevideo",
        _ => "local",
    }
}

pub fn app_id(app: &str) -> String {
    let slug: String = app
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("app:{}:{}", app_provider(app), slug.trim_matches('-'))
}

pub fn provider_enabled_key(provider: &str) -> String {
    format!("provider.{provider}.enabled")
}

pub fn provider_hidden_key(provider: &str) -> String {
    format!("provider.{provider}.hidden")
}

pub fn provider_url_key(provider: &str) -> String {
    format!("provider.{provider}.url")
}

pub fn provider_user_key(provider: &str) -> String {
    format!("provider.{provider}.user")
}

pub fn provider_client_key(provider: &str) -> String {
    format!("provider.{provider}.client")
}

pub fn provider_success_key(provider: &str) -> String {
    format!("provider.{provider}.lastSuccess")
}

pub fn provider_error_key(provider: &str) -> String {
    format!("provider.{provider}.lastError")
}
