use super::{
    client, now, normalized_time, ActivityItem, ActivityType, ProgressSource, ProviderConfig,
    ResumeKind,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(|value| value.as_f64().or_else(|| value.as_i64().map(|v| v as f64)))
}

fn item(value: &Value, fallback: &str) -> Option<ActivityItem> {
    let kind = text(value, "type")?;
    let watched_at = normalized_time(text(value, "watched_at"), fallback);
    match kind.as_str() {
        "movie" => {
            let movie = value.get("movie")?;
            let title = text(movie, "title")?;
            let ids = movie.get("ids").unwrap_or(&Value::Null);
            let id = text(ids, "trakt")?.to_string();
            let slug = text(ids, "slug");
            Some(ActivityItem {
                id: format!("trakt:movie:{id}"),
                provider: "trakt".into(),
                provider_item_id: Some(id),
                profile_id: None,
                item_type: ActivityType::Movie,
                title,
                subtitle: number(movie, "year").map(|year| year as u32).map(|year| year.to_string()),
                artwork: None,
                last_activity_at: watched_at,
                progress_seconds: None,
                duration_seconds: None,
                progress_source: ProgressSource::Provider,
                completed: Some(true),
                can_resume: false,
                launch_target: Some(match slug {
                    Some(slug) => format!("https://trakt.tv/movies/{slug}"),
                    None => "https://trakt.tv/movies".into(),
                }),
                resume_kind: ResumeKind::Content,
                launch_count: None,
            })
        }
        "episode" => {
            let episode = value.get("episode")?;
            let show = value.get("show")?;
            let title = text(show, "title")?;
            let ids = episode.get("ids").unwrap_or(&Value::Null);
            let id = text(ids, "trakt")?.to_string();
            let season = number(episode, "season").unwrap_or(0.0) as u32;
            let index = number(episode, "number").unwrap_or(0.0) as u32;
            let episode_title = text(episode, "title").unwrap_or_else(|| format!("Episode {index}"));
            let show_slug = show.get("ids").and_then(|ids| text(ids, "slug"));
            Some(ActivityItem {
                id: format!("trakt:episode:{id}"),
                provider: "trakt".into(),
                provider_item_id: Some(id),
                profile_id: None,
                item_type: ActivityType::Episode,
                title,
                subtitle: Some(format!("S{season} E{index} · {episode_title}")),
                artwork: None,
                last_activity_at: watched_at,
                progress_seconds: None,
                duration_seconds: None,
                progress_source: ProgressSource::Provider,
                completed: Some(true),
                can_resume: false,
                launch_target: show_slug.map(|slug| {
                    format!("https://trakt.tv/shows/{slug}/seasons/{season}/episodes/{index}")
                }),
                resume_kind: ResumeKind::Content,
                launch_count: None,
            })
        }
        _ => None,
    }
}

pub async fn refresh(config: &ProviderConfig) -> Result<Vec<ActivityItem>, String> {
    let client_id = config
        .client_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or("Add a Trakt client ID in Activity settings.")?;
    let token = config
        .token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or("Add an authorized Trakt access token in Activity settings.")?;
    let response = client()
        .get("https://api.trakt.tv/sync/history")
        .query(&[("limit", "50"), ("extended", "full")])
        .header("trakt-api-version", "2")
        .header("trakt-api-key", client_id)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| format!("Trakt could not be reached: {error}"))?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 | 403 => "Trakt rejected the access token or client ID.".into(),
            status => format!("Trakt returned HTTP {status}."),
        });
    }
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| format!("Trakt returned an unreadable response: {error}"))?;
    let entries = value
        .as_array()
        .ok_or("Trakt returned an unexpected history response.")?;
    let now = now();
    let mut items = BTreeMap::new();
    for entry in entries {
        if let Some(item) = item(entry, &now) {
            items.insert(item.id.clone(), item);
        }
    }
    Ok(items.into_values().collect())
}
