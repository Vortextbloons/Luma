use super::{
    credentials,
    models::{
        app_provider, provider_client_key, provider_enabled_key, provider_error_key,
        provider_hidden_key, provider_info, provider_success_key, provider_url_key,
        provider_user_key, ActivityItem, ActivitySnapshot, OpenOutcome, PlaybackReport,
        ProviderConfig, ProviderConfigInput, ProviderInfo, ProviderKind, ProviderState,
        ProviderStatus, RefreshResult, ResumeKind, PROVIDERS,
    },
    providers::{self, local, now, validated_base},
    storage::Storage,
};
use crate::commands::{self, apps};
use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

const TRACKING_KEY: &str = "tracking.enabled";
const TOKEN_SAVED_KEY: &str = "tokenSaved";

#[derive(Default)]
struct RefreshState {
    sequence: u64,
    pending: BTreeMap<String, u64>,
}

impl RefreshState {
    fn begin(&mut self, provider: &str) -> u64 {
        self.sequence += 1;
        self.pending.insert(provider.into(), self.sequence);
        self.sequence
    }
}

pub struct Activity {
    storage: Storage,
    refresh_revision: Mutex<RefreshState>,
}

impl Activity {
    pub fn initialize(app: &AppHandle) -> Result<Self, String> {
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        Ok(Self {
            storage: Storage::open(&directory.join("activity.sqlite3"))?,
            refresh_revision: Mutex::new(RefreshState::default()),
        })
    }

    pub fn snapshot(&self) -> Result<ActivitySnapshot, String> {
        let tracking_enabled = self.storage.bool_preference(TRACKING_KEY, true)?;
        let items = self.storage.list(50)?;
        Ok(ActivitySnapshot {
            items,
            providers: self.providers()?,
            tracking_enabled,
            generated_at: now(),
        })
    }

    fn providers(&self) -> Result<Vec<ProviderStatus>, String> {
        PROVIDERS
            .iter()
            .map(|provider| self.status(provider))
            .collect()
    }

    fn status(&self, provider: &ProviderInfo) -> Result<ProviderStatus, String> {
        let enabled = self
            .storage
            .bool_preference(&provider_enabled_key(provider.id), true)?;
        let hidden = self
            .storage
            .bool_preference(&provider_hidden_key(provider.id), false)?;
        let last_success_at = self
            .storage
            .preference(&provider_success_key(provider.id))?;
        let last_error = self.storage.preference(&provider_error_key(provider.id))?;
        let configured = match provider.kind {
            ProviderKind::Plex | ProviderKind::Jellyfin => {
                self.storage
                    .preference(&provider_url_key(provider.id))?
                    .is_some()
                    && self.storage.bool_preference(
                        &format!("provider.{}.{TOKEN_SAVED_KEY}", provider.id),
                        false,
                    )?
            }
            ProviderKind::Trakt => {
                self.storage
                    .preference(&provider_client_key(provider.id))?
                    .is_some()
                    && self.storage.bool_preference(
                        &format!("provider.{}.{TOKEN_SAVED_KEY}", provider.id),
                        false,
                    )?
            }
            _ => true,
        };
        let observed_playback = match provider.kind {
            ProviderKind::YouTube | ProviderKind::Spotify => {
                self.storage.count_playback(provider.id)? > 0
            }
            _ => false,
        };
        let validated = match provider.kind {
            ProviderKind::Local | ProviderKind::Launcher => true,
            ProviderKind::YouTube | ProviderKind::Spotify => observed_playback,
            ProviderKind::Plex | ProviderKind::Jellyfin | ProviderKind::Trakt => {
                last_success_at.is_some()
            }
        };
        let state = if !enabled {
            ProviderState::Disabled
        } else {
            match provider.kind {
                ProviderKind::Plex | ProviderKind::Jellyfin | ProviderKind::Trakt => {
                    if !configured {
                        ProviderState::NeedsSetup
                    } else if last_error.is_some() {
                        ProviderState::Error
                    } else if last_success_at.is_none() {
                        ProviderState::NotValidated
                    } else {
                        ProviderState::Connected
                    }
                }
                _ => ProviderState::Available,
            }
        };
        let detail = match (provider.kind, state) {
            (_, ProviderState::Disabled) => "Off. You can still clear its history.".into(),
            (ProviderKind::Local, _) => "Records apps opened from Luma on this PC.".into(),
            (ProviderKind::YouTube, _) => "Records videos played in Luma's YouTube window.".into(),
            (ProviderKind::Spotify, _) => {
                "Records what Spotify is playing while Luma is open.".into()
            }
            (ProviderKind::Launcher, _) if provider.id == "stremio" => {
                "Connect a Stremio account under Integrations to see library progress.".into()
            }
            (ProviderKind::Launcher, _) => "Records when you open this app from Luma.".into(),
            (_, ProviderState::NeedsSetup) => match provider.kind {
                ProviderKind::Trakt => {
                    "Add a Trakt client ID and access token under Advanced.".into()
                }
                _ => format!("Add your {} details under Advanced.", provider.name),
            },
            (_, ProviderState::NotValidated) => {
                "Settings saved. Check the connection to import activity.".into()
            }
            (_, ProviderState::Error) => last_error
                .clone()
                .unwrap_or_else(|| "The last check did not work. Try again.".into()),
            (_, ProviderState::Connected) => format!(
                "Connected. Last checked {}.",
                last_success_at.clone().unwrap_or_default()
            ),
            _ => format!("{} is ready on this PC.", provider.name),
        };
        Ok(ProviderStatus {
            id: provider.id.into(),
            name: provider.name.into(),
            state,
            detail,
            enabled,
            hidden,
            configured,
            validated,
            requires_config: matches!(
                provider.kind,
                ProviderKind::Plex | ProviderKind::Jellyfin | ProviderKind::Trakt
            ),
            capabilities: provider
                .capabilities
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            last_success_at,
            last_error,
        })
    }

    fn tracking_enabled(&self) -> Result<bool, String> {
        self.storage.bool_preference(TRACKING_KEY, true)
    }

    fn provider_enabled(&self, provider: &str) -> Result<bool, String> {
        self.storage
            .bool_preference(&provider_enabled_key(provider), true)
    }

    pub fn record_launch(&self, app: &str) -> Result<Option<ActivityItem>, String> {
        let provider = app_provider(app);
        if !self.tracking_enabled()? || !self.provider_enabled(provider)? {
            return Ok(None);
        }
        self.storage.record_launch(app, &now()).map(Some)
    }

    pub fn record_youtube(&self, report: &PlaybackReport) -> Result<Option<ActivityItem>, String> {
        let valid_id = report.video_id.len() == 11
            && report
                .video_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if !valid_id
            || report.title.trim().is_empty()
            || !report.position_seconds.is_finite()
            || !report.duration_seconds.is_finite()
            || report.duration_seconds <= 0.0
            || !self.tracking_enabled()?
            || !self.provider_enabled("youtube")?
        {
            return Ok(None);
        }
        let item = local::youtube(report, &now());
        self.storage.upsert(&item)?;
        Ok(Some(item))
    }

    pub fn record_spotify(
        &self,
        title: &str,
        artist: &str,
        track_id: Option<&str>,
        artwork: Option<&str>,
    ) -> Result<Option<ActivityItem>, String> {
        let title = title.trim();
        if title.is_empty() || !self.tracking_enabled()? || !self.provider_enabled("spotify")? {
            return Ok(None);
        }
        let mut item = super::providers::spotify::activity(title, artist, track_id, &now());
        item.artwork = artwork.map(str::to_owned);
        if let Some(latest) = self.storage.latest_for_provider("spotify")? {
            if latest.id == item.id && latest.artwork == item.artwork {
                if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&latest.last_activity_at) {
                    if chrono::Utc::now().signed_duration_since(last.with_timezone(&chrono::Utc))
                        < chrono::Duration::minutes(30)
                    {
                        return Ok(None);
                    }
                }
            }
        }
        self.storage.upsert(&item)?;
        Ok(Some(item))
    }

    pub fn set_tracking(&self, enabled: bool) -> Result<(), String> {
        let _revision = self.invalidate_refreshes()?;
        self.storage.set_bool_preference(TRACKING_KEY, enabled)
    }

    pub fn set_provider_enabled(&self, provider: &str, enabled: bool) -> Result<(), String> {
        let _revision = self.invalidate_refreshes()?;
        provider_info(provider).ok_or("Unknown activity provider.")?;
        self.storage
            .set_bool_preference(&provider_enabled_key(provider), enabled)
    }

    pub fn set_provider_hidden(&self, provider: &str, hidden: bool) -> Result<(), String> {
        provider_info(provider).ok_or("Unknown activity provider.")?;
        self.storage
            .set_bool_preference(&provider_hidden_key(provider), hidden)
    }

    pub fn clear(&self, item_id: Option<&str>, provider: Option<&str>) -> Result<(), String> {
        let _revision = self.invalidate_refreshes()?;
        match (item_id, provider) {
            (Some(id), _) => {
                if id.trim().is_empty() {
                    return Err("Choose an activity item to remove.".into());
                }
                self.storage.delete(id)
            }
            (None, Some(provider)) => {
                provider_info(provider).ok_or("Unknown activity provider.")?;
                self.storage.clear(Some(provider))
            }
            (None, None) => self.storage.clear(None),
        }
    }

    pub fn save_config(&self, provider: &str, input: &ProviderConfigInput) -> Result<(), String> {
        let _revision = self.invalidate_refreshes()?;
        let info = provider_info(provider).ok_or("Unknown activity provider.")?;
        if !matches!(
            info.kind,
            ProviderKind::Plex | ProviderKind::Jellyfin | ProviderKind::Trakt
        ) {
            return Err(format!(
                "{} does not use connection credentials.",
                info.name
            ));
        }
        if let Some(value) = input.server_url.as_deref() {
            let base = validated_base(value)?;
            self.storage
                .set_preference(&provider_url_key(provider), &base)?;
        }
        if let Some(value) = input.user_id.as_deref() {
            let value = value.trim();
            if value.len() > 128 {
                return Err("The Jellyfin user ID is too long.".into());
            }
            if value.is_empty() {
                self.storage
                    .remove_preference(&provider_user_key(provider))?;
            } else {
                self.storage
                    .set_preference(&provider_user_key(provider), value)?;
            }
        }
        if let Some(value) = input.client_id.as_deref() {
            let value = value.trim();
            if value.len() > 256 {
                return Err("The Trakt client ID is too long.".into());
            }
            if value.is_empty() {
                self.storage
                    .remove_preference(&provider_client_key(provider))?;
            } else {
                self.storage
                    .set_preference(&provider_client_key(provider), value)?;
            }
        }
        if let Some(token) = input.token.as_deref() {
            let token = token.trim();
            if token.is_empty() {
                credentials::delete(provider)?;
                self.storage.set_bool_preference(
                    &format!("provider.{provider}.{TOKEN_SAVED_KEY}"),
                    false,
                )?;
            } else {
                if token.len() > 4096 {
                    return Err("That token is too long.".into());
                }
                credentials::save(provider, token)?;
                self.storage
                    .set_bool_preference(&format!("provider.{provider}.{TOKEN_SAVED_KEY}"), true)?;
            }
        }
        if input.server_url.is_some()
            || input.user_id.is_some()
            || input.client_id.is_some()
            || input.token.is_some()
        {
            self.storage
                .remove_preference(&provider_success_key(provider))?;
            self.storage
                .remove_preference(&provider_error_key(provider))?;
        }
        Ok(())
    }

    pub fn disconnect(&self, provider: &str) -> Result<(), String> {
        let _revision = self.invalidate_refreshes()?;
        provider_info(provider).ok_or("Unknown activity provider.")?;
        credentials::delete(provider)?;
        self.storage
            .set_bool_preference(&format!("provider.{provider}.{TOKEN_SAVED_KEY}"), false)?;
        self.storage
            .remove_preference(&provider_success_key(provider))?;
        self.storage
            .remove_preference(&provider_error_key(provider))?;
        Ok(())
    }

    fn invalidate_refreshes(&self) -> Result<MutexGuard<'_, RefreshState>, String> {
        let mut revision = self.refresh_revision.lock().map_err(|e| e.to_string())?;
        revision.pending.clear();
        Ok(revision)
    }

    pub async fn refresh(&self, provider: &str) -> Result<RefreshResult, String> {
        let info = provider_info(provider).ok_or("Unknown activity provider.")?;
        if !self.tracking_enabled()? {
            return Err("Activity tracking is off.".into());
        }
        if !self.provider_enabled(provider)? {
            return Err(format!("{} is disabled in Activity settings.", info.name));
        }
        if !matches!(
            info.kind,
            ProviderKind::Plex | ProviderKind::Jellyfin | ProviderKind::Trakt
        ) {
            return Err(format!(
                "{} has no remote activity source to refresh.",
                info.name
            ));
        }
        let (revision, config) = {
            let mut revision = self.refresh_revision.lock().map_err(|e| e.to_string())?;
            let config = ProviderConfig {
                server_url: self.storage.preference(&provider_url_key(provider))?,
                user_id: self.storage.preference(&provider_user_key(provider))?,
                client_id: self.storage.preference(&provider_client_key(provider))?,
                token: credentials::read(provider)?,
            };
            (revision.begin(provider), config)
        };
        let configured = config.token.is_some()
            && match info.kind {
                ProviderKind::Trakt => config.client_id.is_some(),
                _ => config.server_url.is_some(),
            };
        if !configured {
            return Err(format!(
                "{} needs {} in Activity settings.",
                info.name,
                if info.kind == ProviderKind::Trakt {
                    "a client ID and access token"
                } else {
                    "a server URL and token"
                }
            ));
        }
        let result = providers::refresh(provider, &config).await;
        self.finish_refresh(provider, revision, result)
    }

    fn finish_refresh(
        &self,
        provider: &str,
        revision: u64,
        result: Result<providers::ProviderRefresh, String>,
    ) -> Result<RefreshResult, String> {
        let mut current = self.refresh_revision.lock().map_err(|e| e.to_string())?;
        if current.pending.get(provider) != Some(&revision)
            || !self.tracking_enabled()?
            || !self.provider_enabled(provider)?
        {
            return Err(
                "Activity settings changed during synchronization. Check the connection again."
                    .into(),
            );
        }
        current.pending.remove(provider);
        match result {
            Ok(data) => {
                let updated = data.items.len();
                self.storage.import_provider(provider, &data, &now())?;
                Ok(RefreshResult {
                    snapshot: self.snapshot()?,
                    updated,
                    error: None,
                })
            }
            Err(error) => {
                self.storage
                    .set_preference(&provider_error_key(provider), &error)?;
                Ok(RefreshResult {
                    snapshot: self.snapshot()?,
                    updated: 0,
                    error: Some(error),
                })
            }
        }
    }

    pub async fn open(
        &self,
        app: &AppHandle,
        id: &str,
        fullscreen: bool,
    ) -> Result<OpenOutcome, String> {
        let item = self
            .storage
            .get(id)?
            .ok_or("That activity item is no longer stored.")?;
        if item.item_type == super::models::ActivityType::Application {
            if item.title == "YouTube" {
                commands::youtube::open_youtube(app.clone(), fullscreen).await?;
                return Ok(OpenOutcome {
                    kind: ResumeKind::Service,
                    message: "YouTube opened.".into(),
                });
            }
            let name = item
                .launch_target
                .clone()
                .unwrap_or_else(|| item.title.clone());
            apps::launch_app(name, app.state::<apps::Apps>(), app.clone()).await?;
            return Ok(OpenOutcome {
                kind: ResumeKind::Service,
                message: format!("{} opened.", item.title),
            });
        }
        if item.provider == "youtube" {
            let video_id = item
                .provider_item_id
                .as_deref()
                .ok_or("This YouTube item has no video identifier.")?;
            commands::youtube::open_video(app.clone(), video_id, fullscreen).await?;
            return Ok(OpenOutcome {
                kind: ResumeKind::Content,
                message: "YouTube video opened.".into(),
            });
        }
        if item.resume_kind == ResumeKind::Service {
            let name = item
                .launch_target
                .clone()
                .ok_or("This item does not name a service to open.")?;
            if name == "YouTube" {
                commands::youtube::open_youtube(app.clone(), fullscreen).await?;
            } else {
                apps::launch_app(name, app.state::<apps::Apps>(), app.clone()).await?;
            }
            return Ok(OpenOutcome {
                kind: ResumeKind::Service,
                message: format!("{} opened.", item.title),
            });
        }
        let target = item
            .launch_target
            .as_deref()
            .ok_or("This item does not provide a supported content link.")?;
        let url = safe_target(self, &item.provider, target)?;
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|error| format!("Could not open the content link: {error}"))?;
        Ok(OpenOutcome {
            kind: item.resume_kind,
            message: if item.resume_kind == ResumeKind::Direct {
                "Resumed where you left off.".into()
            } else {
                "Opened in the service.".into()
            },
        })
    }
}

fn safe_target(activity: &Activity, provider: &str, target: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(target)
        .map_err(|_| "The provider returned an invalid content link.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Only http or https content links can be opened.".into());
    }
    let host = url.host_str().unwrap_or_default();
    let allowed = match provider {
        "plex" => {
            let configured = activity
                .storage
                .preference(&provider_url_key(provider))?
                .and_then(|value| reqwest::Url::parse(&value).ok());
            (url.scheme() == "https" && (host == "app.plex.tv" || host.ends_with(".plex.tv")))
                || configured.is_some_and(|configured| {
                    matches!(url.scheme(), "http" | "https")
                        && configured.host_str() == url.host_str()
                        && configured.port_or_known_default() == url.port_or_known_default()
                })
        }
        "trakt" => url.scheme() == "https" && (host == "trakt.tv" || host.ends_with(".trakt.tv")),
        "jellyfin" => {
            let configured = activity
                .storage
                .preference(&provider_url_key(provider))?
                .and_then(|value| reqwest::Url::parse(&value).ok());
            configured.is_some_and(|configured| {
                configured.host_str() == url.host_str()
                    && configured.port_or_known_default() == url.port_or_known_default()
            })
        }
        _ => false,
    };
    if !allowed {
        return Err("The content link does not match the configured provider.".into());
    }
    Ok(url.to_string())
}

pub fn publish(app: &AppHandle) {
    let Some(activity) = app.try_state::<Activity>() else {
        return;
    };
    match activity.snapshot() {
        Ok(snapshot) => {
            let _ = app.emit("activity-updated", snapshot);
        }
        Err(error) => eprintln!("Activity snapshot: {error}"),
    }
}

pub fn record_launch(app: &AppHandle, name: &str) {
    let Some(activity) = app.try_state::<Activity>() else {
        return;
    };
    match activity.record_launch(name) {
        Ok(Some(_)) => publish(app),
        Ok(None) => {}
        Err(error) => eprintln!("Activity launch: {error}"),
    }
}

pub fn record_youtube(app: &AppHandle, report: &PlaybackReport) {
    let Some(activity) = app.try_state::<Activity>() else {
        return;
    };
    match activity.record_youtube(report) {
        Ok(Some(_)) => publish(app),
        Ok(None) => {}
        Err(error) => eprintln!("Activity YouTube: {error}"),
    }
}

pub fn record_spotify(
    app: &AppHandle,
    title: &str,
    artist: &str,
    track_id: Option<&str>,
    artwork: Option<&str>,
) {
    let Some(activity) = app.try_state::<Activity>() else {
        return;
    };
    match activity.record_spotify(title, artist, track_id, artwork) {
        Ok(Some(_)) => publish(app),
        Ok(None) => {}
        Err(error) => eprintln!("Activity Spotify: {error}"),
    }
}

#[tauri::command]
pub fn activity_snapshot(activity: State<'_, Activity>) -> Result<ActivitySnapshot, String> {
    activity.snapshot()
}

#[tauri::command]
pub async fn refresh_provider(app: AppHandle, provider: String) -> Result<RefreshResult, String> {
    let activity = app.state::<Activity>();
    let result = activity.refresh(&provider).await?;
    let _ = app.emit("activity-updated", &result.snapshot);
    Ok(result)
}

#[tauri::command]
pub async fn open_activity(
    app: AppHandle,
    id: String,
    fullscreen: bool,
) -> Result<OpenOutcome, String> {
    let activity = app.state::<Activity>();
    let outcome = activity.open(&app, &id, fullscreen).await?;
    publish(&app);
    Ok(outcome)
}

#[tauri::command]
pub fn clear_activity(
    app: AppHandle,
    activity: State<'_, Activity>,
    item_id: Option<String>,
    provider: Option<String>,
) -> Result<ActivitySnapshot, String> {
    activity.clear(item_id.as_deref(), provider.as_deref())?;
    let snapshot = activity.snapshot()?;
    let _ = app.emit("activity-updated", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn set_activity_tracking(
    app: AppHandle,
    activity: State<'_, Activity>,
    enabled: bool,
) -> Result<ActivitySnapshot, String> {
    activity.set_tracking(enabled)?;
    let snapshot = activity.snapshot()?;
    let _ = app.emit("activity-updated", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn set_provider_enabled(
    app: AppHandle,
    activity: State<'_, Activity>,
    provider: String,
    enabled: bool,
) -> Result<ActivitySnapshot, String> {
    activity.set_provider_enabled(&provider, enabled)?;
    let snapshot = activity.snapshot()?;
    let _ = app.emit("activity-updated", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn set_provider_hidden(
    app: AppHandle,
    activity: State<'_, Activity>,
    provider: String,
    hidden: bool,
) -> Result<ActivitySnapshot, String> {
    activity.set_provider_hidden(&provider, hidden)?;
    let snapshot = activity.snapshot()?;
    let _ = app.emit("activity-updated", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn save_provider_config(
    app: AppHandle,
    activity: State<'_, Activity>,
    provider: String,
    config: ProviderConfigInput,
) -> Result<ActivitySnapshot, String> {
    activity.save_config(&provider, &config)?;
    let snapshot = activity.snapshot()?;
    let _ = app.emit("activity-updated", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn disconnect_provider(
    app: AppHandle,
    activity: State<'_, Activity>,
    provider: String,
) -> Result<ActivitySnapshot, String> {
    activity.disconnect(&provider)?;
    let snapshot = activity.snapshot()?;
    let _ = app.emit("activity-updated", &snapshot);
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::storage::Storage;

    fn activity() -> Activity {
        Activity {
            storage: Storage::memory().unwrap(),
            refresh_revision: Mutex::new(RefreshState::default()),
        }
    }

    #[test]
    fn disabling_tracking_stops_new_records() {
        let activity = activity();
        assert!(activity.record_launch("Spotify").unwrap().is_some());
        activity.set_tracking(false).unwrap();
        assert!(activity.record_launch("Spotify").unwrap().is_none());
        assert_eq!(activity.snapshot().unwrap().items.len(), 1);
    }

    #[test]
    fn hidden_providers_are_not_returned_to_the_home_screen() {
        let activity = activity();
        activity.record_launch("Netflix").unwrap();
        activity.set_provider_hidden("netflix", true).unwrap();
        let snapshot = activity.snapshot().unwrap();
        assert!(snapshot.items.is_empty());
        assert!(snapshot
            .providers
            .iter()
            .any(|provider| provider.id == "netflix" && provider.hidden));
    }

    #[test]
    fn provider_state_is_honest_before_validation() {
        let activity = activity();
        activity
            .storage
            .set_preference("provider.plex.url", "http://plex.local:32400")
            .unwrap();
        activity
            .storage
            .set_bool_preference("provider.plex.tokenSaved", true)
            .unwrap();
        let status = activity
            .snapshot()
            .unwrap()
            .providers
            .into_iter()
            .find(|provider| provider.id == "plex")
            .unwrap();
        assert_eq!(status.state, ProviderState::NotValidated);
        assert!(!status.validated);
        assert!(status.configured);
    }

    #[test]
    fn safe_links_are_limited_to_the_configured_service() {
        let store = activity();
        store
            .storage
            .set_preference("provider.jellyfin.url", "http://jellyfin.local:8096")
            .unwrap();
        store
            .storage
            .set_preference("provider.plex.url", "http://plex.local:32400")
            .unwrap();
        assert!(safe_target(&store, "trakt", "https://trakt.tv/movies/example").is_ok());
        assert!(safe_target(&store, "trakt", "https://example.com/movie").is_err());
        assert!(safe_target(&store, "plex", "http://plex.local:32400/web/index.html").is_ok());
        assert!(safe_target(&store, "plex", "http://other.local:32400/web").is_err());
        assert!(safe_target(
            &store,
            "jellyfin",
            "http://jellyfin.local:8096/web/index.html#/details?id=1"
        )
        .is_ok());
        assert!(safe_target(
            &store,
            "jellyfin",
            "http://other.local:8096/web/index.html#/details?id=1"
        )
        .is_err());
    }
    #[test]
    fn hidden_history_does_not_consume_the_visible_limit() {
        let activity = activity();
        activity
            .storage
            .record_launch("Netflix", "2025-01-01T00:00:00.000Z")
            .unwrap();
        for index in 0..50 {
            let report = PlaybackReport {
                video_id: format!("{index:011}"),
                title: "Video".into(),
                channel: None,
                artwork: None,
                position_seconds: 10.0,
                duration_seconds: 100.0,
                completed: false,
            };
            activity.record_youtube(&report).unwrap();
        }
        activity.set_provider_hidden("youtube", true).unwrap();
        let snapshot = activity.snapshot().unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].title, "Netflix");
    }

    #[test]
    fn delayed_sync_cannot_undo_history_or_connection_changes() {
        for change in 0..4 {
            let activity = activity();
            let revision = activity.refresh_revision.lock().unwrap().begin("plex");
            let mut stale = activity.record_launch("Netflix").unwrap().unwrap();
            stale.id = "plex:stale".into();
            stale.provider = "plex".into();
            match change {
                0 => activity.clear(None, None).unwrap(),
                1 => {
                    activity.set_tracking(false).unwrap();
                    activity.set_tracking(true).unwrap();
                }
                2 => {
                    activity.set_provider_enabled("plex", false).unwrap();
                    activity.set_provider_enabled("plex", true).unwrap();
                }
                _ => activity
                    .save_config(
                        "plex",
                        &ProviderConfigInput {
                            server_url: Some("http://new-server:32400".into()),
                            user_id: None,
                            client_id: None,
                            token: None,
                        },
                    )
                    .unwrap(),
            }
            let data = providers::ProviderRefresh {
                items: vec![stale],
                resume_ids: Some(vec![]),
            };
            assert!(activity.finish_refresh("plex", revision, Ok(data)).is_err());
            assert!(activity
                .finish_refresh("plex", revision, Err("Old connection failed".into()))
                .is_err());
            assert!(activity.storage.get("plex:stale").unwrap().is_none());
            assert!(activity
                .storage
                .preference(&provider_success_key("plex"))
                .unwrap()
                .is_none());
            assert!(activity
                .storage
                .preference(&provider_error_key("plex"))
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn newer_sync_supersedes_only_the_same_provider() {
        let activity = activity();
        let old = activity.refresh_revision.lock().unwrap().begin("plex");
        let jellyfin = activity.refresh_revision.lock().unwrap().begin("jellyfin");
        let current = activity.refresh_revision.lock().unwrap().begin("plex");
        assert!(activity
            .finish_refresh("plex", old, Err("Outdated failure".into()))
            .is_err());
        assert!(activity
            .finish_refresh(
                "plex",
                current,
                Ok(providers::ProviderRefresh {
                    items: vec![],
                    resume_ids: None
                })
            )
            .is_ok());
        assert!(activity
            .finish_refresh(
                "jellyfin",
                jellyfin,
                Ok(providers::ProviderRefresh {
                    items: vec![],
                    resume_ids: None
                })
            )
            .is_ok());
    }
}
