pub use super::models::{
    ActivityItem, ActivityType, PlaybackReport, ProgressSource, ProviderConfig, ResumeKind,
};
use std::sync::OnceLock;

pub mod jellyfin;
pub mod local;
pub mod plex;
pub mod spotify;
pub mod trakt;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .user_agent("Luma/0.1 activity manager")
            .build()
            .unwrap_or_default()
    })
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn from_unix(seconds: i64) -> String {
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(now)
}

pub fn normalized_time(value: Option<String>, fallback: &str) -> String {
    value
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| {
            value
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        })
        .unwrap_or_else(|| fallback.to_string())
}

pub fn validated_base(value: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(value.trim())
        .map_err(|_| "Enter a complete http:// or https:// server URL.".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Use an http:// or https:// server URL without credentials, a query, or a fragment."
                .into(),
        );
    }
    Ok(url.to_string().trim_end_matches('/').to_string())
}

pub fn safe_artwork(value: Option<&str>, fallback: Option<String>) -> Option<String> {
    value
        .and_then(|value| reqwest::Url::parse(value).ok())
        .filter(|url| {
            url.scheme() == "https"
                && url.host_str().is_some_and(|host| {
                    [
                        "ytimg.com",
                        "ggpht.com",
                        "googleusercontent.com",
                        "yt3.ggpht.com",
                    ]
                    .iter()
                    .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
                })
        })
        .map(|url| url.to_string())
        .or(fallback)
}

pub async fn refresh(provider: &str, config: &ProviderConfig) -> Result<Vec<ActivityItem>, String> {
    match provider {
        "plex" => plex::refresh(config).await,
        "jellyfin" => jellyfin::refresh(config).await,
        "trakt" => trakt::refresh(config).await,
        _ => Err(format!(
            "{provider} does not provide a refreshable activity source."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_urls_must_be_plain_http_or_https() {
        assert_eq!(
            validated_base("http://192.168.1.20:32400/").unwrap(),
            "http://192.168.1.20:32400"
        );
        for value in [
            "ftp://server",
            "http://user:pass@server",
            "http://server/path?token=x",
            "server.local",
        ] {
            assert!(validated_base(value).is_err(), "{value}");
        }
    }

    #[test]
    fn artwork_only_accepts_provider_https_hosts() {
        assert_eq!(
            safe_artwork(
                Some("https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg"),
                None
            )
            .as_deref(),
            Some("https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg")
        );
        assert_eq!(safe_artwork(Some("http://example.com/a.jpg"), None), None);
        assert_eq!(
            safe_artwork(
                Some("https://evil.example/a.jpg"),
                Some("https://i.ytimg.com/".into())
            ),
            Some("https://i.ytimg.com/".into())
        );
    }
}
