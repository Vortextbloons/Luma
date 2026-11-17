use super::{
    client, now, validated_base, ActivityItem, ActivityType, ProgressSource, ProviderConfig,
    ResumeKind,
};
use serde_json::Value;
use std::collections::BTreeMap;

const TICKS: f64 = 10_000_000.0;

fn headers(request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    request
        .header("X-Emby-Token", token)
        .header("Accept", "application/json")
}

async fn json(request: reqwest::RequestBuilder, provider: &str) -> Result<Value, String> {
    let response = request
        .send()
        .await
        .map_err(|error| format!("{provider} could not be reached: {error}"))?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 | 403 => format!("{provider} rejected the API key."),
            status => format!("{provider} returned HTTP {status}."),
        });
    }
    response
        .json::<Value>()
        .await
        .map_err(|error| format!("{provider} returned an unreadable response: {error}"))
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

fn timestamp(value: Option<String>, fallback: &str) -> String {
    value
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| {
            value
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        })
        .unwrap_or_else(|| fallback.to_string())
}

async fn user_id(base: &str, token: &str, configured: Option<&str>) -> Result<String, String> {
    if let Some(value) = configured.filter(|value| !value.trim().is_empty()) {
        return Ok(value.trim().to_string());
    }
    let value = json(
        headers(client().get(format!("{base}/Users")), token),
        "Jellyfin",
    )
    .await?;
    let users = value
        .as_array()
        .ok_or("Jellyfin did not return a user list. Set the user ID manually.")?;
    if users.len() == 1 {
        if let Some(id) = text(&users[0], "Id") {
            return Ok(id);
        }
    }
    let names = users
        .iter()
        .filter_map(|user| text(user, "Name"))
        .take(4)
        .collect::<Vec<_>>();
    Err(format!(
        "Set a Jellyfin user ID. Available users: {}",
        if names.is_empty() {
            "none returned".into()
        } else {
            names.join(", ")
        }
    ))
}

fn item(value: &Value, user: &str, base: &str, fallback: &str) -> Option<ActivityItem> {
    let id = text(value, "Id")?;
    let kind = text(value, "Type").unwrap_or_else(|| "Video".into());
    let item_type = match kind.as_str() {
        "Movie" => ActivityType::Movie,
        "Episode" => ActivityType::Episode,
        _ => ActivityType::Video,
    };
    let name = text(value, "Name").unwrap_or_default();
    let series = text(value, "SeriesName");
    let title = if item_type == ActivityType::Episode {
        series.clone().unwrap_or_else(|| name.clone())
    } else {
        name.clone()
    };
    if title.is_empty() {
        return None;
    }
    let subtitle = if item_type == ActivityType::Episode {
        let season = number(value, "ParentIndexNumber").unwrap_or(0.0) as u32;
        let episode = number(value, "IndexNumber").unwrap_or(0.0) as u32;
        Some(format!("S{season} E{episode} · {name}"))
    } else {
        number(value, "ProductionYear").map(|year| (year as u32).to_string())
    };
    let user_data = value.get("UserData").unwrap_or(&Value::Null);
    let played = user_data
        .get("Played")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let position = number(user_data, "PlaybackPositionTicks").unwrap_or(0.0) / TICKS;
    let duration = number(value, "RunTimeTicks").unwrap_or(0.0) / TICKS;
    let progress = (!played && position > 0.0 && duration > 0.0).then_some(position);
    let last_activity = timestamp(
        user_data
            .get("LastPlayedDate")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        fallback,
    );
    Some(ActivityItem {
        id: format!("jellyfin:{user}:{id}"),
        provider: "jellyfin".into(),
        provider_item_id: Some(id.clone()),
        profile_id: Some(user.to_string()),
        item_type,
        title,
        subtitle,
        artwork: None,
        last_activity_at: last_activity,
        progress_seconds: progress,
        duration_seconds: (duration > 0.0).then_some(duration),
        progress_source: ProgressSource::Provider,
        completed: Some(played),
        can_resume: false,
        launch_target: Some(format!("{base}/web/index.html#/details?id={id}")),
        resume_kind: ResumeKind::Content,
        launch_count: None,
    })
}

fn resume_ids(value: &Value, user: &str, base: &str, now: &str) -> Option<Vec<String>> {
    let entries = value.get("Items")?.as_array()?;
    let total = value.get("TotalRecordCount")?.as_u64()?;
    if total > entries.len() as u64 {
        return None;
    }
    entries
        .iter()
        .map(|value| item(value, user, base, now).map(|item| item.id))
        .collect()
}

pub async fn refresh(config: &ProviderConfig) -> Result<super::ProviderRefresh, String> {
    let base = validated_base(config.server_url.as_deref().unwrap_or(""))?;
    let token = config
        .token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or("Add a Jellyfin API key in Activity settings.")?;
    let user = user_id(&base, token, config.user_id.as_deref()).await?;
    let now = now();
    let mut items = BTreeMap::new();

    let resume = headers(
        client()
            .get(format!("{base}/Users/{user}/Items/Resume"))
            .query(&[("Limit", "50"), ("MediaTypes", "Video")]),
        token,
    );
    let played = headers(
        client().get(format!("{base}/Users/{user}/Items")).query(&[
            ("SortBy", "DatePlayed"),
            ("SortOrder", "Descending"),
            ("Filters", "IsPlayed"),
            ("Recursive", "true"),
            ("IncludeItemTypes", "Movie,Episode"),
            ("Limit", "50"),
        ]),
        token,
    );
    let (resume, played) = tokio::join!(json(resume, "Jellyfin"), json(played, "Jellyfin"));

    if let (Err(resume_error), Err(_)) = (&resume, &played) {
        return Err(resume_error.clone());
    }

    let resume_ids = resume
        .as_ref()
        .ok()
        .and_then(|value| resume_ids(value, &user, &base, &now));

    for value in [resume.ok(), played.ok()].into_iter().flatten() {
        if let Some(entries) = value.get("Items").and_then(Value::as_array) {
            for entry in entries {
                if let Some(item) = item(entry, &user, &base, &now) {
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
    fn only_complete_resume_responses_can_clear_old_progress() {
        let entry = json!({"Id": "42", "Name": "Example", "Type": "Movie", "RunTimeTicks": 100000000,
            "UserData": {"PlaybackPositionTicks": 50000000}});
        assert_eq!(
            resume_ids(
                &json!({"Items": [entry], "TotalRecordCount": 1}),
                "user",
                "base",
                "now"
            ),
            Some(vec!["jellyfin:user:42".into()])
        );
        assert_eq!(
            resume_ids(
                &json!({"Items": [entry], "TotalRecordCount": 51}),
                "user",
                "base",
                "now"
            ),
            None
        );
        assert_eq!(
            resume_ids(
                &json!({"Items": [], "TotalRecordCount": 0}),
                "user",
                "base",
                "now"
            ),
            Some(vec![])
        );
        assert_eq!(
            resume_ids(&json!({"Items": []}), "user", "base", "now"),
            None
        );
    }
}
