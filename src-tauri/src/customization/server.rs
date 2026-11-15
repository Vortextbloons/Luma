use super::{
    document::{self, Config, Document},
    publish, Commit, Error, Store,
};
use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tauri::Manager;

const ENV: &str = "LUMA_CUSTOMIZATION_TOKEN";
const ADDRESS: &str = "127.0.0.1:9878";
const MAX_BODY: usize = 32 * 1024;

#[derive(Clone)]
struct Auth {
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Restore {
    expected_revision: u64,
}

pub async fn start(app: tauri::AppHandle) -> Result<(), String> {
    let Some(token) = configured()? else {
        return Ok(());
    };
    let listener = tokio::net::TcpListener::bind(ADDRESS)
        .await
        .map_err(|error| format!("Cannot listen on {ADDRESS}: {error}"))?;
    axum::serve(listener, router(app, token))
        .await
        .map_err(|error| error.to_string())
}

fn configured() -> Result<Option<String>, String> {
    match std::env::var(ENV) {
        Ok(token) => token_from(token).map(Some),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(format!("{ENV} must be valid UTF-8.")),
    }
}

fn token_from(token: String) -> Result<String, String> {
    if token.len() < 32 || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(format!(
            "{ENV} must be at least 32 printable non-whitespace ASCII characters."
        ));
    }
    Ok(token)
}

fn router(app: tauri::AppHandle, token: String) -> Router {
    protect::<tauri::AppHandle>(
        Router::new()
            .route("/v1/config", get(config).put(commit))
            .route("/v1/validate", axum::routing::post(validate))
            .route("/v1/restore", axum::routing::post(restore)),
        token,
    )
    .with_state(app)
}

fn protect<S>(router: Router<S>, token: String) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn_with_state(Auth { token }, authorized))
}

async fn authorized(State(auth): State<Auth>, request: Request, next: Next) -> Response {
    let mut response = match guard(&auth, request.headers()) {
        Ok(()) => next.run(request).await,
        Err(error) => failure(error),
    };
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn guard(auth: &Auth, headers: &HeaderMap) -> Result<(), Error> {
    if headers.contains_key(header::ORIGIN) {
        return Err(fail(
            "forbidden_origin",
            "This API does not accept browser requests.",
        ));
    }
    if headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(ADDRESS)
    {
        return Err(fail(
            "forbidden_host",
            format!("Requests must be sent to {ADDRESS}."),
        ));
    }
    let candidate = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or("");
    if !same(candidate, &auth.token) {
        return Err(fail("unauthorized", "Missing or invalid bearer token."));
    }
    Ok(())
}

fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut difference = a.len() ^ b.len();
    for index in 0..a.len().max(b.len()) {
        difference |= usize::from(a.get(index).unwrap_or(&0) ^ b.get(index).unwrap_or(&0));
    }
    difference == 0
}

async fn config(State(app): State<tauri::AppHandle>) -> Response {
    match blocking(&app, |store| store.read()).await {
        Ok(document) => Json(document).into_response(),
        Err(error) => failure(error),
    }
}

async fn validate(body: Result<Json<Config>, JsonRejection>) -> Response {
    let Json(config) = match body {
        Ok(body) => body,
        Err(error) => return rejection(error),
    };
    match document::validate(&config) {
        Ok(()) => Json(serde_json::json!({ "valid": true })).into_response(),
        Err(message) => failure(fail("validation", message)),
    }
}

async fn commit(
    State(app): State<tauri::AppHandle>,
    body: Result<Json<Commit>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(error) => return rejection(error),
    };
    reply(write(&app, move |store| store.commit(request)).await)
}

async fn restore(
    State(app): State<tauri::AppHandle>,
    body: Result<Json<Restore>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(body) => body,
        Err(error) => return rejection(error),
    };
    reply(write(&app, move |store| store.restore(request.expected_revision)).await)
}

async fn blocking<T: Send + 'static>(
    app: &tauri::AppHandle,
    task: impl FnOnce(&Store) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || task(&app.state::<Store>()))
        .await
        .map_err(|error| fail("storage", error))?
}

async fn write(
    app: &tauri::AppHandle,
    task: impl FnOnce(&Store) -> Result<Document, Error> + Send + 'static,
) -> Result<Document, Error> {
    let document = blocking(app, task).await?;
    publish(app, &document);
    Ok(document)
}

fn reply(result: Result<Document, Error>) -> Response {
    match result {
        Ok(document) => Json(document).into_response(),
        Err(error) => failure(error),
    }
}

fn fail(code: &'static str, message: impl ToString) -> Error {
    Error {
        code,
        message: message.to_string(),
    }
}

fn failure(error: Error) -> Response {
    (status(error.code), Json(error)).into_response()
}

fn rejection(rejection: JsonRejection) -> Response {
    let status = rejection.status();
    let code = match status {
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::UNPROCESSABLE_ENTITY => "validation",
        _ => "bad_request",
    };
    (status, Json(fail(code, rejection.body_text()))).into_response()
}

fn status(code: &str) -> StatusCode {
    match code {
        "unauthorized" => StatusCode::UNAUTHORIZED,
        "forbidden_origin" | "forbidden_host" => StatusCode::FORBIDDEN,
        "conflict" | "not_initialized" | "no_previous" | "revision_limit" => StatusCode::CONFLICT,
        "validation" => StatusCode::UNPROCESSABLE_ENTITY,
        "storage" | "invalid_document" | "unsupported_version" => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::BAD_REQUEST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn probe(calls: Arc<AtomicUsize>) -> Router {
        protect(
            Router::new().route(
                "/v1/restore",
                axum::routing::post(move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        "restored"
                    }
                }),
            ),
            TOKEN.into(),
        )
    }

    fn serve(router: Router) -> u16 {
        tauri::async_runtime::block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tauri::async_runtime::spawn(async move {
                let _ = axum::serve(listener, router).await;
            });
            port
        })
    }

    fn send(port: u16, host: &str, headers: &[(&str, &str)]) -> (u16, String) {
        let mut request = format!(
            "POST /v1/restore HTTP/1.1\r\nhost: {host}\r\ncontent-length: 2\r\nconnection: close\r\n"
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n{}");
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response
            .split("\r\n")
            .next()
            .and_then(|line| line.split(' ').nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap();
        (status, response)
    }

    #[test]
    fn unauthorized_requests_never_reach_handlers() {
        let calls = Arc::new(AtomicUsize::new(0));
        let port = serve(probe(calls.clone()));
        let wrong = format!("Bearer {TOKEN}x");
        assert_eq!(send(port, ADDRESS, &[]).0, 401);
        assert_eq!(
            send(port, ADDRESS, &[("authorization", "Bearer wrong")]).0,
            401
        );
        assert_eq!(send(port, ADDRESS, &[("authorization", &wrong)]).0, 401);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn authorized_requests_reach_handlers_without_caching() {
        let calls = Arc::new(AtomicUsize::new(0));
        let port = serve(probe(calls.clone()));
        let auth = format!("Bearer {TOKEN}");
        let (status, response) = send(port, ADDRESS, &[("authorization", &auth)]);
        assert_eq!(status, 200);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let response = response.to_lowercase();
        assert!(response.contains("cache-control: no-store"));
        assert!(response.contains("x-content-type-options: nosniff"));
    }

    #[test]
    fn browser_origins_and_foreign_hosts_are_rejected() {
        let calls = Arc::new(AtomicUsize::new(0));
        let port = serve(probe(calls.clone()));
        let auth = format!("Bearer {TOKEN}");
        assert_eq!(
            send(
                port,
                ADDRESS,
                &[
                    ("authorization", &auth),
                    ("origin", "http://127.0.0.1:9878")
                ]
            )
            .0,
            403
        );
        assert_eq!(
            send(port, "localhost:9878", &[("authorization", &auth)]).0,
            403
        );
        assert_eq!(
            send(port, "127.0.0.1:9879", &[("authorization", &auth)]).0,
            403
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn tokens_must_be_long_printable_ascii() {
        assert!(token_from("a".repeat(32)).is_ok());
        assert!(token_from("a".repeat(31)).is_err());
        assert!(token_from(format!("{} ", "a".repeat(31))).is_err());
        assert!(token_from(format!("{}é", "a".repeat(31))).is_err());
    }

    #[test]
    fn error_codes_map_to_documented_statuses() {
        assert_eq!(status("conflict"), StatusCode::CONFLICT);
        assert_eq!(status("not_initialized"), StatusCode::CONFLICT);
        assert_eq!(status("validation"), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(status("storage"), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(status("bad_request"), StatusCode::BAD_REQUEST);
    }
}
