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

pub struct ProviderRefresh {
    pub items: Vec<ActivityItem>,
    pub resume_ids: Option<Vec<String>>,
}

pub async fn refresh(provider: &str, config: &ProviderConfig) -> Result<ProviderRefresh, String> {
    match provider {
        "plex" => plex::refresh(config).await,
        "jellyfin" => jellyfin::refresh(config).await,
        "trakt" => trakt::refresh(config).await.map(|items| ProviderRefresh {
            items,
            resume_ids: None,
        }),
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
    #[tokio::test]
    async fn independent_provider_lists_load_concurrently() {
        use axum::{extract::State, routing::get, Json, Router};
        use serde_json::{json, Value};
        use std::sync::Arc;
        use tokio::sync::Barrier;

        async fn list(State(barrier): State<Arc<Barrier>>) -> Json<Value> {
            barrier.wait().await;
            Json(
                json!({"MediaContainer": {"Metadata": [], "size": 0}, "Items": [], "TotalRecordCount": 0}),
            )
        }
        for provider in ["plex", "jellyfin"] {
            let barrier = Arc::new(Barrier::new(2));
            let router = Router::new()
                .route(
                    "/identity",
                    get(|| async {
                        Json(json!({"MediaContainer": {"machineIdentifier": "machine"}}))
                    }),
                )
                .route("/library/onDeck", get(list))
                .route("/library/recentlyViewed", get(list))
                .route("/Users/user/Items/Resume", get(list))
                .route("/Users/user/Items", get(list))
                .with_state(barrier);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
            let config = ProviderConfig {
                server_url: Some(format!("http://{address}")),
                token: Some("test-token".into()),
                user_id: Some("user".into()),
                client_id: None,
            };
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                refresh(provider, &config),
            )
            .await;
            server.abort();
            let data = result
                .expect("both list requests must start before either finishes")
                .unwrap();
            assert!(data.items.is_empty());
            assert_eq!(data.resume_ids, Some(vec![]));
        }
    }
}
