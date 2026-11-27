use serde_json::{json, Value};
use std::time::Duration;
use tauri::Manager;
use url::Url;

const CINEMETA: &str = "https://v3-cinemeta.strem.io/catalog";

fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

async fn cinemeta(client: &reqwest::Client, kind: &str, query: &str) -> Result<Vec<Value>, ()> {
    let url = format!("{CINEMETA}/{kind}/top/search={}.json", encode(query));
    let response = client.get(url).send().await.map_err(|_| ())?.error_for_status().map_err(|_| ())?;
    let data: Value = response.json().await.map_err(|_| ())?;
    let label = if kind == "movie" { "Movie" } else { "Show" };
    Ok(data
        .get("metas")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("id")?.as_str()?;
                    let title = item.get("name")?.as_str()?;
                    let year = item.get("year").and_then(Value::as_str).unwrap_or("");
                    let poster = item
                        .get("poster")
                        .or_else(|| item.get("background"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .replace("/poster/small/", "/poster/medium/");
                    Some(json!({
                        "kind": kind,
                        "title": title,
                        "subtitle": if year.is_empty() { label.to_string() } else { format!("{label} · {year}") },
                        "image": poster,
                        "target": { "kind": kind, "id": id }
                    }))
                })
                .take(8)
                .collect()
        })
        .unwrap_or_default())
}

fn stream_east_url(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
        return None;
    }
    let host = url.host_str()?;
    if host == "golygoals.com"
        && matches!(url.path(), "/streameast.html" | "/streame.html")
        && url.query_pairs().any(|(key, _)| key == "id" || key == "key")
    {
        return Some(url.to_string());
    }
    if host == "streamec.st" {
        let name = url.path().strip_prefix("/embed/ch")?.strip_suffix(".php")?;
        if !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some(url.to_string());
        }
    }
    None
}

fn sport_league(api_id: &str, title: &str, sport_name: &str) -> String {
    let id = api_id.to_ascii_lowercase();
    for (needle, label) in [
        ("nba", "NBA"),
        ("wnba", "WNBA"),
        ("nfl", "NFL"),
        ("mlb", "MLB"),
        ("nhl", "NHL"),
        ("college-football", "CFB"),
        ("mens-college-basketball", "NCAA Men"),
        ("womens-college-basketball", "NCAA Women"),
    ] {
        if id.contains(needle) {
            return label.into();
        }
    }
    if title.eq_ignore_ascii_case("NFL RedZone") {
        return "NFL".into();
    }
    if sport_name.trim().is_empty() {
        "Other".into()
    } else {
        sport_name.trim().into()
    }
}

#[derive(Clone)]
struct SportEvent {
    id: String,
    url: String,
    sources: Vec<String>,
    league: String,
    title: String,
    teams: Vec<(String, String)>,
    time: String,
}

fn parse_streameast(value: &Value) -> Vec<SportEvent> {
    value
        .get("matches")
        .and_then(Value::as_array)
        .map(|matches| {
            matches
                .iter()
                .filter_map(|item| {
                    let mut sources = Vec::new();
                    for stream in item.get("streams").and_then(Value::as_array).into_iter().flatten() {
                        if let Some(url) = stream.get("url").and_then(Value::as_str).and_then(stream_east_url) {
                            if !sources.contains(&url) {
                                sources.push(url);
                            }
                        }
                    }
                    if sources.is_empty() {
                        return None;
                    }
                    sources.truncate(12);
                    let api_id = item.get("api_id").and_then(Value::as_str).unwrap_or("");
                    let sport_name = item.get("sport_name").and_then(Value::as_str).unwrap_or("");
                    let title_text = item.get("title").and_then(Value::as_str).unwrap_or("");
                    let league = sport_league(api_id, title_text, sport_name);
                    let mut teams = Vec::new();
                    for key in ["home", "away"] {
                        if let Some(team) = item.get(key) {
                            if let Some(name) = team.get("name").and_then(Value::as_str).map(str::trim).filter(|name| !name.is_empty()) {
                                let logo = team
                                    .get("badge")
                                    .and_then(Value::as_str)
                                    .filter(|value| value.starts_with("https://"))
                                    .unwrap_or("");
                                teams.push((name.to_string(), logo.to_string()));
                            }
                        }
                    }
                    if teams.is_empty() && !title_text.trim().is_empty() {
                        teams.push((title_text.trim().to_string(), String::new()));
                    }
                    if teams.is_empty() {
                        return None;
                    }
                    let live = item.get("live").and_then(Value::as_bool).unwrap_or(false);
                    let time = if live { "In Progress".into() } else { "Upcoming".into() };
                    let title = teams.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(" vs ");
                    Some(SportEvent {
                        id: format!("streameast:{}", item.get("id").and_then(Value::as_str).unwrap_or(title_text)),
                        url: sources[0].clone(),
                        sources,
                        league,
                        title,
                        teams,
                        time,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn clean(value: &str) -> String {
    unescape(value).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_after(row: &str, marker: &str) -> Option<String> {
    let start = row.find(marker)?;
    let open = row[start..].find('>')? + start + 1;
    let close = row[open..].find('<')? + open;
    Some(clean(&row[open..close]))
}

fn parse_sports_html(html: &str) -> Vec<SportEvent> {
    let mut events = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<a class=\"row MaclariListele") {
        rest = &rest[start..];
        let end = rest.find("</a>").unwrap_or(rest.len());
        let row = &rest[..end];
        rest = &rest[end.saturating_add(4).min(rest.len())..];
        let Some(href) = attr(row, "href") else { continue };
        let Ok(url) = Url::parse(&href) else { continue };
        if url.scheme() != "https"
            || url.host_str() != Some("isportsurge.ws")
            || !(url.path().starts_with("/watch/") || url.path().starts_with("/event/"))
        {
            continue;
        }
        let mut teams = Vec::new();
        let mut cursor = row;
        while let Some(position) = cursor.find("team-name-event-row") {
            cursor = &cursor[position..];
            let block_end = cursor.find("</div>").unwrap_or(cursor.len());
            let block = &cursor[..block_end];
            let logo = block
                .find("<img")
                .map(|start| {
                    let end = block[start..].find('>').map(|value| value + start).unwrap_or(block.len());
                    &block[start..end]
                })
                .and_then(|tag| attr(tag, "src"))
                .unwrap_or_default();
            let name = block
                .find("<span")
                .and_then(|start| {
                    let tag = &block[start..];
                    let open = tag.find('>')? + 1;
                    let close = tag.find("</span>")?;
                    Some(clean(&tag[open..close]))
                })
                .filter(|name| !name.is_empty())
                .or_else(|| {
                    block
                        .find("<img")
                        .map(|start| {
                            let end = block[start..].find('>').map(|value| value + start).unwrap_or(block.len());
                            &block[start..end]
                        })
                        .and_then(|tag| attr(tag, "alt"))
                        .map(|name| clean(&name))
                });
            if let Some(name) = name.filter(|name| !name.is_empty()) {
                teams.push((name, logo));
            }
            cursor = &cursor[block_end.saturating_add(6).min(cursor.len())..];
        }
        if teams.is_empty() {
            continue;
        }
        let time = text_after(row, "class=\"time-badge").unwrap_or_default();
        let league = text_after(row, "ListelemeDuzen").filter(|value| !value.is_empty()).unwrap_or_else(|| "Other".into());
        let title = teams.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(" vs ");
        events.push(SportEvent {
            id: url.to_string(),
            url: url.to_string(),
            sources: vec![url.to_string()],
            league,
            title,
            teams,
            time,
        });
    }
    events
}

fn sport_key(event: &SportEvent) -> String {
    if event.teams.len() == 2 {
        let mut names: Vec<String> = event
            .teams
            .iter()
            .map(|(name, _)| name.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect())
            .collect();
        names.sort();
        format!("{}:{}", event.league.to_ascii_lowercase(), names.join(":"))
    } else {
        format!(
            "{}:{}",
            event.league.to_ascii_lowercase(),
            event.title.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>()
        )
    }
}

fn merge_sports(primary: Vec<SportEvent>, fallback: Vec<SportEvent>) -> Vec<SportEvent> {
    let mut consumed = std::collections::HashSet::new();
    let mut result: Vec<SportEvent> = primary
        .into_iter()
        .map(|mut event| {
            let key = sport_key(&event);
            let matches: Vec<&SportEvent> = fallback.iter().filter(|candidate| sport_key(candidate) == key).collect();
            if matches.len() == 1 {
                consumed.insert(matches[0].id.clone());
                if !event.sources.contains(&matches[0].url) {
                    event.sources.push(matches[0].url.clone());
                    event.sources.truncate(12);
                }
            }
            event
        })
        .collect();
    for event in fallback {
        if !consumed.contains(&event.id) && !result.iter().any(|existing| existing.id == event.id) {
            result.push(event);
        }
    }
    result
}

pub(crate) async fn search(app: &tauri::AppHandle, text: String) -> Result<Value, String> {
    let query = text.trim();
    if query.is_empty() || query.len() > 1024 {
        return Err("Enter a search of up to 1024 bytes.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Luma Display Box")
        .build()
        .map_err(|error| error.to_string())?;
    let history = app
        .state::<crate::activity::Activity>()
        .snapshot()
        .map(|snapshot| snapshot.items)
        .unwrap_or_default();
    let (movies, shows, videos, east, html) = tokio::join!(
        cinemeta(&client, "movie", query),
        cinemeta(&client, "series", query),
        crate::commands::youtube::youtube_search(query.to_string()),
        crate::commands::sports::sports_streameast_listing(app.clone(), Some(false)),
        crate::commands::sports::sports_listing(),
    );
    let needle = query.to_lowercase();
    let mut results = Vec::new();
    let mut unavailable = Vec::new();
    match movies {
        Ok(items) => results.extend(items),
        Err(_) => unavailable.push("Movies"),
    }
    match shows {
        Ok(items) => results.extend(items),
        Err(_) => unavailable.push("Shows"),
    }
    match videos {
        Ok(items) => {
            for item in items.into_iter().take(12) {
                results.push(json!({
                    "kind": "video",
                    "title": item.title,
                    "subtitle": "YouTube",
                    "image": format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", item.id),
                    "target": { "kind": "video", "id": item.id }
                }));
            }
        }
        Err(_) => unavailable.push("YouTube"),
    }
    let sports = match (east, html) {
        (east, html) if east.is_err() && html.is_err() => {
            unavailable.push("Sports");
            Vec::new()
        }
        (east, html) => {
            let primary = east.map(|value| parse_streameast(&value)).unwrap_or_default();
            let fallback = html.map(|value| parse_sports_html(&value)).unwrap_or_default();
            merge_sports(primary, fallback)
        }
    };
    for event in sports
        .into_iter()
        .filter(|event| format!("{} {}", event.title, event.league).to_lowercase().contains(&needle))
        .take(12)
    {
        results.push(json!({
            "kind": "sports",
            "title": event.title,
            "subtitle": format!("{} · {}", event.league, event.time),
            "image": event.teams.first().map(|(_, logo)| logo.clone()).unwrap_or_default(),
            "target": { "kind": "sports", "url": event.url, "title": event.title, "sources": event.sources }
        }));
    }
    for item in history
        .into_iter()
        .filter(|item| item.title.to_lowercase().contains(&needle))
        .filter(|item| item.resume_kind != crate::activity::models::ResumeKind::Unavailable)
        .take(10)
    {
        results.push(json!({
            "kind": "activity",
            "title": item.title,
            "subtitle": item.provider,
            "image": item.artwork.unwrap_or_default(),
            "target": { "kind": "activity", "id": item.id }
        }));
    }
    Ok(json!({ "results": results, "unavailable": unavailable }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_encoding_keeps_path_safe_characters() {
        assert_eq!(encode("a b/c"), "a%20b%2Fc");
        assert_eq!(encode("Top-Gun_2.0~"), "Top-Gun_2.0~");
    }

    #[test]
    fn streameast_listing_reads_sources_and_teams() {
        let value = json!({"matches": [{
            "id": "1",
            "title": "A vs B",
            "sport_name": "Basketball",
            "api_id": "league-nba-123",
            "live": true,
            "home": {"name": "A", "badge": "https://cdn/a.png"},
            "away": {"name": "B", "badge": "https://cdn/b.png"},
            "streams": [
                {"url": "https://golygoals.com/streameast.html?id=1"},
                {"url": "https://golygoals.com/streameast.html?id=1"},
                {"url": "https://evil.example/stream"}
            ]
        }]});
        let events = parse_streameast(&value);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].league, "NBA");
        assert_eq!(events[0].title, "A vs B");
        assert_eq!(events[0].sources, vec!["https://golygoals.com/streameast.html?id=1"]);
        assert_eq!(events[0].time, "In Progress");
    }

    #[test]
    fn sports_html_reads_rows_and_rejects_external_links() {
        let html = r#"
            <a class="row MaclariListele align-items-center" href="https://isportsurge.ws/watch/nba/a-b/1">
              <span class="time-badge schedule-ss">In Progress</span>
              <div class="team-name-event-row"><img alt="A" src="https://cdn/a.png"><span> A </span></div>
              <div class="team-name-event-row"><img alt="B" src="https://cdn/b.png"><span> B </span></div>
              <div class="col-md-2 col-2 ListelemeDuzen text-center">NBA</div>
            </a>
            <a class="row MaclariListele" href="https://evil.example/watch/nba/c-d/2">
              <div class="team-name-event-row"><span>C</span></div>
            </a>
        "#;
        let events = parse_sports_html(html);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].teams.len(), 2);
        assert_eq!(events[0].league, "NBA");
        assert_eq!(events[0].title, "A vs B");
    }

    #[test]
    fn fallback_source_is_merged_once() {
        let primary = vec![SportEvent {
            id: "1".into(),
            url: "https://golygoals.com/streameast.html?id=1".into(),
            sources: vec!["https://golygoals.com/streameast.html?id=1".into()],
            league: "NBA".into(),
            title: "A vs B".into(),
            teams: vec![("A".into(), String::new()), ("B".into(), String::new())],
            time: "In Progress".into(),
        }];
        let fallback = vec![SportEvent {
            id: "2".into(),
            url: "https://isportsurge.ws/watch/nba/a-b/2".into(),
            sources: vec!["https://isportsurge.ws/watch/nba/a-b/2".into()],
            league: "NBA".into(),
            title: "A vs B".into(),
            teams: vec![("A".into(), String::new()), ("B".into(), String::new())],
            time: "In Progress".into(),
        }];
        let merged = merge_sports(primary, fallback);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].sources.len(), 2);
    }
}
