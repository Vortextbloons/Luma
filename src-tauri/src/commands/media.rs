use crate::integrations::spotify::{self, auth::Auth};
use tauri::Manager;

pub fn stop(app: &tauri::AppHandle) {
    spotify::shutdown(app);
}

enum PlayerTarget {
    Embedded(tauri::WebviewWindow),
    Sports,
    Service(String),
    #[cfg(windows)]
    Native(String),
    External,
    Music,
}

fn player_target(app: &tauri::AppHandle) -> PlayerTarget {
    crate::viewing::refresh_external_focus(app);
    if crate::viewing::external_selected(app) {
        return crate::viewing::external_source(app)
            .map(PlayerTarget::Service)
            .unwrap_or(PlayerTarget::External);
    }
    if let Some(window) = crate::viewing::selected(app) {
        return if window.label() == "sports" {
            PlayerTarget::Sports
        } else {
            PlayerTarget::Embedded(window)
        };
    }
    let mut windows: Vec<_> = ["sports", "youtube", "stremio"]
        .into_iter()
        .filter_map(|label| app.get_webview_window(label))
        .filter(|window| {
            window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
        })
        .collect();
    windows.sort_by_key(|window| !window.is_focused().unwrap_or(false));
    if let Some(window) = windows.into_iter().next() {
        return if window.label() == "sports" {
            PlayerTarget::Sports
        } else {
            PlayerTarget::Embedded(window)
        };
    }
    #[cfg(windows)]
    if let Some(executable) = crate::commands::session::native_executable(app) {
        return PlayerTarget::Native(executable);
    }
    crate::commands::services::current_source(app)
        .map(PlayerTarget::Service)
        .unwrap_or(PlayerTarget::Music)
}

pub async fn control(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    if !matches!(action, "toggle" | "previous" | "next") {
        return Err("Unknown playback command.".into());
    }
    match player_target(app) {
        PlayerTarget::Sports => crate::commands::sports::control(app, action).await,
        PlayerTarget::Embedded(window) => {
            let action = serde_json::to_string(action).map_err(|error| error.to_string())?;
            window
                .eval(format!(
                    "({})({action});",
                    include_str!("../../resources/playback.js")
                ))
                .map_err(|error| error.to_string())
        }
        PlayerTarget::Service(source) => {
            let action = serde_json::to_string(action).map_err(|error| error.to_string())?;
            let script = format!("(async () => {{ if (!document.querySelector('video')) return null; await ({})({action}); return true; }})()", include_str!("../../resources/playback.js"));
            let result = crate::commands::services::evaluate(app, &source, &script).await?;
            if result.as_bool() == Some(true) {
                Ok(())
            } else {
                Err("This player is not exposing playback controls.".into())
            }
        }
        #[cfg(windows)]
        PlayerTarget::Native(executable) => {
            let action = action.to_owned();
            let result = tauri::async_runtime::spawn_blocking(move || {
                native_playback(&executable, None, Some(&action))
            })
            .await
            .map_err(|error| error.to_string())??;
            if result.get("accepted").and_then(serde_json::Value::as_bool) == Some(true) {
                Ok(())
            } else {
                Err("This app is not exposing that playback control.".into())
            }
        }
        PlayerTarget::External => Err("Use this window's own playback controls.".into()),
        PlayerTarget::Music => music(app, action).await,
    }
}

pub fn has_video(app: &tauri::AppHandle) -> bool {
    !matches!(player_target(app), PlayerTarget::Music)
}

pub async fn music(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    if !matches!(action, "toggle" | "previous" | "next") {
        return Err("Unknown music command.".into());
    }
    let auth = app.state::<Auth>();
    spotify::control(app, &auth, action, None).await
}

pub async fn timeline(
    app: &tauri::AppHandle,
    position: Option<f64>,
) -> Result<serde_json::Value, String> {
    let script = format!(
        "({})({})",
        include_str!("../../resources/timeline.js"),
        serde_json::to_string(&position).map_err(|error| error.to_string())?
    );
    match player_target(app) {
        PlayerTarget::Sports => crate::commands::sports::timeline(app, position).await,
        PlayerTarget::Service(source) => {
            crate::commands::services::evaluate(app, &source, &script).await
        }
        #[cfg(windows)]
        PlayerTarget::Native(executable) => tauri::async_runtime::spawn_blocking(move || {
            native_playback(&executable, position, None)
        })
        .await
        .map_err(|error| error.to_string())?,
        PlayerTarget::External if position.is_some() => {
            Err("Use this window's own timeline controls.".into())
        }
        PlayerTarget::External | PlayerTarget::Music => Ok(serde_json::Value::Null),
        PlayerTarget::Embedded(window) => {
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let sender = std::sync::Mutex::new(Some(sender));
            window
                .eval_with_callback(script, move |result| {
                    if let Ok(mut sender) = sender.lock() {
                        if let Some(sender) = sender.take() {
                            let _ = sender.send(result);
                        }
                    }
                })
                .map_err(|error| error.to_string())?;
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), receiver)
                .await
                .map_err(|_| "The player did not respond. Try again.".to_string())?
                .map_err(|_| "The player closed before responding.".to_string())?;
            serde_json::from_str(&result).map_err(|error| error.to_string())
        }
    }
}

#[cfg(windows)]
fn matches_source(executable: &str, source: &str) -> bool {
    let executable = executable.to_ascii_lowercase();
    let source = source.to_ascii_lowercase();
    if source.is_empty() {
        return false;
    }
    if executable == source || executable.rsplit(['\\', '/']).next() == Some(source.as_str()) {
        return true;
    }
    source
        .split_once('!')
        .and_then(|(family, _)| family.rsplit_once('_'))
        .is_some_and(|(package, _)| executable.contains(&format!("\\windowsapps\\{package}_")))
}

#[cfg(windows)]
fn native_playback(
    executable: &str,
    position: Option<f64>,
    action: Option<&str>,
) -> Result<serde_json::Value, String> {
    use windows::{
        Media::Control::GlobalSystemMediaTransportControlsSessionManager,
        Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
    };
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe { RoUninitialize() };
            }
        }
    }
    let apartment = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    let _apartment = match apartment {
        Ok(()) => Apartment(true),
        Err(error) if error.code() == windows::Win32::Foundation::RPC_E_CHANGED_MODE => {
            Apartment(false)
        }
        Err(error) => return Err(error.to_string()),
    };
    let read = || -> windows::core::Result<serde_json::Value> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
        let sessions = manager.GetSessions()?;
        for index in 0..sessions.Size()? {
            let session = sessions.GetAt(index)?;
            if !matches_source(executable, &session.SourceAppUserModelId()?.to_string()) {
                continue;
            }
            if let Some(action) = action {
                let controls = session.GetPlaybackInfo()?.Controls()?;
                let operation = match action {
                    "toggle" if controls.IsPlayPauseToggleEnabled()? => {
                        session.TryTogglePlayPauseAsync()?
                    }
                    "previous" | "next" if controls.IsPlaybackPositionEnabled()? => {
                        let timeline = session.GetTimelineProperties()?;
                        let delta = if action == "next" {
                            100_000_000
                        } else {
                            -100_000_000
                        };
                        let target = timeline.Position()?.Duration.saturating_add(delta).clamp(
                            timeline.StartTime()?.Duration,
                            timeline
                                .EndTime()?
                                .Duration
                                .max(timeline.StartTime()?.Duration),
                        );
                        session.TryChangePlaybackPositionAsync(target)?
                    }
                    "previous" if controls.IsPreviousEnabled()? => {
                        session.TrySkipPreviousAsync()?
                    }
                    "next" if controls.IsNextEnabled()? => session.TrySkipNextAsync()?,
                    _ => return Ok(serde_json::json!({ "accepted": false })),
                };
                return Ok(serde_json::json!({ "accepted": operation.get()? }));
            }
            if !session
                .GetPlaybackInfo()?
                .Controls()?
                .IsPlaybackPositionEnabled()?
            {
                continue;
            }
            let timeline = session.GetTimelineProperties()?;
            let start = timeline.StartTime()?.Duration;
            let end = timeline.EndTime()?.Duration;
            if end <= start {
                continue;
            }
            let target = position.map(|position| {
                start
                    .saturating_add((position * 10_000_000.0) as i64)
                    .clamp(start, end)
            });
            if let Some(target) = target {
                if !session.TryChangePlaybackPositionAsync(target)?.get()? {
                    return Ok(
                        serde_json::json!({ "error": "The app declined the seek request. Try again." }),
                    );
                }
            }
            return Ok(serde_json::json!({
                "position": (target.unwrap_or(timeline.Position()?.Duration) - start).max(0) as f64 / 10_000_000.0,
                "duration": (end - start) as f64 / 10_000_000.0,
            }));
        }
        Ok(serde_json::Value::Null)
    };
    read().map_err(|error| error.to_string())
}

#[cfg(all(test, windows))]
mod tests {
    use super::matches_source;

    #[test]
    fn native_playback_only_matches_the_launched_application() {
        assert!(matches_source(r"C:\Players\player.exe", "player.exe"));
        assert!(matches_source(
            r"C:\Program Files\WindowsApps\Netflix.Netflix_1.0_x64_publisher\app.exe",
            "Netflix.Netflix_publisher!App"
        ));
        assert!(!matches_source(r"C:\Players\player.exe", "spotify.exe"));
        assert!(!matches_source(r"C:\Players\player.exe", ""));
        assert!(!matches_source(
            r"C:\Players\app.exe",
            "Netflix.Netflix_publisher!App"
        ));
    }
}
