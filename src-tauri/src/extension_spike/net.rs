use std::{future::Future, net::IpAddr, time::Duration};

use serde::Serialize;
use serde_json::Value;

const MAX_BODY: usize = 2 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

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

    pub(crate) fn cancelled() -> Self {
        Self {
            code: "cancelled",
            message: "The package runtime stopped before the request finished.",
        }
    }

    fn denied() -> Self {
        Self {
            code: "denied",
            message: "That host did not resolve to a public address.",
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
    within(REQUEST_TIMEOUT, request(origin, path, query)).await
}

async fn within<T, F>(duration: Duration, future: F) -> Result<T, Error>
where
    F: Future<Output = Result<T, Error>>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| Error::timeout())?
}

async fn request(origin: &str, path: &str, query: &[(String, String)]) -> Result<Value, Error> {
    let url = build_url(origin, path, query)?;
    let host = url.host_str().ok_or_else(Error::invalid)?.to_string();
    let addresses = public_addresses(&host).await?;
    let response = client(&host, &addresses)?
        .get(url)
        .send()
        .await
        .map_err(transport)?;
    require_ok(response.status())?;
    let body = read_body(response).await?;
    serde_json::from_slice(&body).map_err(|_| Error::malformed())
}

async fn public_addresses(host: &str) -> Result<Vec<std::net::SocketAddr>, Error> {
    let resolved: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host, 443))
        .await
        .map_err(|_| Error::unavailable())?
        .collect();
    if resolved.is_empty() || !resolved.iter().all(|address| is_public(address.ip())) {
        return Err(Error::denied());
    }
    Ok(resolved)
}

fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            !(address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_broadcast()
                || address.is_documentation()
                || address.is_unspecified()
                || address.is_multicast()
                || octets[0] == 0
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 198 && matches!(octets[1], 18 | 19))
                || octets[0] >= 240)
        }
        IpAddr::V6(address) => {
            let mapped = address.to_ipv4_mapped();
            if let Some(address) = mapped {
                return is_public(IpAddr::V4(address));
            }
            !(address.is_loopback()
                || address.is_unspecified()
                || address.is_multicast()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || (address.segments()[0] == 0x2001 && address.segments()[1] == 0x0db8)
                || (address.segments()[0] & 0xffc0) == 0xfe80
                || (address.segments()[0] & 0xff00) == 0xff00)
        }
    }
}

fn client(host: &str, addresses: &[std::net::SocketAddr]) -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(Duration::from_secs(3))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, addresses)
        .build()
        .map_err(|_| Error::unavailable())
}

fn build_url(origin: &str, path: &str, query: &[(String, String)]) -> Result<reqwest::Url, Error> {
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
    use super::{
        append_chunk, build_url, declared_too_large, is_public, require_ok, within, Error,
        MAX_BODY, REQUEST_TIMEOUT,
    };
    use std::{net::IpAddr, time::Duration};

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
    fn localhost_resolution_is_rejected_as_a_private_target() {
        let error =
            tauri::async_runtime::block_on(super::public_addresses("localhost")).unwrap_err();
        assert_eq!(error.code, "denied");
    }

    #[test]
    fn the_whole_lookup_and_request_share_one_bounded_deadline() {
        assert!(REQUEST_TIMEOUT <= Duration::from_secs(10));
        let error = tauri::async_runtime::block_on(within(Duration::from_millis(20), async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok::<(), Error>(())
        }))
        .unwrap_err();
        assert_eq!(error.code, "timeout");
    }

    #[test]
    fn rejects_private_and_reserved_addresses() {
        for address in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.4",
            "169.254.1.1",
            "0.0.0.0",
            "100.64.0.1",
            "198.18.0.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:192.168.0.1",
        ] {
            assert!(
                !is_public(address.parse::<IpAddr>().unwrap()),
                "{address} must not be a public target"
            );
        }
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(
                is_public(address.parse::<IpAddr>().unwrap()),
                "{address} must remain a public target"
            );
        }
    }

    #[test]
    fn reports_typed_errors() {
        for error in [
            Error::invalid(),
            Error::cancelled(),
            Error::denied(),
            Error::timeout(),
            Error::unavailable(),
            Error::too_large(),
            Error::malformed(),
        ] {
            assert!(!error.code.is_empty());
            assert!(!error.message.is_empty());
        }
    }
}
