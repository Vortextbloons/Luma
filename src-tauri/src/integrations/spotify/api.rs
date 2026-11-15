use super::{
    auth::Auth,
    models::{
        now_ms, Album, Command, Device, HomeData, PlaybackState, Playlist, Queue, SearchResults,
        Track,
    },
};
use reqwest::{Method, Url};
use serde_json::{json, Value};

const BASE: &str = "https://api.spotify.com/v1";

async fn request(
    auth: &Auth,
    method: Method,
    path: &str,
    query: &[(&str, String)],
    body: Option<Value>,
) -> Result<Option<Value>, String> {
    let mut retried = false;
    loop {
        let token = auth.access_token().await?;
        let mut url = Url::parse(&format!("{BASE}{path}")).map_err(|error| error.to_string())?;
        for (key, value) in query {
            url.query_pairs_mut().append_pair(key, value);
        }
        let mut builder = auth
            .client()
            .request(method.clone(), url)
            .bearer_auth(&token);
        if let Some(ref body) = body {
            builder = builder
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("Spotify could not be reached: {error}"))?;
        let status = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        if status == reqwest::StatusCode::UNAUTHORIZED && !retried {
            retried = true;
            auth.refresh().await?;
            continue;
        }
        let text = response.text().await.map_err(|error| error.to_string())?;
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }
        if status.is_success() {
            if text.trim().is_empty() {
                return Ok(None);
            }
            return serde_json::from_str(&text)
                .map(Some)
                .map_err(|error| format!("Spotify returned an unexpected response: {error}"));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(format!(
                "Spotify rate limit reached. Try again in {} seconds.",
                retry_after.unwrap_or(5)
            ));
        }
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err("Spotify sign-in expired. Reconnect your account in Settings.".into());
        }
        if status == reqwest::StatusCode::FORBIDDEN
            && path.starts_with("/playlists/")
            && path.ends_with("/items")
        {
            return Err("Spotify only allows browsing playlists you own or collaborate on. You can still play this playlist using Play playlist.".into());
        }
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(format!(
                "Spotify rejected this request. Development Mode apps have limited endpoint access. {}",
                message(&text)
            ));
        }
        if status == reqwest::StatusCode::NOT_FOUND && text.contains("NO_ACTIVE_DEVICE") {
            return Err(
                "No active Spotify device. Open Spotify on this PC and start a track first.".into(),
            );
        }
        return Err(format!(
            "Spotify request failed ({status}): {}",
            message(&text)
        ));
    }
}

fn message(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .or_else(|| {
                    value
                        .get("error")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "The request was rejected.".into())
}

pub async fn profile(auth: &Auth) -> Result<Option<String>, String> {
    let value = request(auth, Method::GET, "/me", &[], None).await?;
    Ok(value.and_then(|value| {
        value
            .get("display_name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .filter(|value| !value.is_empty())
    }))
}

pub async fn playback(auth: &Auth) -> Result<PlaybackState, String> {
    let value = request(auth, Method::GET, "/me/player", &[], None).await?;
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(PlaybackState {
            connected: true,
            updated_at: now_ms(),
            ..Default::default()
        });
    };
    let item = value.get("item").filter(|value| !value.is_null());
    let track = item.map(track).unwrap_or_default();
    let device = value.get("device");
    let disallows = value.pointer("/actions/disallows");
    let blocked = |key: &str| {
        disallows
            .and_then(|value| value.get(key))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    };
    let duration_ms = track.duration_ms.or_else(|| {
        value
            .get("item")
            .and_then(|item| item.get("duration_ms"))
            .and_then(|value| value.as_u64())
    });
    let can_seek = !blocked("seeking") && duration_ms.is_some();
    Ok(PlaybackState {
        connected: true,
        player_available: true,
        playing: value
            .get("is_playing")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        track_id: track.id.clone(),
        title: (!track.title.trim().is_empty()).then_some(track.title.clone()),
        artist: (!track.artist.trim().is_empty()).then_some(track.artist.clone()),
        album: (!track.album.trim().is_empty()).then_some(track.album.clone()),
        artwork_url: track.artwork_url.clone(),
        position_ms: value.get("progress_ms").and_then(|value| value.as_u64()),
        duration_ms,
        device_id: device
            .and_then(|device| device.get("id"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        device_name: device
            .and_then(|device| device.get("name"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        can_seek,
        can_skip_next: !blocked("skipping_next"),
        can_skip_previous: !blocked("skipping_prev"),
        can_toggle: !blocked("pausing") || !blocked("resuming"),
        shuffle: value.get("shuffle_state").and_then(|value| value.as_bool()),
        repeat: value
            .get("repeat_state")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        volume: device
            .and_then(|device| device.get("volume_percent"))
            .and_then(|value| value.as_u64())
            .map(|value| value.min(100) as u8),
        updated_at: now_ms(),
        error: None,
    })
}

pub async fn home(auth: &Auth) -> Result<HomeData, String> {
    if !auth.authenticated() {
        return Ok(HomeData::default());
    }
    let display_name = profile(auth).await.ok().flatten();
    let mut warnings = Vec::new();
    let recently_played = match request(
        auth,
        Method::GET,
        "/me/player/recently-played",
        &[("limit", "20".into())],
        None,
    )
    .await
    {
        Ok(value) => value
            .and_then(|value| {
                value
                    .get("items")
                    .and_then(|value| value.as_array())
                    .cloned()
            })
            .map(|items| items.iter().map(track).filter(has_track).collect())
            .unwrap_or_default(),
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };
    let playlists = match request(
        auth,
        Method::GET,
        "/me/playlists",
        &[("limit", "20".into())],
        None,
    )
    .await
    {
        Ok(value) => value
            .and_then(|value| {
                value
                    .get("items")
                    .and_then(|value| value.as_array())
                    .cloned()
            })
            .map(|items| items.iter().map(playlist).collect())
            .unwrap_or_default(),
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };
    let albums = match request(
        auth,
        Method::GET,
        "/me/albums",
        &[("limit", "20".into())],
        None,
    )
    .await
    {
        Ok(value) => value
            .and_then(|value| {
                value
                    .get("items")
                    .and_then(|value| value.as_array())
                    .cloned()
            })
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("album"))
                    .map(album)
                    .collect()
            })
            .unwrap_or_default(),
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };
    let tracks = match request(
        auth,
        Method::GET,
        "/me/tracks",
        &[("limit", "20".into())],
        None,
    )
    .await
    {
        Ok(value) => value
            .and_then(|value| {
                value
                    .get("items")
                    .and_then(|value| value.as_array())
                    .cloned()
            })
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("track"))
                    .map(track)
                    .filter(has_track)
                    .collect()
            })
            .unwrap_or_default(),
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };
    let devices = match request(auth, Method::GET, "/me/player/devices", &[], None).await {
        Ok(value) => value
            .and_then(|value| {
                value
                    .get("devices")
                    .and_then(|value| value.as_array())
                    .cloned()
            })
            .map(|items| items.iter().map(device).collect())
            .unwrap_or_default(),
        Err(error) => {
            warnings.push(error);
            Vec::new()
        }
    };
    Ok(HomeData {
        authenticated: auth.authenticated(),
        display_name,
        recently_played,
        playlists,
        albums,
        tracks,
        devices,
        warnings,
    })
}

pub async fn search(auth: &Auth, query: &str) -> Result<SearchResults, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(SearchResults::default());
    }
    let value = request(
        auth,
        Method::GET,
        "/search",
        &[
            ("q", query.into()),
            ("type", "track,album,playlist".into()),
            ("limit", "10".into()),
        ],
        None,
    )
    .await?;
    let Some(value) = value else {
        return Ok(SearchResults::default());
    };
    let tracks = value
        .pointer("/tracks/items")
        .and_then(|value| value.as_array())
        .map(|items| items.iter().map(track).filter(has_track).collect())
        .unwrap_or_default();
    let albums = value
        .pointer("/albums/items")
        .and_then(|value| value.as_array())
        .map(|items| items.iter().map(album).collect())
        .unwrap_or_default();
    let playlists = value
        .pointer("/playlists/items")
        .and_then(|value| value.as_array())
        .map(|items| items.iter().map(playlist).collect())
        .unwrap_or_default();
    Ok(SearchResults {
        tracks,
        albums,
        playlists,
    })
}

pub async fn queue(auth: &Auth) -> Result<Queue, String> {
    let value = request(auth, Method::GET, "/me/player/queue", &[], None).await?;
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(Queue::default());
    };
    Ok(Queue {
        currently_playing: value
            .get("currently_playing")
            .filter(|value| !value.is_null())
            .map(track)
            .filter(has_track),
        items: value
            .get("queue")
            .and_then(|value| value.as_array())
            .map(|items| items.iter().map(track).filter(has_track).collect())
            .unwrap_or_default(),
    })
}

pub async fn devices(auth: &Auth) -> Result<Vec<Device>, String> {
    let value = request(auth, Method::GET, "/me/player/devices", &[], None).await?;
    Ok(value
        .and_then(|value| {
            value
                .get("devices")
                .and_then(|value| value.as_array())
                .cloned()
        })
        .map(|items| items.iter().map(device).collect())
        .unwrap_or_default())
}

pub async fn play(
    auth: &Auth,
    uri: &str,
    device_id: Option<&str>,
    context_uri: Option<&str>,
) -> Result<(), String> {
    let body = if let Some(context) = context_uri {
        if !uri.starts_with("spotify:track:")
            || !(context.starts_with("spotify:album:") || context.starts_with("spotify:playlist:"))
        {
            return Err("Unsupported Spotify playback context.".into());
        }
        json!({ "context_uri": context, "offset": { "uri": uri } })
    } else if uri.starts_with("spotify:track:") {
        json!({ "uris": [uri] })
    } else if uri.starts_with("spotify:album:")
        || uri.starts_with("spotify:playlist:")
        || uri.starts_with("spotify:artist:")
    {
        json!({ "context_uri": uri })
    } else {
        return Err("Unsupported Spotify item.".into());
    };
    let query = device_id
        .filter(|value| !value.is_empty())
        .map(|value| vec![("device_id", value.to_string())])
        .unwrap_or_default();
    request(auth, Method::PUT, "/me/player/play", &query, Some(body))
        .await
        .map(|_| ())
}

pub async fn transfer(auth: &Auth, device_id: &str) -> Result<(), String> {
    if device_id.is_empty() {
        return Err("Choose a Spotify device.".into());
    }
    request(
        auth,
        Method::PUT,
        "/me/player",
        &[],
        Some(json!({ "device_ids": [device_id], "play": false })),
    )
    .await
    .map(|_| ())
}

pub async fn control(auth: &Auth, command: Command) -> Result<(), String> {
    match command {
        Command::Toggle => {
            let state = playback(auth).await?;
            if state.playing {
                request(auth, Method::PUT, "/me/player/pause", &[], None)
                    .await
                    .map(|_| ())
            } else {
                request(auth, Method::PUT, "/me/player/play", &[], None)
                    .await
                    .map(|_| ())
            }
        }
        Command::Play => request(auth, Method::PUT, "/me/player/play", &[], None)
            .await
            .map(|_| ()),
        Command::Pause => request(auth, Method::PUT, "/me/player/pause", &[], None)
            .await
            .map(|_| ()),
        Command::Next => request(auth, Method::POST, "/me/player/next", &[], None)
            .await
            .map(|_| ()),
        Command::Previous => request(auth, Method::POST, "/me/player/previous", &[], None)
            .await
            .map(|_| ()),
        Command::Seek(position) => request(
            auth,
            Method::PUT,
            "/me/player/seek",
            &[("position_ms", position.to_string())],
            None,
        )
        .await
        .map(|_| ()),
        Command::Shuffle(active) => request(
            auth,
            Method::PUT,
            "/me/player/shuffle",
            &[("state", active.to_string())],
            None,
        )
        .await
        .map(|_| ()),
        Command::Repeat(mode) => request(
            auth,
            Method::PUT,
            "/me/player/repeat",
            &[("state", mode)],
            None,
        )
        .await
        .map(|_| ()),
    }
}

pub async fn volume(auth: &Auth, value: u8) -> Result<(), String> {
    request(
        auth,
        Method::PUT,
        "/me/player/volume",
        &[("volume_percent", value.min(100).to_string())],
        None,
    )
    .await
    .map(|_| ())
}

fn has_track(track: &Track) -> bool {
    !track.title.trim().is_empty()
}

fn track(value: &Value) -> Track {
    let value = value
        .get("track")
        .or_else(|| value.get("item"))
        .filter(|value| !value.is_null())
        .unwrap_or(value);
    let album = value.get("album").filter(|value| !value.is_null());
    let show = value.get("show").filter(|value| !value.is_null());
    let artist = value
        .get("artists")
        .and_then(|value| value.as_array())
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist.get("name").and_then(|value| value.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .or_else(|| {
            show.and_then(|show| show.get("name"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let album_name = album
        .and_then(|album| album.get("name"))
        .and_then(|value| value.as_str())
        .or_else(|| {
            show.and_then(|show| show.get("name"))
                .and_then(|value| value.as_str())
        })
        .unwrap_or_default()
        .to_string();
    Track {
        id: value
            .get("id")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        uri: value
            .get("uri")
            .and_then(|uri| uri.as_str())
            .filter(|_| value.get("is_playable").and_then(Value::as_bool) != Some(false))
            .map(str::to_string),
        title: value
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        artist,
        album: album_name,
        artwork_url: album
            .and_then(|album| album.get("images"))
            .and_then(images)
            .or_else(|| value.get("images").and_then(images))
            .or_else(|| show.and_then(|show| show.get("images")).and_then(images)),
        duration_ms: value.get("duration_ms").and_then(|value| value.as_u64()),
    }
}

fn playlist(value: &Value) -> Playlist {
    Playlist {
        id: value
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        uri: value
            .get("uri")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        name: value
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        description: value
            .get("description")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        owner: value
            .pointer("/owner/display_name")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        artwork_url: value.get("images").and_then(images),
        track_count: value
            .pointer("/items/total")
            .or_else(|| value.pointer("/tracks/total"))
            .and_then(|value| value.as_u64())
            .unwrap_or_default(),
    }
}

fn album(value: &Value) -> Album {
    Album {
        id: value
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        uri: value
            .get("uri")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        name: value
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        artist: value
            .get("artists")
            .and_then(|value| value.as_array())
            .map(|artists| {
                artists
                    .iter()
                    .filter_map(|artist| artist.get("name").and_then(|value| value.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default(),
        artwork_url: value.get("images").and_then(images),
        year: value
            .get("release_date")
            .and_then(|value| value.as_str())
            .map(|value| value.chars().take(4).collect()),
    }
}

fn device(value: &Value) -> Device {
    Device {
        id: value
            .get("id")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        name: value
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("Spotify device")
            .to_string(),
        kind: value
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        active: value
            .get("is_active")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        restricted: value
            .get("is_restricted")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        volume: value
            .get("volume_percent")
            .and_then(|value| value.as_u64())
            .map(|value| value.min(100) as u8),
    }
}

fn images(value: &Value) -> Option<String> {
    value
        .as_array()?
        .iter()
        .find_map(|image| image.get("url").and_then(|value| value.as_str()))
        .map(str::to_string)
}

pub async fn browse(
    auth: &Auth,
    kind: &str,
    id: &str,
    offset: u32,
) -> Result<super::models::BrowsePage, String> {
    let path = match kind {
        "tracks" => "/me/tracks".to_string(),
        "playlists" => "/me/playlists".to_string(),
        "albums" => "/me/albums".to_string(),
        "playlist" | "album" => {
            if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                return Err("Choose a valid Spotify collection.".into());
            }
            if kind == "playlist" {
                format!("/playlists/{id}/items")
            } else {
                format!("/albums/{id}/tracks")
            }
        }
        _ => return Err("Unsupported Spotify collection.".into()),
    };
    let value = request(
        auth,
        Method::GET,
        &path,
        &[("limit", "50".into()), ("offset", offset.to_string())],
        None,
    )
    .await?
    .ok_or_else(|| "Spotify returned no collection data.".to_string())?;
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| "Spotify returned an invalid collection.".to_string())?;
    let mut page = super::models::BrowsePage {
        total: value
            .get("total")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        next_offset: value
            .get("next")
            .and_then(Value::as_str)
            .map(|_| u64::from(offset) + items.len() as u64),
        ..Default::default()
    };
    match kind {
        "playlists" => {
            page.playlists = items
                .iter()
                .filter(|item| !item.is_null())
                .map(playlist)
                .collect()
        }
        "albums" => {
            page.albums = items
                .iter()
                .filter_map(|item| item.get("album"))
                .map(album)
                .collect()
        }
        _ => page.tracks = items.iter().map(track).filter(has_track).collect(),
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_items_parse_current_payload_and_keep_unavailable_tracks_visible() {
        let current = json!({ "item": { "id": "abc", "uri": "spotify:track:abc", "name": "Song", "artists": [{ "name": "Artist" }], "duration_ms": 180000 } });
        let parsed = track(&current);
        assert_eq!(parsed.title, "Song");
        assert_eq!(parsed.artist, "Artist");
        assert_eq!(parsed.uri.as_deref(), Some("spotify:track:abc"));
        assert!(!has_track(&track(&json!({ "item": null }))));
        let unavailable = track(
            &json!({ "item": { "name": "Unavailable song", "uri": "spotify:track:abc", "is_playable": false } }),
        );
        assert!(has_track(&unavailable));
        assert!(unavailable.uri.is_none());
    }
}
