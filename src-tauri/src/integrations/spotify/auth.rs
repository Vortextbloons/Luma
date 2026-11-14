use super::models::now_ms;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::Duration,
};
use tauri::Manager;
use tokio::sync::{oneshot, Mutex as AsyncMutex};

const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
const SCOPES: &str = "user-read-playback-state user-modify-playback-state playlist-read-private playlist-read-collaborative user-library-read user-read-recently-played user-read-private";
const CREDENTIAL_TARGET: &str = "Luma/Spotify/OAuth";

#[derive(Clone, Default, Serialize, Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    client_id: String,
}

pub struct Auth {
    client_id: Mutex<String>,
    tokens: Mutex<Option<Tokens>>,
    client: Client,
    connecting: AsyncMutex<()>,
    config_dir: std::path::PathBuf,
    client_id_configured: Mutex<bool>,
}

impl Auth {
    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&config_dir).map_err(|error| error.to_string())?;
        let config = std::fs::read(config_dir.join("spotify.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Config>(&bytes).ok())
            .unwrap_or_default();
        let tokens = credential_read()
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice::<Tokens>(&bytes).ok());
        let configured = !config.client_id.is_empty();
        Ok(Self {
            client_id: Mutex::new(config.client_id),
            tokens: Mutex::new(tokens),
            client: Client::builder()
                .user_agent("Luma/0.1 (Spotify integration)")
                .timeout(Duration::from_secs(20))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .map_err(|error| error.to_string())?,
            connecting: AsyncMutex::new(()),
            config_dir,
            client_id_configured: Mutex::new(configured),
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn client_id(&self) -> String {
        self.client_id
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    pub fn client_id_configured(&self) -> bool {
        self.client_id_configured
            .lock()
            .map(|value| *value)
            .unwrap_or(false)
    }

    pub fn save_client_id(&self, value: &str) -> Result<(), String> {
        let value = value.trim();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err("Enter the Client ID from your Spotify app dashboard.".into());
        }
        let config = Config {
            client_id: value.into(),
        };
        std::fs::write(
            self.config_dir.join("spotify.json"),
            serde_json::to_vec(&config).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        *self.client_id.lock().map_err(|error| error.to_string())? = value.into();
        *self
            .client_id_configured
            .lock()
            .map_err(|error| error.to_string())? = true;
        Ok(())
    }

    pub fn authenticated(&self) -> bool {
        self.tokens
            .lock()
            .map(|tokens| {
                tokens
                    .as_ref()
                    .is_some_and(|tokens| !tokens.refresh_token.is_empty())
            })
            .unwrap_or(false)
    }

    pub async fn access_token(&self) -> Result<String, String> {
        let current = {
            let tokens = self.tokens.lock().map_err(|error| error.to_string())?;
            tokens.as_ref().map(|tokens| {
                (
                    tokens.access_token.clone(),
                    tokens.expires_at,
                )
            })
        };
        if let Some((token, expires_at)) = current {
            if !token.is_empty() && expires_at > now_ms() / 1000 + 30 {
                return Ok(token);
            }
        }
        self.refresh().await
    }

    pub async fn refresh(&self) -> Result<String, String> {
        let refresh_token = {
            let tokens = self.tokens.lock().map_err(|error| error.to_string())?;
            tokens
                .as_ref()
                .map(|tokens| tokens.refresh_token.clone())
                .filter(|value| !value.is_empty())
                .ok_or("Connect a Spotify account to use this feature.")?
        };
        let body = form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", &self.client_id()),
        ]);
        let response = self
            .client
            .post(TOKEN_URL)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|error| format!("Spotify could not be reached: {error}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| error.to_string())?;
        if !status.is_success() {
            if status == reqwest::StatusCode::BAD_REQUEST
                || status == reqwest::StatusCode::UNAUTHORIZED
            {
                self.clear_tokens();
            }
            return Err(token_error(status, &text));
        }
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        let access_token = value
            .get("access_token")
            .and_then(|value| value.as_str())
            .ok_or("Spotify did not return an access token.")?
            .to_string();
        let expires_in = value
            .get("expires_in")
            .and_then(|value| value.as_u64())
            .unwrap_or(3600);
        let stored = {
            let mut tokens = self.tokens.lock().map_err(|error| error.to_string())?;
            let tokens = tokens.get_or_insert_with(Tokens::default);
            tokens.access_token = access_token.clone();
            if let Some(refresh_token) = value.get("refresh_token").and_then(|value| value.as_str()) {
                tokens.refresh_token = refresh_token.to_string();
            }
            tokens.expires_at = now_ms() / 1000 + expires_in;
            tokens.clone()
        };
        let _ = credential_write(&serde_json::to_vec(&stored).map_err(|error| error.to_string())?);
        Ok(access_token)
    }

    pub fn disconnect(&self) -> Result<(), String> {
        self.clear_tokens();
        Ok(())
    }

    fn clear_tokens(&self) {
        if let Ok(mut tokens) = self.tokens.lock() {
            *tokens = None;
        }
        let _ = credential_delete();
    }

    pub async fn connect(&self) -> Result<(), String> {
        let _guard = self.connecting.lock().await;
        let client_id = self.client_id();
        if client_id.is_empty() {
            return Err("Add your Spotify Client ID in Settings first.".into());
        }
        let show_dialog = if self.authenticated() { "true" } else { "false" };
        let verifier = random_token(64)?;
        let challenge = base64url(&sha256(verifier.as_bytes())?);
        let state = random_token(24)?;
        let (sender, receiver) = oneshot::channel();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8888")
            .await
            .map_err(|error| {
                format!(
                    "Cannot listen on 127.0.0.1:8888 for the Spotify callback: {error}. Close any app using that port and try again."
                )
            })?;
        let callback_state = CallbackState {
            state: state.clone(),
            sender: std::sync::Arc::new(Mutex::new(Some(sender))),
        };
        let router = axum::Router::new()
            .route("/callback", axum::routing::get(callback))
            .with_state(callback_state);
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        let mut url = Url::parse(AUTHORIZE_URL).map_err(|error| error.to_string())?;
        url.query_pairs_mut()
            .append_pair("client_id", &client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("scope", SCOPES)
            .append_pair("state", &state)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", &challenge)
            .append_pair("show_dialog", show_dialog);
        if let Err(error) = open_browser(url.as_str()) {
            server.abort();
            return Err(error);
        }
        let code = match tokio::time::timeout(Duration::from_secs(180), receiver).await {
            Ok(Ok(Ok(code))) => code,
            Ok(Ok(Err(error))) => {
                server.abort();
                return Err(error);
            }
            Ok(Err(_)) => {
                server.abort();
                return Err("Spotify authorization was cancelled.".into());
            }
            Err(_) => {
                server.abort();
                return Err("Spotify authorization timed out. Try connecting again.".into());
            }
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        server.abort();
        let body = form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
        ]);
        let response = self
            .client
            .post(TOKEN_URL)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|error| format!("Spotify could not be reached: {error}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(token_error(status, &text));
        }
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        let access_token = value
            .get("access_token")
            .and_then(|value| value.as_str())
            .ok_or("Spotify did not return an access token.")?
            .to_string();
        let refresh_token = value
            .get("refresh_token")
            .and_then(|value| value.as_str())
            .ok_or("Spotify did not return a refresh token.")?
            .to_string();
        let expires_in = value
            .get("expires_in")
            .and_then(|value| value.as_u64())
            .unwrap_or(3600);
        let tokens = Tokens {
            access_token,
            refresh_token,
            expires_at: now_ms() / 1000 + expires_in,
        };
        credential_write(&serde_json::to_vec(&tokens).map_err(|error| error.to_string())?)?;
        {
            let mut stored = self.tokens.lock().map_err(|error| error.to_string())?;
            *stored = Some(tokens);
        }
        Ok(())
    }
}

#[derive(Clone)]
struct CallbackState {
    state: String,
    sender: std::sync::Arc<Mutex<Option<oneshot::Sender<Result<String, String>>>>>,
}

async fn callback(
    axum::extract::State(state): axum::extract::State<CallbackState>,
    axum::extract::Query(query): axum::extract::Query<HashMap<String, String>>,
) -> axum::response::Html<&'static str> {
    let result = if query.get("state") != Some(&state.state) {
        Err("The Spotify authorization response could not be verified.".into())
    } else if let Some(error) = query.get("error") {
        Err(format!("Spotify authorization was denied: {error}"))
    } else {
        query
            .get("code")
            .cloned()
            .ok_or_else(|| "Spotify did not return an authorization code.".to_string())
    };
    let success = result.is_ok();
    if let Ok(mut sender) = state.sender.lock() {
        if let Some(sender) = sender.take() {
            let _ = sender.send(result);
        }
    }
    if success {
        axum::response::Html(
            "<!doctype html><html><head><title>Luma</title></head><body style=\"font-family:Segoe UI,sans-serif;background:#0b0d12;color:#f8f8fa;display:grid;place-items:center;height:100vh;margin:0\"><main style=\"text-align:center\"><h1>Spotify authorization received</h1><p style=\"color:#a9aab5\">Return to Luma to finish connecting. You can close this tab.</p></main></body></html>",
        )
    } else {
        axum::response::Html(
            "<!doctype html><html><head><title>Luma</title></head><body style=\"font-family:Segoe UI,sans-serif;background:#0b0d12;color:#f8f8fa;display:grid;place-items:center;height:100vh;margin:0\"><main style=\"text-align:center\"><h1>Spotify connection failed</h1><p style=\"color:#a9aab5\">Return to Luma and try again.</p></main></body></html>",
        )
    }
}

fn token_error(status: reqwest::StatusCode, text: &str) -> String {
    let detail = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("error_description")
                .and_then(|value| value.as_str())
                .or_else(|| value.get("error").and_then(|value| value.as_str()))
                .map(str::to_string)
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Spotify rejected the request.".into());
    format!("Spotify authorization failed ({status}): {detail}")
}

fn form(values: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in values {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

fn random_token(bytes: usize) -> Result<String, String> {
    let mut random = vec![0u8; bytes];
    getrandom::fill(&mut random).map_err(|error| error.to_string())?;
    Ok(base64url(&random))
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0] as u32;
        let second = chunk.get(1).copied().unwrap_or(0) as u32;
        let third = chunk.get(2).copied().unwrap_or(0) as u32;
        let value = (first << 16) | (second << 8) | third;
        output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((value >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(value & 63) as usize] as char);
        }
    }
    output
}

#[cfg(windows)]
fn sha256(input: &[u8]) -> Result<[u8; 32], String> {
    use windows::{
        core::PCWSTR,
        Win32::Security::Cryptography::{
            BCryptCloseAlgorithmProvider, BCryptHash, BCryptOpenAlgorithmProvider,
            BCRYPT_ALG_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS, BCRYPT_SHA256_ALGORITHM,
        },
    };
    let mut algorithm = BCRYPT_ALG_HANDLE::default();
    let opened = unsafe {
        BCryptOpenAlgorithmProvider(
            &mut algorithm,
            BCRYPT_SHA256_ALGORITHM,
            PCWSTR::null(),
            BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
        )
    };
    if opened.0 < 0 {
        return Err("Windows could not initialize SHA-256 for Spotify sign-in.".into());
    }
    let mut output = [0u8; 32];
    let hashed = unsafe { BCryptHash(algorithm, None, input, &mut output) };
    unsafe { let _ = BCryptCloseAlgorithmProvider(algorithm, 0); };
    if hashed.0 < 0 {
        return Err("Windows could not complete the Spotify sign-in challenge.".into());
    }
    Ok(output)
}

#[cfg(not(windows))]
fn sha256(_: &[u8]) -> Result<[u8; 32], String> {
    Err("Spotify sign-in requires Windows.".into())
}

#[cfg(windows)]
fn open_browser(url: &str) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let operation = wide("open");
    let file = wide(url);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Windows could not open a browser for Spotify sign-in.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_browser(_: &str) -> Result<(), String> {
    Err("Spotify sign-in requires Windows.".into())
}

#[cfg(windows)]
fn credential_write(value: &[u8]) -> Result<(), String> {
    use windows::{
        core::PWSTR,
        Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        },
    };
    let mut target = wide(CREDENTIAL_TARGET);
    let mut secret = value.to_vec();
    let credential = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        Comment: PWSTR::null(),
        LastWritten: Default::default(),
        CredentialBlobSize: secret.len() as u32,
        CredentialBlob: secret.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: PWSTR::null(),
        UserName: PWSTR::null(),
    };
    unsafe { CredWriteW(&credential, 0) }
        .map_err(|error| format!("Windows Credential Manager rejected the Spotify token: {error}"))
}

#[cfg(windows)]
fn credential_read() -> Result<Option<Vec<u8>>, String> {
    use windows::{
        core::PCWSTR,
        Win32::Security::Credentials::{CredFree, CredReadW, CRED_TYPE_GENERIC},
    };
    let target = wide(CREDENTIAL_TARGET);
    let mut credential = std::ptr::null_mut();
    let result = unsafe {
        CredReadW(
            PCWSTR(target.as_ptr()),
            CRED_TYPE_GENERIC,
            None,
            &mut credential,
        )
    };
    if result.is_err() {
        return Ok(None);
    }
    let value = unsafe {
        let credential = &*credential;
        std::slice::from_raw_parts(
            credential.CredentialBlob,
            credential.CredentialBlobSize as usize,
        )
        .to_vec()
    };
    unsafe { CredFree(credential as *const core::ffi::c_void) };
    Ok(Some(value))
}

#[cfg(windows)]
fn credential_delete() -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC},
    };
    let target = wide(CREDENTIAL_TARGET);
    match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if error.code().0 as u32 == 0x8007_0490 => Ok(()),
        Err(error) => Err(format!(
            "Windows Credential Manager could not remove the Spotify token: {error}"
        )),
    }
}

#[cfg(not(windows))]
fn credential_write(_: &[u8]) -> Result<(), String> {
    Err("Secure Spotify token storage requires Windows.".into())
}

#[cfg(not(windows))]
fn credential_read() -> Result<Option<Vec<u8>>, String> {
    Ok(None)
}

#[cfg(not(windows))]
fn credential_delete() -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc_7636_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = base64url(&sha256(verifier.as_bytes()).unwrap());
        assert_eq!(
            challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
