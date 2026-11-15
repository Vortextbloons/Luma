use crate::commands::{apps, youtube};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Emitter, Manager};

const APPEARANCE_TOKENS: [&str; 20] = [
    "--bg",
    "--bg-soft",
    "--ambient",
    "--surface",
    "--surface-2",
    "--surface-3",
    "--line",
    "--line-strong",
    "--text",
    "--text-dim",
    "--muted",
    "--dim",
    "--accent",
    "--accent-strong",
    "--accent-ink",
    "--accent-soft",
    "--good",
    "--warn",
    "--danger",
    "--rating",
];
const MAX_APPEARANCE_TOKENS: usize = 24;
const MAX_THEMES: usize = 8;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SoundPreset {
    #[default]
    Real,
    Google,
    Roku,
    Off,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    theme: String,
    tokens: BTreeMap<String, String>,
    themes: Vec<ThemeOption>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeOption {
    id: String,
    name: String,
    tokens: BTreeMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingOption {
    value: String,
    label: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingChoice {
    key: String,
    group: String,
    label: String,
    description: String,
    options: Vec<SettingOption>,
    value: String,
}

#[tauri::command]
pub fn update_settings(remote: tauri::State<Remote>, settings: Vec<SettingChoice>) -> Result<(), String> {
    remote.state.lock().map_err(|error| error.to_string())?.settings = settings;
    Ok(())
}

pub(crate) fn change_setting(app: &tauri::AppHandle, key: String, value: String) -> Result<(), String> {
    if matches!(key.as_str(), "theme" | "density" | "appTileShape") {
        let document = app.state::<crate::customization::Store>().edit(|config| match key.as_str() {
            "theme" => config.theme = value.clone(),
            "density" => config.density = value.clone(),
            "appTileShape" => config.app_tile_shape = value.clone(),
            _ => {},
        }).map_err(|error| error.message)?;
        crate::customization::publish(app, &document);
        return Ok(());
    }
    {
        let remote = app.state::<Remote>();
        let state = remote.state.lock().map_err(|error| error.to_string())?;
        let choice = state.settings.iter().find(|choice| choice.key == key).ok_or("Unknown setting")?;
        if !choice.options.iter().any(|option| option.value == value) {
            return Err("Invalid setting value".into());
        }
    }
    app.emit_to("main", "app-setting", serde_json::json!({ "key": key, "value": value })).map_err(|error| error.to_string())
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    #[serde(skip)]
    customization_revision: u64,
    view: String,
    fullscreen: bool,
    sound: SoundPreset,
    window: Option<String>,
    window_fullscreen: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    appearance: Option<Appearance>,
    app_order: Vec<String>,
    settings: Vec<SettingChoice>,
}

impl AppState {
    pub(crate) fn app_order(&self) -> &[String] {
        &self.app_order
    }
}

fn valid_token_value(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_tokens(tokens: &BTreeMap<String, String>) -> Result<(), String> {
    if tokens.len() > MAX_APPEARANCE_TOKENS {
        return Err(format!(
            "Appearance has too many tokens (maximum {MAX_APPEARANCE_TOKENS})."
        ));
    }
    for (name, value) in tokens {
        if !APPEARANCE_TOKENS.contains(&name.as_str()) {
            return Err(format!("Unsupported appearance token: {name}."));
        }
        if !valid_token_value(value) {
            return Err(format!("{name} must be a #rrggbb or #rrggbbaa color."));
        }
    }
    Ok(())
}

fn valid_theme_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_appearance(appearance: &Appearance) -> Result<(), String> {
    if !valid_theme_id(&appearance.theme) {
        return Err("Appearance theme id is invalid.".into());
    }
    validate_tokens(&appearance.tokens)?;
    if appearance.themes.len() > MAX_THEMES {
        return Err(format!(
            "Appearance has too many themes (maximum {MAX_THEMES})."
        ));
    }
    for theme in &appearance.themes {
        if !valid_theme_id(&theme.id) {
            return Err(format!("Theme id is invalid: {}.", theme.id));
        }
        if theme.name.chars().count() > 48 || theme.name.chars().any(char::is_control) {
            return Err(format!("Theme name is invalid: {}.", theme.id));
        }
        validate_tokens(&theme.tokens)?;
    }
    Ok(())
}

pub(crate) fn normalize_app_order(order: Vec<String>) -> Vec<String> {
    let names: Vec<String> = apps::catalog()
        .into_iter()
        .map(|entry| entry.name.to_string())
        .collect();
    let mut result: Vec<String> = Vec::new();
    for name in order {
        if name == "Custom" || !names.iter().any(|known| known == &name) || result.contains(&name) {
            continue;
        }
        result.push(name);
    }
    for name in &names {
        if name != "Custom" && !result.contains(name) {
            result.push(name.clone());
        }
    }
    result.push("Custom".into());
    result
}

pub(crate) fn set_app_order(
    app: &tauri::AppHandle,
    order: Vec<String>,
) -> Result<(), String> {
    let order = normalize_app_order(order);
    let document = app.state::<crate::customization::Store>().edit(|config| config.order = order).map_err(|error| error.message)?;
    crate::customization::publish(app, &document);
    Ok(())
}

pub(crate) fn sync_customization(app: &tauri::AppHandle, document: &crate::customization::document::Document) -> Result<(), String> {
    let remote = app.state::<Remote>();
    let mut state = remote.state.lock().map_err(|error| error.to_string())?;
    if document.revision < state.customization_revision { return Ok(()); }
    state.customization_revision = document.revision;
    let config = &document.config;
    state.app_order = config.order.clone();
    remote.app_order.send_replace(config.order.clone());
    if let Some(appearance) = &mut state.appearance {
        if let Some(theme) = appearance.themes.iter().find(|theme| theme.id == config.theme) {
            appearance.theme = theme.id.clone();
            appearance.tokens = theme.tokens.clone();
        }
    }
    remote.appearance.send_replace(state.appearance.clone());
    for choice in &mut state.settings {
        match choice.key.as_str() {
            "theme" => choice.value = config.theme.clone(),
            "density" => choice.value = config.density.clone(),
            "appTileShape" => choice.value = config.app_tile_shape.clone(),
            _ => {},
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NavigationCommand {
    Up,
    Down,
    Left,
    Right,
    Activate,
    Back,
    Home,
}

impl NavigationCommand {
    fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Activate => "activate",
            Self::Back => "back",
            Self::Home => "home",
        }
    }
}

fn should_wake(visible: bool, external_session: bool) -> bool {
    !visible && !external_session
}

pub(crate) async fn wake(app: &tauri::AppHandle) -> Result<(), String> {
    let visible = ["main", "youtube", "stremio"].iter().any(|label|
        app.get_webview_window(label).is_some_and(|window| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)));
    if should_wake(visible, crate::commands::session::active(app)) {
        let window = crate::background::main_window(app).await?;
        crate::background::present(window, None).await?;
    }
    Ok(())
}

pub(crate) fn stop(app: &tauri::AppHandle) -> Result<(), String> {
    crate::commands::session::close_steam(app);
    crate::remote::pointer::stop_all(app);
    crate::remote::pointer::show_cursor(app);
    for label in ["main", "youtube", "stremio"] {
        if let Some(window) = app.get_webview_window(label) {
            if label != "main" { let _ = window.eval("document.querySelectorAll('video,audio').forEach(media => media.pause())"); }
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    crate::background::unload(app, "main");
    crate::background::unload(app, "youtube");
    crate::background::unload(app, "stremio");
    let _ = crate::tv::restore_audio_sync();
    Ok(())
}

#[cfg(windows)]
pub(crate) fn type_text(text: &str) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE, KEYEVENTF_KEYUP};
    let inputs: Vec<INPUT> = text.encode_utf16().flat_map(|unit| [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP].map(|flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wScan: unit, dwFlags: flags, ..Default::default() } },
    })).collect();
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() { return Err("Windows could not type the text. Focus a text field on the PC and try again.".into()); }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn type_text(_: &str) -> Result<(), String> { Err("Remote typing requires Windows.".into()) }

pub(crate) fn navigate(app: &tauri::AppHandle, command: NavigationCommand) {
    match command {
        NavigationCommand::Home => {
            crate::remote::pointer::stop_all(app);
            crate::remote::pointer::hide_cursor(app);
        }
        NavigationCommand::Up
        | NavigationCommand::Down
        | NavigationCommand::Left
        | NavigationCommand::Right
        | NavigationCommand::Activate
        | NavigationCommand::Back => crate::remote::pointer::hide_cursor(app),
    }
    let handled = crate::commands::session::route(app, command.as_str());
    let embedded = if handled {
        false
    } else if app
        .get_webview_window("stremio")
        .filter(|window| window.is_visible().unwrap_or(false))
        .is_some()
    {
        crate::commands::stremio::forward(app, command.as_str());
        true
    } else if app
        .get_webview_window("youtube")
        .filter(|window| window.is_visible().unwrap_or(false))
        .is_some()
    {
        youtube::forward(app, command.as_str());
        true
    } else {
        false
    };
    if frontend_receives(handled, embedded, command) {
        if show_main(handled, embedded, command) {
            crate::background::show(app);
        }
        let _ = app.emit("remote-navigation", command);
    }
}

fn frontend_receives(handled: bool, embedded: bool, command: NavigationCommand) -> bool {
    matches!(command, NavigationCommand::Home) || (!handled && !embedded)
}

fn show_main(handled: bool, embedded: bool, command: NavigationCommand) -> bool {
    matches!(command, NavigationCommand::Home) && !handled && !embedded
}

pub(crate) fn open_music(app: &tauri::AppHandle) -> Result<(), String> {
    crate::commands::session::release(app)?;
    for label in ["stremio", "youtube"] {
        if app
            .get_webview_window(label)
            .is_some_and(|window| window.is_visible().unwrap_or(false))
        {
            if label == "stremio" {
                crate::commands::stremio::close(app);
            } else {
                youtube::close(app);
            }
        }
    }
    crate::background::show(app);
    app.emit_to("main", "open-music", ())
        .map_err(|error| error.to_string())?;
    app.state::<Remote>()
        .state
        .lock()
        .map_err(|error| error.to_string())?
        .view = "Music".into();
    Ok(())
}

pub struct Remote {
    state: Mutex<AppState>,
    appearance: tokio::sync::watch::Sender<Option<Appearance>>,
    app_order: tokio::sync::watch::Sender<Vec<String>>,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            state: Mutex::new(AppState {
                customization_revision: 0,
                view: "Home".into(),
                fullscreen: false,
                sound: SoundPreset::Real,
                window: None,
                window_fullscreen: None,
                appearance: None,
                app_order: Vec::new(),
                settings: Vec::new(),
            }),
            appearance: tokio::sync::watch::channel(None).0,
            app_order: tokio::sync::watch::channel(Vec::new()).0,
        }
    }
}

impl Remote {
    pub(crate) fn subscribe_appearance(
        &self,
    ) -> tokio::sync::watch::Receiver<Option<Appearance>> {
        self.appearance.subscribe()
    }

    pub(crate) fn subscribe_app_order(&self) -> tokio::sync::watch::Receiver<Vec<String>> {
        self.app_order.subscribe()
    }
}

fn active_window(app: &tauri::AppHandle) -> Option<(&'static str, tauri::WebviewWindow)> {
    let windows: Vec<_> = [("main", "Luma"), ("youtube", "YouTube"), ("stremio", "Stremio")]
        .into_iter()
        .filter_map(|(label, name)| {
            let window = app.get_webview_window(label)?;
            if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
                return None;
            }
            Some((name, window))
        })
        .collect();
    windows
        .iter()
        .find(|(_, window)| window.is_focused().unwrap_or(false))
        .or_else(|| windows.first())
        .cloned()
}

pub(crate) fn snapshot(app: &tauri::AppHandle) -> Result<AppState, String> {
    app.state::<Remote>()
        .state
        .lock()
        .map(|state| state.clone())
        .map_err(|e| e.to_string())
}

pub(crate) fn snapshot_live(app: &tauri::AppHandle) -> Result<AppState, String> {
    let window = active_window(app).map(|(name, window)| (name, window.is_fullscreen().ok()));
    let remote = app.state::<Remote>();
    let mut state = remote.state.lock().map_err(|e| e.to_string())?;
    if let Some((name, fullscreen)) = window {
        state.window = Some(name.into());
        state.window_fullscreen = fullscreen;
    } else {
        state.window = None;
        state.window_fullscreen = None;
    }
    Ok(state.clone())
}

pub(crate) fn fullscreen(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(result) = crate::commands::session::fullscreen(app) {
        return result;
    }
    let (_, window) = active_window(app).ok_or("Open Luma or an app before using PC fullscreen.")?;
    let value = !window.is_fullscreen().map_err(|error| error.to_string())?;
    window
        .set_fullscreen(value)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_app_state(
    remote: tauri::State<Remote>,
    view: String,
    fullscreen: bool,
    sound: SoundPreset,
) -> Result<(), String> {
    if !["Home", "Watch", "Music", "Apps", "Settings", "Details"].contains(&view.as_str()) {
        return Err("Invalid view.".into());
    }
    let mut state = remote.state.lock().map_err(|e| e.to_string())?;
    state.view = view;
    state.fullscreen = fullscreen;
    state.sound = sound;
    Ok(())
}

#[tauri::command]
pub fn update_appearance(
    remote: tauri::State<Remote>,
    appearance: Appearance,
) -> Result<(), String> {
    validate_appearance(&appearance)?;
    remote
        .state
        .lock()
        .map_err(|error| error.to_string())?
        .appearance = Some(appearance.clone());
    remote.appearance.send_replace(Some(appearance));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        frontend_receives, show_main, should_wake, validate_appearance, Appearance,
        NavigationCommand, SoundPreset, ThemeOption,
    };
    use std::collections::BTreeMap;

    fn theme(id: &str, name: &str) -> ThemeOption {
        ThemeOption {
            id: id.into(),
            name: name.into(),
            tokens: BTreeMap::from([("--bg".into(), "#101010".into())]),
        }
    }

    fn valid_appearance() -> Appearance {
        Appearance {
            theme: "violet-night".into(),
            tokens: BTreeMap::from([
                ("--bg".into(), "#0b0d12".into()),
                ("--line".into(), "#ffffff17".into()),
            ]),
            themes: vec![
                theme("violet-night", "Violet Night"),
                theme("mint", "Mint"),
            ],
        }
    }

    #[test]
    fn appearance_validation_accepts_a_theme_payload() {
        assert!(validate_appearance(&valid_appearance()).is_ok());
    }

    #[test]
    fn appearance_validation_rejects_unknown_tokens() {
        let mut appearance = valid_appearance();
        appearance.tokens.insert("--evil".into(), "#ffffff".into());
        assert!(validate_appearance(&appearance).is_err());
        let mut nested = valid_appearance();
        nested.themes[0]
            .tokens
            .insert("--text-shadow".into(), "#000000".into());
        assert!(validate_appearance(&nested).is_err());
    }

    #[test]
    fn appearance_validation_rejects_invalid_hex_colors() {
        let mut appearance = valid_appearance();
        appearance.tokens.insert("--accent".into(), "red".into());
        assert!(validate_appearance(&appearance).is_err());
        appearance.tokens.insert("--accent".into(), "#12345".into());
        assert!(validate_appearance(&appearance).is_err());
        appearance.tokens.insert("--accent".into(), "#12345g".into());
        assert!(validate_appearance(&appearance).is_err());
    }

    #[test]
    fn appearance_validation_rejects_too_many_themes() {
        let mut appearance = valid_appearance();
        appearance.themes = (0..9)
            .map(|index| theme(&format!("theme-{index}"), "Theme"))
            .collect();
        assert!(validate_appearance(&appearance).is_err());
        appearance.themes.truncate(8);
        assert!(validate_appearance(&appearance).is_ok());
    }

    #[test]
    fn app_order_keeps_known_apps_and_forces_custom_last() {
        let order = super::normalize_app_order(vec![
            "Netflix".into(),
            "Netflix".into(),
            "Unknown".into(),
            "Custom".into(),
            "YouTube".into(),
        ]);
        assert_eq!(order.first().map(String::as_str), Some("Netflix"));
        assert_eq!(order.get(1).map(String::as_str), Some("YouTube"));
        assert_eq!(order.last().map(String::as_str), Some("Custom"));
        assert_eq!(order.iter().filter(|name| name.as_str() == "Custom").count(), 1);
        assert!(order.iter().any(|name| name == "Steam"));
    }

    #[test]
    fn external_session_prevents_waking_the_launcher() {
        assert!(!should_wake(false, true));
        assert!(should_wake(false, false));
        assert!(!should_wake(true, false));
    }

    #[test]
    fn home_always_resets_the_luma_frontend() {
        assert!(frontend_receives(true, false, NavigationCommand::Home));
        assert!(frontend_receives(false, true, NavigationCommand::Home));
        assert!(frontend_receives(false, false, NavigationCommand::Home));
    }

    #[test]
    fn routed_commands_are_not_duplicated_on_the_frontend() {
        assert!(!frontend_receives(true, false, NavigationCommand::Left));
        assert!(!frontend_receives(false, true, NavigationCommand::Activate));
        assert!(frontend_receives(false, false, NavigationCommand::Left));
    }

    #[test]
    fn pc_window_state_uses_the_field_names_the_remotes_read() {
        let state = super::AppState {
            customization_revision: 0,
            view: "Home".into(),
            fullscreen: false,
            sound: SoundPreset::Real,
            window: Some("YouTube".into()),
            window_fullscreen: Some(true),
            appearance: None,
            app_order: Vec::new(),
                settings: Vec::new(),
        };
        let value = serde_json::to_value(state).unwrap();
        assert_eq!(value["window"], "YouTube");
        assert_eq!(value["windowFullscreen"], true);
        assert_eq!(value["sound"], "real");
    }

    #[test]
    fn home_brings_back_a_hidden_luma_but_not_a_handled_app() {
        assert!(show_main(false, false, NavigationCommand::Home));
        assert!(!show_main(true, false, NavigationCommand::Home));
        assert!(!show_main(false, true, NavigationCommand::Home));
        assert!(!show_main(false, false, NavigationCommand::Left));
    }
}
