use super::{
    client, from_unix, now, validated_base, ActivityItem, ActivityType, ProgressSource,
    ProviderConfig, ResumeKind,
};
use serde_json::Value;
use std::collections::BTreeMap;

async fn json(url: &str, token: &str) -> Result<Value, String> {
    let response = client()
        .get(url)
        .header("X-Plex-Token", token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| format!("Plex could not be reached: {error}"))?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 | 403 => "Plex rejected the token. Create a new X-Plex-Token and try again.".into(),
            status => format!("Plex returned HTTP {status}."),
        });
    }
    response
        .json::<Value>()
        .await
        .map_err(|error| format!("Plex returned an unreadable response: {error}"))
}

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

fn item(value: &Value, machine: &str, base: &str, fallback_time: &str) -> Option<ActivityItem> {
    let rating_key = text(value, "ratingKey")?;
    let kind = text(value, "type").unwrap_or_else(|| "video".into());
    let item_type = match kind.as_str() {
        "movie" => ActivityType::Movie,
        "episode" => ActivityType::Episode,
        _ => ActivityType::Video,
    };
    let episode_title = text(value, "title").unwrap_or_default();
    let show_title = text(value, "grandparentTitle");
    let title = if item_type == ActivityType::Episode {
        show_title.clone().unwrap_or_else(|| episode_title.clone())
    } else {
        episode_title.clone()
    };
    if title.is_empty() {
        return None;
    }
    let subtitle = if item_type == ActivityType::Episode {
        let season = number(value, "parentIndex").unwrap_or(0.0) as u32;
        let episode = number(value, "index").unwrap_or(0.0) as u32;
        Some(format!("S{season} E{episode} · {episode_title}"))
    } else {
        text(value, "year")
    };
    let view_offset = number(value, "viewOffset").unwrap_or(0.0);
    let duration = number(value, "duration").unwrap_or(0.0);
    let progress = (view_offset > 0.0 && duration > 0.0).then_some(view_offset / 1000.0);
    let duration_seconds = (duration > 0.0).then_some(duration / 1000.0);
    let last_activity = number(value, "lastViewedAt")
        .map(|value| from_unix(value as i64))
        .unwrap_or_else(|| fallback_time.to_string());
    let (launch_target, resume_kind) = if machine.is_empty() {
        (Some(base.to_string()), ResumeKind::Content)
    } else {
        (
            Some(format!(
                "https://app.plex.tv/desktop/#!/server/{machine}/details?key=%2Flibrary%2Fmetadata%2F{rating_key}"
            )),
            ResumeKind::Content,
        )
    };
    Some(ActivityItem {
        id: format!(
            "plex:{}:{rating_key}",
            if machine.is_empty() {
                "server"
            } else {
                machine
            }
        ),
        provider: "plex".into(),
        provider_item_id: Some(rating_key),
        profile_id: Some(if machine.is_empty() {
            base.to_string()
        } else {
            machine.to_string()
        }),
        item_type,
        title,
        subtitle,
        artwork: None,
        last_activity_at: last_activity,
        progress_seconds: progress,
        duration_seconds,
        progress_source: ProgressSource::Provider,
        completed: Some(number(value, "viewCount").unwrap_or(0.0) > 0.0 && view_offset <= 0.0),
        can_resume: false,
        launch_target,
        resume_kind,
        launch_count: None,
    })
}

fn resume_ids(value: &Value, machine: &str, base: &str, now: &str) -> Option<Vec<String>> {
    let container = value.get("MediaContainer")?;
    let empty = Vec::new();
    let entries = container
        .get("Metadata")
        .and_then(Value::as_array)
        .or_else(|| (container.get("size").and_then(Value::as_u64) == Some(0)).then_some(&empty))?;
    if container.get("offset").and_then(Value::as_u64).unwrap_or(0) != 0
        || container
            .get("totalSize")
            .and_then(Value::as_u64)
            .unwrap_or(entries.len() as u64)
            > entries.len() as u64
    {
        return None;
    }
    entries
        .iter()
        .map(|value| item(value, machine, base, now).map(|item| item.id))
        .collect()
}

pub async fn refresh(config: &ProviderConfig) -> Result<super::ProviderRefresh, String> {
    let base = validated_base(config.server_url.as_deref().unwrap_or(""))?;
    let token = config
        .token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or("Add a Plex token in Activity settings.")?;
    let now = now();
    let identity = json(&format!("{base}/identity"), token).await?;
    let machine = identity
        .pointer("/MediaContainer/machineIdentifier")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut items = BTreeMap::new();
    let deck_url = format!("{base}/library/onDeck");
    let recent_url = format!("{base}/library/recentlyViewed");
    let (on_deck, recently_viewed) =
        tokio::join!(json(&deck_url, token), json(&recent_url, token),);

    if let (Err(deck_error), Err(_)) = (&on_deck, &recently_viewed) {
        return Err(deck_error.clone());
    }

    let resume_ids = on_deck
        .as_ref()
        .ok()
        .and_then(|value| resume_ids(value, &machine, &base, &now));

    for value in [on_deck.ok(), recently_viewed.ok()].into_iter().flatten() {
        if let Some(metadata) = value
            .pointer("/MediaContainer/Metadata")
            .and_then(Value::as_array)
        {
            for entry in metadata {
                if let Some(item) = item(entry, &machine, &base, &now) {
                    items.entry(item.id.clone()).or_insert(item);
                }
            }
        }
    }

    Ok(super::ProviderRefresh {
        items: items.into_values().collect(),
        resume_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn on_deck_episode_keeps_provider_progress_and_content_link() {
        let value = json!({
            "ratingKey": "42",
            "type": "episode",
            "title": "Pilot",
            "grandparentTitle": "Example Show",
            "parentIndex": 1,
            "index": 2,
            "viewOffset": 600000,
            "duration": 3000000,
            "lastViewedAt": 1767225600
        });
        let item = item(
            &value,
            "machine",
            "http://plex.local:32400",
            "2026-01-01T00:00:00.000Z",
        )
        .unwrap();
        assert_eq!(item.title, "Example Show");
        assert_eq!(item.subtitle.as_deref(), Some("S1 E2 · Pilot"));
        assert_eq!(item.progress_seconds, Some(600.0));
        assert_eq!(item.duration_seconds, Some(3000.0));
        assert_eq!(item.resume_kind, ResumeKind::Content);
        assert!(!item.can_resume);
        assert!(item
            .launch_target
            .unwrap()
            .contains("%2Flibrary%2Fmetadata%2F42"));
    }

    #[test]
    fn recently_viewed_partial_item_keeps_progress_instead_of_claiming_finished() {
        let value = json!({
            "ratingKey": "43",
            "type": "movie",
            "title": "Partly watched",
            "viewOffset": 120000,
            "duration": 600000,
            "viewCount": 1,
            "lastViewedAt": 1767225600
        });
        let item = item(
            &value,
            "machine",
            "http://plex.local:32400",
            "2026-01-01T00:00:00.000Z",
        )
        .unwrap();
        assert_eq!(item.progress_seconds, Some(120.0));
        assert_eq!(item.duration_seconds, Some(600.0));
        assert_eq!(item.completed, Some(false));
    }
    #[test]
    fn only_complete_resume_responses_can_clear_old_progress() {
        let entry = json!({"ratingKey": "42", "type": "movie", "title": "Example", "viewOffset": 10, "duration": 100});
        let complete = json!({"MediaContainer": {"Metadata": [entry], "size": 1, "totalSize": 1}});
        assert_eq!(
            resume_ids(&complete, "machine", "base", "now"),
            Some(vec!["plex:machine:42".into()])
        );
        let truncated = json!({"MediaContainer": {"Metadata": [entry], "totalSize": 2}});
        assert_eq!(resume_ids(&truncated, "machine", "base", "now"), None);
        assert_eq!(
            resume_ids(
                &json!({"MediaContainer": {"size": 0}}),
                "machine",
                "base",
                "now"
            ),
            Some(vec![])
        );
        assert_eq!(
            resume_ids(&json!({"MediaContainer": {}}), "machine", "base", "now"),
            None
        );
    }
}
