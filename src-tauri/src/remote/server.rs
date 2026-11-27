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
const MAX_THEMES: usize = 12;

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
    #[serde(default)]
    section: String,
    #[serde(default = "default_setting_kind")]
    kind: String,
    label: String,
    description: String,
    options: Vec<SettingOption>,
    value: String,
}

fn default_setting_kind() -> String {
    "choice".into()
}

fn setting_option(value: &str, label: &str) -> SettingOption {
    SettingOption {
        value: value.into(),
        label: label.into(),
    }
}

fn setting_choice(
    key: &str,
    group: &str,
    section: &str,
    label: &str,
    description: &str,
    value: &str,
    options: Vec<SettingOption>,
) -> SettingChoice {
    SettingChoice {
        key: key.into(),
        group: group.into(),
        section: section.into(),
        kind: "choice".into(),
        label: label.into(),
        description: description.into(),
        options,
        value: value.into(),
    }
}

fn setting_catalog(config: &crate::customization::document::Config) -> Vec<SettingChoice> {
    vec![
        setting_choice(
            "theme",
            "Appearance",
            "theme",
            "App theme",
            "The look of Luma on the TV and connected displays.",
            &config.theme,
            vec![
                setting_option("violet-night", "Violet Night"),
                setting_option("sapphire", "Sapphire"),
                setting_option("mint", "Mint"),
                setting_option("ember", "Ember"),
                setting_option("graphite", "Graphite"),
                setting_option("onyx", "Onyx"),
                setting_option("aurora", "Aurora"),
                setting_option("synthwave", "Synthwave"),
                setting_option("biolume", "Biolume"),
            ],
        ),
        setting_choice(
            "appTileShape",
            "Layout",
            "layout",
            "App tile shape",
            "Choose wide or square app tiles.",
            &config.app_tile_shape,
            vec![
                setting_option("wide", "Pills"),
                setting_option("square", "Squares"),
            ],
        ),
        setting_choice(
            "density",
            "Layout",
            "layout",
            "Spacing",
            "Compact fits more rows on screen.",
            &config.density,
            vec![
                setting_option("comfortable", "Comfortable"),
                setting_option("compact", "Compact"),
            ],
        ),
        setting_choice(
            "textSize",
            "Layout",
            "layout",
            "Text size",
            "Make text and controls easier to read from the sofa.",
            &config.text_size,
            vec![
                setting_option("standard", "Standard"),
                setting_option("large", "Large"),
                setting_option("larger", "Extra large"),
            ],
        ),
        SettingChoice {
            key: "reducedMotion".into(),
            group: "Layout".into(),
            section: "layout".into(),
            kind: "toggle".into(),
            label: "Reduce motion".into(),
            description: "Use fewer animations.".into(),
            options: vec![setting_option("true", "On"), setting_option("false", "Off")],
            value: config.reduced_motion.to_string(),
        },
        setting_choice(
            "interfaceSounds",
            "Sound",
            "sound",
            "Menu sounds",
            "Play a sound when moving or selecting.",
            &config.interface_sounds,
            vec![
                setting_option("real", "Real hardware"),
                setting_option("google", "Google TV"),
                setting_option("roku", "Roku"),
                setting_option("off", "Off"),
            ],
        ),
        setting_choice(
            "soundLevel",
            "Sound",
            "sound",
            "Menu volume",
            "Standard is tuned for a quiet room.",
            &config.sound_level,
            vec![
                setting_option("soft", "Soft"),
                setting_option("standard", "Standard"),
                setting_option("loud", "Loud"),
            ],
        ),
        setting_choice(
            "homeColumns",
            "Layout",
            "remote-layout",
            "Home columns",
            "One or two columns of rows on Home.",
            &config.home_columns.to_string(),
            vec![
                setting_option("1", "One column"),
                setting_option("2", "Two columns"),
            ],
        ),
    ]
}

fn sound_preset(value: &str) -> SoundPreset {
    match value {
        "google" => SoundPreset::Google,
        "roku" => SoundPreset::Roku,
        "off" => SoundPreset::Off,
        _ => SoundPreset::Real,
    }
}

pub(crate) fn change_setting(
    app: &tauri::AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    {
        let remote = app.state::<Remote>();
        let state = remote.state.lock().map_err(|error| error.to_string())?;
        let choice = state
            .settings
            .iter()
            .find(|choice| choice.key == key)
            .ok_or("Unknown setting")?;
        if !choice.options.iter().any(|option| option.value == value) {
            return Err("Invalid setting value".into());
        }
    }
    if !matches!(
        key.as_str(),
        "theme"
            | "density"
            | "appTileShape"
            | "textSize"
            | "reducedMotion"
            | "interfaceSounds"
            | "soundLevel"
            | "homeColumns"
    ) {
        return Err("Unknown setting".into());
    }
    let document = app
        .state::<crate::customization::Store>()
        .edit(|config| match key.as_str() {
            "theme" => config.theme = value.clone(),
            "density" => config.density = value.clone(),
            "appTileShape" => config.app_tile_shape = value.clone(),
            "textSize" => config.text_size = value.clone(),
            "reducedMotion" => config.reduced_motion = value == "true",
            "interfaceSounds" => config.interface_sounds = value.clone(),
            "soundLevel" => config.sound_level = value.clone(),
            "homeColumns" => config.home_columns = if value == "2" { 2 } else { 1 },
            _ => {}
        })
        .map_err(|error| error.message)?;
    crate::customization::publish(app, &document);
    Ok(())
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
    #[serde(skip_serializing_if = "Option::is_none")]
    music: Option<MusicState>,
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MusicState {
    pub playing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    pub has_artwork: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub updated_at: u64,
    pub can_toggle: bool,
    pub can_skip_next: bool,
    pub can_skip_previous: bool,
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

pub(crate) fn set_app_order(app: &tauri::AppHandle, order: Vec<String>) -> Result<(), String> {
    let order = normalize_app_order(order);
    let document = app
        .state::<crate::customization::Store>()
        .edit(|config| config.order = order)
        .map_err(|error| error.message)?;
    crate::customization::publish(app, &document);
    Ok(())
}

pub(crate) fn sync_customization(
    app: &tauri::AppHandle,
    document: &crate::customization::document::Document,
) -> Result<(), String> {
    let remote = app.state::<Remote>();
    let mut state = remote.state.lock().map_err(|error| error.to_string())?;
    if document.revision < state.customization_revision {
        return Ok(());
    }
    state.customization_revision = document.revision;
    let config = &document.config;
    state.app_order = config.order.clone();
    remote.app_order.send_replace(config.order.clone());
    if let Some(appearance) = &mut state.appearance {
        if let Some(theme) = appearance
            .themes
            .iter()
            .find(|theme| theme.id == config.theme)
        {
            appearance.theme = theme.id.clone();
            appearance.tokens = theme.tokens.clone();
        }
    }
    remote.appearance.send_replace(state.appearance.clone());
    state.settings = setting_catalog(config);
    state.sound = sound_preset(&config.interface_sounds);
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
    let visible = crate::extension_spike::active(app)
        || ["main", "youtube", "stremio", "sports"]
            .iter()
            .any(|label| {
                app.get_webview_window(label).is_some_and(|window| {
                    window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
                })
            });
    if should_wake(visible, crate::commands::session::active(app)) {
        let window = crate::background::main_window(app).await?;
        crate::background::present(window, None).await?;
    }
    Ok(())
}

pub(crate) fn stop(app: &tauri::AppHandle) -> Result<(), String> {
    crate::viewing::restore_external(app);
    crate::commands::session::close_steam(app);
    crate::extension_spike::launcher_hidden(app);
    crate::remote::pointer::stop_all(app);
    crate::remote::pointer::show_cursor(app);
    for label in ["main", "youtube", "stremio", "sports"] {
        if let Some(window) = app.get_webview_window(label) {
            if label != "main" {
                let _ = window.eval(
                    "document.querySelectorAll('video,audio').forEach(media => media.pause())",
                );
            }
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    crate::background::unload(app, "main");
    crate::background::unload(app, "youtube");
    crate::background::unload(app, "stremio");
    crate::background::unload(app, "sports");
    let _ = crate::tv::restore_audio_sync();
    Ok(())
}

#[cfg(windows)]
pub(crate) fn type_text(text: &str) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    };
    let inputs: Vec<INPUT> = text
        .encode_utf16()
        .flat_map(|unit| {
            [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP].map(|flags| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wScan: unit,
                        dwFlags: flags,
                        ..Default::default()
                    },
                },
            })
        })
        .collect();
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(
            "Windows could not type the text. Focus a text field on the PC and try again.".into(),
        );
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn type_text(_: &str) -> Result<(), String> {
    Err("Remote typing requires Windows.".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpikeRoute {
    Dismiss,
    Close,
    Forward,
}

fn spike_route(command: NavigationCommand) -> SpikeRoute {
    match command {
        NavigationCommand::Home => SpikeRoute::Dismiss,
        NavigationCommand::Back => SpikeRoute::Close,
        NavigationCommand::Up
        | NavigationCommand::Down
        | NavigationCommand::Left
        | NavigationCommand::Right
        | NavigationCommand::Activate => SpikeRoute::Forward,
    }
}

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
    if crate::extension_spike::active(app) {
        match spike_route(command) {
            SpikeRoute::Dismiss => crate::extension_spike::dismiss(app),
            SpikeRoute::Close => {
                crate::extension_spike::close(app);
                return;
            }
            SpikeRoute::Forward => {
                crate::extension_spike::forward(app, command.as_str());
                return;
            }
        }
    }
    if crate::viewing::navigate(app, command.as_str()) {
        return;
    }
    if crate::viewing::browsing(app) {
        let _ = app.emit_to("main", "remote-navigation", command);
        return;
    }
    if app
        .get_webview_window("sports")
        .is_some_and(|window| window.is_visible().unwrap_or(false))
    {
        crate::commands::sports::forward(app, command.as_str());
        return;
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
    let watching = crate::commands::media::has_video(app)
        || crate::commands::session::active(app)
        || app
            .get_webview_window("sports")
            .is_some_and(|window| window.is_visible().unwrap_or(false));
    if watching {
        return Ok(());
    }
    crate::commands::session::release(app)?;
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

pub(crate) async fn open_sports(app: &tauri::AppHandle, league: &str) -> Result<(), String> {
    crate::commands::session::release(app)?;
    let window = crate::background::main_window(app).await?;
    crate::background::present(window, None).await?;
    app.emit_to("main", "open-sports", league)
        .map_err(|error| error.to_string())?;
    app.state::<Remote>()
        .state
        .lock()
        .map_err(|error| error.to_string())?
        .view = "Sports".into();
    Ok(())
}

pub struct Remote {
    state: Mutex<AppState>,
    appearance: tokio::sync::watch::Sender<Option<Appearance>>,
    app_order: tokio::sync::watch::Sender<Vec<String>>,
    music: tokio::sync::watch::Sender<Option<MusicState>>,
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
                settings: setting_catalog(&crate::customization::document::Config::default()),
                music: None,
            }),
            appearance: tokio::sync::watch::channel(None).0,
            app_order: tokio::sync::watch::channel(Vec::new()).0,
            music: tokio::sync::watch::channel(None).0,
        }
    }
}

impl Remote {
    pub(crate) fn subscribe_appearance(&self) -> tokio::sync::watch::Receiver<Option<Appearance>> {
        self.appearance.subscribe()
    }

    pub(crate) fn subscribe_app_order(&self) -> tokio::sync::watch::Receiver<Vec<String>> {
        self.app_order.subscribe()
    }

    pub(crate) fn subscribe_music(&self) -> tokio::sync::watch::Receiver<Option<MusicState>> {
        self.music.subscribe()
    }

    pub(crate) fn publish_music(&self, state: Option<MusicState>) {
        if let Ok(mut current) = self.state.try_lock() {
            current.music = state.clone();
        }
        self.music.send_replace(state);
    }
}

fn active_window(app: &tauri::AppHandle) -> Option<(&'static str, tauri::WebviewWindow)> {
    if let Some(window) = crate::viewing::selected(app) {
        let name = match window.label() {
            "youtube" => "YouTube",
            "stremio" => "Stremio",
            _ => "Sports",
        };
        return Some((name, window));
    }
    let windows: Vec<_> = [
        ("main", "Luma"),
        ("youtube", "YouTube"),
        ("stremio", "Stremio"),
        ("sports", "Sports"),
    ]
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
    crate::viewing::refresh_external_focus(app);
    let window = crate::viewing::external_name(app)
        .map(|name| (name, None))
        .or_else(|| {
            active_window(app).map(|(name, window)| (name.into(), window.is_fullscreen().ok()))
        });
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

fn current_music(app: &tauri::AppHandle) -> Option<MusicState> {
    let remote = app.try_state::<Remote>()?;
    let receiver = remote.subscribe_music();
    let music = receiver.borrow().clone();
    music
}

fn panels_response(panels: Vec<crate::customization::plugin::RemotePanel>) -> serde_json::Value {
    serde_json::json!({ "panels": panels })
}

pub(crate) fn phone_panels(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    crate::customization::plugin::remote_panels(app)
        .map(panels_response)
        .map_err(|error| error.message)
}

fn playback_command(action: crate::customization::plugin::RemoteAction) -> &'static str {
    use crate::customization::plugin::RemoteAction;

    match action {
        RemoteAction::PlaybackToggle => "toggle",
        RemoteAction::PlaybackNext => "next",
        RemoteAction::PlaybackPrevious => "previous",
    }
}

fn action_response(
    accepted: bool,
    code: Option<&str>,
    error: Option<String>,
    music: Option<MusicState>,
) -> serde_json::Value {
    let mut response = serde_json::Map::new();
    response.insert("accepted".into(), accepted.into());
    response.insert(
        "status".into(),
        if accepted {
            "success"
        } else if code == Some("plugin_busy") {
            "pending"
        } else {
            "failure"
        }
        .into(),
    );
    if let Some(code) = code {
        response.insert("code".into(), code.into());
    }
    if let Some(error) = error {
        response.insert("error".into(), error.into());
    }
    if let Some(music) = music {
        response.insert("music".into(), serde_json::json!(music));
    }
    response.into()
}

pub(crate) async fn phone_panel_action(
    app: &tauri::AppHandle,
    id: String,
    action: String,
    version: Option<String>,
) -> Result<serde_json::Value, String> {
    use crate::customization::plugin::RemoteInvocation;

    let invocation = match crate::customization::plugin::remote_invocation(app, &id, &action) {
        Ok(invocation) => invocation,
        Err(error) => {
            return Ok(action_response(
                false,
                Some(error.code),
                Some(error.message),
                current_music(app),
            ))
        }
    };
    let failure = match invocation {
        RemoteInvocation::Playback(action) => {
            crate::commands::media::music(app, playback_command(action))
                .await
                .err()
                .map(|error| ("unavailable", error))
        }
        RemoteInvocation::Command(spec) => {
            if version.as_deref() != Some(spec.version.as_str()) {
                Some(("stale", "This panel changed. Try the updated control.".into()))
            } else {
                crate::extension_spike::dispatch_remote_command(app, spec)
                    .await
                    .err()
                    .map(|error| (error.code, error.message))
            }
        }
    };
    match failure {
        None => Ok(action_response(true, None, None, current_music(app))),
        Some((code, error)) => Ok(action_response(
            false,
            Some(code),
            Some(error),
            current_music(app),
        )),
    }
}

pub(crate) fn sync_music(
    app: &tauri::AppHandle,
    state: &crate::integrations::spotify::models::PlaybackState,
) {
    let music = (state.player_available || state.title.is_some()).then(|| MusicState {
        playing: state.playing,
        title: state.title.clone().filter(|value| !value.trim().is_empty()),
        artist: state
            .artist
            .clone()
            .filter(|value| !value.trim().is_empty()),
        has_artwork: state.artwork_url.is_some(),
        position_ms: state.position_ms,
        duration_ms: state.duration_ms,
        updated_at: state.updated_at,
        can_toggle: state.can_toggle,
        can_skip_next: state.can_skip_next,
        can_skip_previous: state.can_skip_previous,
    });
    app.state::<Remote>().publish_music(music);
    crate::extension_spike::playback_changed(app);
}

pub(crate) fn fullscreen(app: &tauri::AppHandle) -> Result<(), String> {
    if !crate::viewing::browsing(app) {
        if let Some(label) = crate::viewing::active_selection(app) {
            return crate::viewing::viewing_configure(
                app.clone(),
                crate::viewing::Mode::Single,
                Some(label),
            )
            .map(|_| ());
        }
    }
    if let Some(result) = crate::commands::session::fullscreen(app) {
        return result;
    }
    let (_, window) =
        active_window(app).ok_or("Open Luma or an app before using PC fullscreen.")?;
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
    if ![
        "Home", "Search", "Watch", "Sports", "Library", "Music", "Apps", "Settings", "Details",
    ]
    .contains(&view.as_str())
    {
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
        frontend_receives, should_wake, show_main, spike_route, validate_appearance, Appearance,
        NavigationCommand, SoundPreset, SpikeRoute, ThemeOption, MAX_THEMES,
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
            themes: vec![theme("violet-night", "Violet Night"), theme("mint", "Mint")],
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
        appearance
            .tokens
            .insert("--accent".into(), "#12345g".into());
        assert!(validate_appearance(&appearance).is_err());
    }

    #[test]
    fn appearance_validation_rejects_too_many_themes() {
        let mut appearance = valid_appearance();
        appearance.themes = (0..=MAX_THEMES)
            .map(|index| theme(&format!("theme-{index}"), "Theme"))
            .collect();
        assert!(validate_appearance(&appearance).is_err());
        appearance.themes.truncate(MAX_THEMES);
        assert!(validate_appearance(&appearance).is_ok());
    }

    #[test]
    fn spike_input_closes_the_spike_before_the_launcher_back_stack() {
        assert_eq!(spike_route(NavigationCommand::Back), SpikeRoute::Close);
        assert_eq!(spike_route(NavigationCommand::Home), SpikeRoute::Dismiss);
        for command in [
            NavigationCommand::Up,
            NavigationCommand::Down,
            NavigationCommand::Left,
            NavigationCommand::Right,
            NavigationCommand::Activate,
        ] {
            assert_eq!(spike_route(command), SpikeRoute::Forward);
        }
    }

    #[test]
    fn ordinary_settings_are_available_before_a_launcher_connects() {
        let settings = super::setting_catalog(&crate::customization::document::Config::default());
        let keys: Vec<&str> = settings.iter().map(|choice| choice.key.as_str()).collect();
        assert!(keys.contains(&"theme"));
        assert!(keys.contains(&"density"));
        assert!(keys.contains(&"appTileShape"));
        assert!(keys.contains(&"textSize"));
        assert!(keys.contains(&"reducedMotion"));
        assert!(keys.contains(&"interfaceSounds"));
        assert!(keys.contains(&"soundLevel"));
        assert!(keys.contains(&"homeColumns"));
        assert_eq!(
            settings
                .iter()
                .find(|choice| choice.key == "reducedMotion")
                .map(|choice| choice.kind.as_str()),
            Some("toggle")
        );
        assert!(settings
            .iter()
            .all(|choice| !choice.section.is_empty() && !choice.options.is_empty()));
    }

    #[test]
    fn ordinary_settings_values_come_from_the_native_configuration() {
        let mut config = crate::customization::document::Config::default();
        config.theme = "onyx".into();
        config.density = "compact".into();
        config.app_tile_shape = "square".into();
        config.text_size = "larger".into();
        config.reduced_motion = true;
        config.interface_sounds = "roku".into();
        config.sound_level = "soft".into();
        config.home_columns = 2;
        let settings = super::setting_catalog(&config);
        let value = |key: &str| {
            settings
                .iter()
                .find(|choice| choice.key == key)
                .map(|choice| choice.value.as_str())
        };
        assert_eq!(value("theme"), Some("onyx"));
        assert_eq!(value("density"), Some("compact"));
        assert_eq!(value("appTileShape"), Some("square"));
        assert_eq!(value("textSize"), Some("larger"));
        assert_eq!(value("reducedMotion"), Some("true"));
        assert_eq!(value("interfaceSounds"), Some("roku"));
        assert_eq!(value("soundLevel"), Some("soft"));
        assert_eq!(value("homeColumns"), Some("2"));
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
        assert_eq!(
            order
                .iter()
                .filter(|name| name.as_str() == "Custom")
                .count(),
            1
        );
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
            music: None,
        };
        let value = serde_json::to_value(state).unwrap();
        assert_eq!(value["window"], "YouTube");
        assert_eq!(value["windowFullscreen"], true);
        assert_eq!(value["sound"], "real");
    }

    #[test]
    fn music_state_is_compact_and_omits_nothing_the_remotes_read() {
        let music = super::MusicState {
            playing: true,
            title: Some("Song".into()),
            artist: Some("Artist".into()),
            has_artwork: true,
            position_ms: Some(1000),
            duration_ms: Some(200_000),
            updated_at: 42,
            can_toggle: true,
            can_skip_next: true,
            can_skip_previous: false,
        };
        let value = serde_json::to_value(&music).unwrap();
        assert_eq!(value["playing"], true);
        assert_eq!(value["title"], "Song");
        assert_eq!(value["hasArtwork"], true);
        assert_eq!(value["canSkipPrevious"], false);
        assert!(value.get("artworkUrl").is_none());
        assert!(serde_json::to_vec(&music).unwrap().len() < 512);
    }

    #[test]
    fn remote_panel_actions_map_only_to_music_commands() {
        use crate::customization::plugin::RemoteAction;

        assert_eq!(
            super::playback_command(RemoteAction::PlaybackToggle),
            "toggle"
        );
        assert_eq!(super::playback_command(RemoteAction::PlaybackNext), "next");
        assert_eq!(
            super::playback_command(RemoteAction::PlaybackPrevious),
            "previous"
        );
    }

    #[test]
    fn remote_panel_action_errors_keep_the_remote_contract() {
        let denied = super::action_response(
            false,
            Some("denied"),
            Some("That plugin did not declare this remote action.".into()),
            None,
        );
        assert_eq!(denied["accepted"], false);
        assert_eq!(denied["status"], "failure");
        assert_eq!(denied["code"], "denied");
        assert_eq!(
            denied["error"],
            "That plugin did not declare this remote action."
        );
        assert!(denied.get("music").is_none());

        let pending = super::action_response(
            false,
            Some("plugin_busy"),
            Some("That plugin is already starting a command.".into()),
            None,
        );
        assert_eq!(pending["accepted"], false);
        assert_eq!(pending["status"], "pending");
        assert_eq!(pending["code"], "plugin_busy");

        let accepted = super::action_response(
            true,
            None,
            None,
            Some(super::MusicState {
                playing: true,
                title: Some("Song".into()),
                ..Default::default()
            }),
        );
        assert_eq!(accepted["accepted"], true);
        assert_eq!(accepted["status"], "success");
        assert_eq!(accepted["music"]["title"], "Song");
        assert!(accepted.get("code").is_none());
        assert!(accepted.get("error").is_none());
    }

    #[test]
    fn remote_panels_use_the_bounded_wire_shape() {
        use crate::customization::plugin::{RemoteAction, RemotePanel};

        let value = super::panels_response(vec![RemotePanel {
            id: "com.example.lounge".into(),
            name: "Lounge".into(),
            title: "Lounge controls".into(),
            actions: vec![RemoteAction::PlaybackToggle, RemoteAction::PlaybackNext],
            version: None,
            commands: Vec::new(),
        }]);
        let panel = &value["panels"][0];
        assert_eq!(panel["id"], "com.example.lounge");
        assert_eq!(panel["name"], "Lounge");
        assert_eq!(panel["title"], "Lounge controls");
        assert_eq!(panel["actions"][0], "playback.toggle");
        assert_eq!(panel["actions"][1], "playback.next");
        assert_eq!(panel.as_object().unwrap().len(), 4);
        assert!(panel.get("commands").is_none());
        assert!(panel.get("permissions").is_none());
        assert!(panel.get("entry").is_none());
    }

    #[test]
    fn command_panels_expose_only_host_rendered_labels() {
        use crate::customization::plugin::{RemotePanel, RemotePanelCommand};

        let value = super::panels_response(vec![RemotePanel {
            id: "com.example.weather".into(),
            name: "Weather".into(),
            title: "Weather controls".into(),
            actions: Vec::new(),
            version: Some("1.0.0".into()),
            commands: vec![RemotePanelCommand {
                id: "openForecast".into(),
                label: "Refresh forecast".into(),
            }],
        }]);
        let panel = &value["panels"][0];
        assert_eq!(panel["actions"].as_array().unwrap().len(), 0);
        assert_eq!(panel["version"], "1.0.0");
        assert_eq!(panel["commands"][0]["id"], "openForecast");
        assert_eq!(panel["commands"][0]["label"], "Refresh forecast");
        assert!(panel["commands"][0].get("entry").is_none());
        assert!(panel["commands"][0].get("html").is_none());
    }

    #[test]
    fn home_brings_back_a_hidden_luma_but_not_a_handled_app() {
        assert!(show_main(false, false, NavigationCommand::Home));
        assert!(!show_main(true, false, NavigationCommand::Home));
        assert!(!show_main(false, true, NavigationCommand::Home));
        assert!(!show_main(false, false, NavigationCommand::Left));
    }
}
