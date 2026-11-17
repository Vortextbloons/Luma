use std::{sync::OnceLock, time::Duration};

use serde::Serialize;
use serde_json::Value;
use tokio::sync::Mutex;

const MAX_BODY: usize = 64 * 1024;

static IN_FLIGHT: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Error {
    pub code: &'static str,
    pub message: &'static str,
}

impl Error {
    pub(crate) fn invalid() -> Self {
        Self {
            code: "invalid",
            message: "The host rejected that network request.",
        }
    }

    fn busy() -> Self {
        Self {
            code: "busy",
            message: "A network request is already running.",
        }
    }

    fn timeout() -> Self {
        Self {
            code: "timeout",
            message: "The network request took too long.",
        }
    }

    fn unavailable() -> Self {
        Self {
            code: "unavailable",
            message: "The host could not reach that service.",
        }
    }

    fn too_large() -> Self {
        Self {
            code: "too_large",
            message: "The response was too large.",
        }
    }

    fn malformed() -> Self {
        Self {
            code: "malformed",
            message: "The response could not be read.",
        }
    }
}

pub(crate) async fn get(
    origin: &str,
    path: &str,
    query: &[(String, String)],
) -> Result<Value, Error> {
    let _guard = IN_FLIGHT.try_lock().map_err(|_| Error::busy())?;
    let url = build_url(origin, path, query)?;
    let response = client()?
        .get(url)
        .send()
        .await
        .map_err(transport)?;
    require_ok(response.status())?;
    let body = read_body(response).await?;
    serde_json::from_slice(&body).map_err(|_| Error::malformed())
}

fn client() -> Result<&'static reqwest::Client, Error> {
    static CLIENT: OnceLock<Result<reqwest::Client, Error>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .connect_timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| Error::unavailable())
        })
        .as_ref()
        .map_err(|error| *error)
}

fn build_url(
    origin: &str,
    path: &str,
    query: &[(String, String)],
) -> Result<reqwest::Url, Error> {
    let mut url = reqwest::Url::parse(origin).map_err(|_| Error::invalid())?;
    url.set_path(path);
    url.query_pairs_mut().extend_pairs(query);
    Ok(url)
}

async fn read_body(mut response: reqwest::Response) -> Result<Vec<u8>, Error> {
    if declared_too_large(response.content_length()) {
        return Err(Error::too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        append_chunk(&mut body, &chunk)?;
    }
    Ok(body)
}

fn declared_too_large(length: Option<u64>) -> bool {
    length.is_some_and(|length| length > MAX_BODY as u64)
}

fn append_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), Error> {
    if body.len() + chunk.len() > MAX_BODY {
        return Err(Error::too_large());
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn require_ok(status: reqwest::StatusCode) -> Result<(), Error> {
    if status == reqwest::StatusCode::OK {
        Ok(())
    } else {
        Err(Error::unavailable())
    }
}

fn transport(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::timeout()
    } else {
        Error::unavailable()
    }
}

#[cfg(test)]
mod tests {
    use super::{append_chunk, build_url, declared_too_large, require_ok, Error, MAX_BODY};

    #[test]
    fn builds_an_allowlisted_url_from_structured_parts() {
        let url = build_url(
            "https://api.open-meteo.com",
            "/v1/forecast",
            &[
                ("latitude".into(), "47.6062".into()),
                ("name".into(), "Seattle & Puget Sound".into()),
            ],
        )
        .unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("api.open-meteo.com"));
        assert_eq!(url.port(), None);
        assert_eq!(url.path(), "/v1/forecast");
        assert_eq!(url.username(), "");
        assert!(url.password().is_none());
        let pairs: Vec<_> = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("latitude".to_string(), "47.6062".to_string()),
                ("name".to_string(), "Seattle & Puget Sound".to_string())
            ]
        );
    }

    #[test]
    fn rejects_non_success_statuses_without_following_redirects() {
        assert!(require_ok(reqwest::StatusCode::OK).is_ok());
        for status in [301, 302, 304, 307, 308, 400, 401, 403, 404, 429, 500, 503] {
            let error = require_ok(reqwest::StatusCode::from_u16(status).unwrap())
                .err()
                .expect("non-success status fails");
            assert_eq!(error.code, "unavailable");
        }
    }

    #[test]
    fn caps_the_response_before_and_during_reading() {
        assert!(!declared_too_large(None));
        assert!(!declared_too_large(Some(MAX_BODY as u64)));
        assert!(declared_too_large(Some(MAX_BODY as u64 + 1)));
        let mut body = Vec::new();
        append_chunk(&mut body, &vec![b' '; MAX_BODY]).unwrap();
        assert_eq!(body.len(), MAX_BODY);
        assert_eq!(
            append_chunk(&mut body, b" ").err().map(|error| error.code),
            Some("too_large")
        );
        assert_eq!(body.len(), MAX_BODY);
    }

    #[test]
    fn reports_typed_errors() {
        for error in [Error::invalid(), Error::busy(), Error::timeout(), Error::unavailable(), Error::too_large(), Error::malformed()] {
            assert!(!error.code.is_empty());
            assert!(!error.message.is_empty());
        }
    }
}
