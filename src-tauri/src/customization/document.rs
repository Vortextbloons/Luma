use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const THEMES: [&str; 9] = [
    "violet-night",
    "sapphire",
    "mint",
    "ember",
    "graphite",
    "onyx",
    "aurora",
    "synthwave",
    "biolume",
];
const DENSITIES: [&str; 2] = ["comfortable", "compact"];
const TILE_SHAPES: [&str; 2] = ["wide", "square"];
const TEXT_SIZES: [&str; 3] = ["standard", "large", "larger"];
const SOUND_PRESETS: [&str; 4] = ["real", "google", "roku", "off"];
const SOUND_LEVELS: [&str; 3] = ["soft", "standard", "loud"];
const COMPONENTS: [&str; 7] = [
    "hero",
    "continue",
    "apps",
    "popular",
    "featured",
    "recent",
    "recentApps",
];
const APP_IDS: [&str; 16] = [
    "YouTube",
    "Spotify",
    "Stremio",
    "Steam",
    "NBA",
    "NFL",
    "MLB",
    "NHL",
    "Netflix",
    "Disney+",
    "Prime Video",
    "Apple TV+",
    "Hulu",
    "Max",
    "Peacock",
    "Custom",
];
const CUSTOM_APP: &str = "Custom";
const PLACEMENTS: [&str; 9] = [
    "bar",
    "top-left",
    "top-center",
    "top-right",
    "bottom-left",
    "bottom-center",
    "bottom-right",
    "rail-left",
    "rail-right",
];
const WIDGET_SIZES: [&str; 3] = ["small", "medium", "large"];
const WIDGET_PRESENCES: [&str; 3] = ["home", "music", "idle"];
const WIDGET_VISIBILITIES: [&str; 3] = ["always", "available", "playing"];
const MAX_WIDGETS: usize = 32;
const MAX_WIDGET_SETTINGS: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub revision: u64,
    pub config: Config,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub theme: String,
    pub density: String,
    pub app_tile_shape: String,
    #[serde(default = "default_text_size")]
    pub text_size: String,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default = "default_interface_sounds")]
    pub interface_sounds: String,
    #[serde(default = "default_sound_level")]
    pub sound_level: String,
    pub home_columns: u8,
    pub home: Vec<HomeRow>,
    pub widgets: Widgets,
    pub order: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HomeRow {
    pub id: String,
    pub component: String,
    pub visible: bool,
    pub span: u8,
}

fn default_home_columns() -> u8 {
    1
}

fn default_span() -> u8 {
    2
}

fn default_text_size() -> String {
    "standard".into()
}

fn default_interface_sounds() -> String {
    "real".into()
}

fn default_sound_level() -> String {
    "standard".into()
}

pub type Widgets = BTreeMap<String, Widget>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Widget {
    #[serde(rename = "type", default = "default_widget_type")]
    pub kind: String,
    pub enabled: bool,
    pub placement: String,
    #[serde(default = "default_widget_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub position: Option<WidgetPosition>,
    #[serde(default = "default_widget_size")]
    pub size: String,
    #[serde(default = "default_widget_variant")]
    pub variant: String,
    #[serde(default = "default_widget_presence")]
    pub presence: Vec<String>,
    #[serde(default)]
    pub order: u32,
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
}

fn default_widget_type() -> String {
    "spotify".into()
}

fn default_widget_visibility() -> String {
    "available".into()
}

fn default_widget_size() -> String {
    "medium".into()
}

fn default_widget_variant() -> String {
    "glass".into()
}

fn default_widget_presence() -> Vec<String> {
    vec!["home".into(), "idle".into()]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WidgetPosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyDocument {
    version: u32,
    revision: u64,
    config: LegacyConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyConfig {
    theme: String,
    density: String,
    app_tile_shape: String,
    home: Vec<LegacyHomeRow>,
    widgets: Widgets,
    order: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyHomeRow {
    id: String,
    visible: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct V2Document {
    version: u32,
    revision: u64,
    config: V2Config,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct V2Config {
    theme: String,
    density: String,
    app_tile_shape: String,
    home: Vec<V2HomeRow>,
    widgets: Widgets,
    order: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct V2HomeRow {
    id: String,
    component: String,
    visible: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "violet-night".into(),
            density: "comfortable".into(),
            app_tile_shape: "wide".into(),
            text_size: default_text_size(),
            reduced_motion: false,
            interface_sounds: default_interface_sounds(),
            sound_level: default_sound_level(),
            home_columns: default_home_columns(),
            home: COMPONENTS
                .iter()
                .map(|component| HomeRow {
                    id: (*component).into(),
                    component: (*component).into(),
                    visible: true,
                    span: default_span(),
                })
                .collect(),
            widgets: BTreeMap::from([(
                "spotify".into(),
                Widget {
                    kind: "spotify".into(),
                    enabled: true,
                    placement: "bar".into(),
                    visibility: default_widget_visibility(),
                    position: None,
                    size: default_widget_size(),
                    variant: default_widget_variant(),
                    presence: default_widget_presence(),
                    order: 0,
                    settings: BTreeMap::new(),
                },
            )]),
            order: APP_IDS.iter().map(|id| (*id).into()).collect(),
        }
    }
}

pub fn validate(config: &Config) -> Result<(), String> {
    if !THEMES.contains(&config.theme.as_str()) {
        return Err(format!("Unknown theme: {}.", config.theme));
    }
    if !DENSITIES.contains(&config.density.as_str()) {
        return Err(format!("Unknown density: {}.", config.density));
    }
    if !TILE_SHAPES.contains(&config.app_tile_shape.as_str()) {
        return Err(format!(
            "Unknown app tile shape: {}.",
            config.app_tile_shape
        ));
    }
    if !TEXT_SIZES.contains(&config.text_size.as_str()) {
        return Err(format!("Unknown text size: {}.", config.text_size));
    }
    if !SOUND_PRESETS.contains(&config.interface_sounds.as_str()) {
        return Err(format!(
            "Unknown interface sound set: {}.",
            config.interface_sounds
        ));
    }
    if !SOUND_LEVELS.contains(&config.sound_level.as_str()) {
        return Err(format!("Unknown sound level: {}.", config.sound_level));
    }
    if !matches!(config.home_columns, 1 | 2) {
        return Err(format!(
            "Home columns must be 1 or 2, got {}.",
            config.home_columns
        ));
    }
    if config.widgets.len() > MAX_WIDGETS {
        return Err(format!(
            "At most {MAX_WIDGETS} widgets are supported, got {}.",
            config.widgets.len()
        ));
    }
    for (id, widget) in &config.widgets {
        validate_widget_id(id)?;
        validate_widget_type(&widget.kind)?;
        if !PLACEMENTS.contains(&widget.placement.as_str()) {
            return Err(format!(
                "Unknown widget placement for {id}: {}.",
                widget.placement
            ));
        }
        if !WIDGET_VISIBILITIES.contains(&widget.visibility.as_str()) {
            return Err(format!("Unknown widget visibility for {id}."));
        }
        if !WIDGET_SIZES.contains(&widget.size.as_str()) {
            return Err(format!("Unknown widget size for {id}: {}.", widget.size));
        }
        if widget.variant.is_empty()
            || widget.variant.len() > 32
            || !widget
                .variant
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(format!("Invalid widget variant for {id}."));
        }
        if widget.presence.is_empty() {
            return Err(format!("Widget {id} needs at least one presence context."));
        }
        let mut presence = Vec::with_capacity(widget.presence.len());
        for value in &widget.presence {
            if !WIDGET_PRESENCES.contains(&value.as_str()) {
                return Err(format!("Unknown widget presence for {id}: {value}."));
            }
            if presence.contains(&value.as_str()) {
                return Err(format!("Duplicate widget presence for {id}: {value}."));
            }
            presence.push(value.as_str());
        }
        if widget
            .position
            .as_ref()
            .is_some_and(|position| position.x > 1000 || position.y > 1000)
        {
            return Err("Widget position must be between 0 and 1000.".into());
        }
        if widget.order > 1000 {
            return Err(format!("Widget order for {id} must be between 0 and 1000."));
        }
        if widget.settings.len() > MAX_WIDGET_SETTINGS {
            return Err(format!(
                "Widget {id} has too many settings; at most {MAX_WIDGET_SETTINGS} are supported."
            ));
        }
    }
    validate_home(&config.home)?;
    validate_order(&config.order)
}

fn validate_home(home: &[HomeRow]) -> Result<(), String> {
    let mut ids: Vec<&str> = Vec::with_capacity(home.len());
    let mut components: Vec<&str> = Vec::with_capacity(home.len());
    for row in home {
        validate_home_id(&row.id)?;
        if !matches!(row.span, 1 | 2) {
            return Err(format!(
                "Home row {} has an invalid span: {}.",
                row.id, row.span
            ));
        }
        if !COMPONENTS.contains(&row.component.as_str()) {
            return Err(format!("Unknown home component: {}.", row.component));
        }
        if ids.contains(&row.id.as_str()) {
            return Err(format!("Duplicate home row id: {}.", row.id));
        }
        ids.push(&row.id);
        if components.contains(&row.component.as_str()) {
            return Err(format!("Duplicate home component: {}.", row.component));
        }
        components.push(&row.component);
    }
    match COMPONENTS
        .iter()
        .find(|component| !components.contains(component))
    {
        Some(component) => Err(format!("Missing home component: {component}.")),
        None => Ok(()),
    }
}

fn validate_home_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Home row id cannot be empty.".into());
    }
    if id.len() > 64 {
        return Err("Home row id must be at most 64 characters.".into());
    }
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(format!("Invalid home row id: {id}."));
    }
    Ok(())
}

fn validate_widget_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Widget id cannot be empty.".into());
    }
    if id.len() > 64 {
        return Err("Widget id must be at most 64 characters.".into());
    }
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(format!("Invalid widget id: {id}."));
    }
    Ok(())
}

fn validate_widget_type(kind: &str) -> Result<(), String> {
    if kind.is_empty() {
        return Err("Widget type cannot be empty.".into());
    }
    if kind.len() > 64 {
        return Err("Widget type must be at most 64 characters.".into());
    }
    if !kind
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.')
    {
        return Err(format!("Invalid widget type: {kind}."));
    }
    Ok(())
}

fn validate_order(order: &[String]) -> Result<(), String> {
    let mut seen: Vec<&str> = Vec::with_capacity(order.len());
    for app in order {
        if !APP_IDS.contains(&app.as_str()) {
            return Err(format!("Unknown app: {app}."));
        }
        if seen.contains(&app.as_str()) {
            return Err(format!("Duplicate app: {app}."));
        }
        seen.push(app.as_str());
    }
    if let Some(app) = APP_IDS.iter().find(|id| !seen.contains(id)) {
        return Err(format!("Missing app: {app}."));
    }
    if order.last().map(String::as_str) != Some(CUSTOM_APP) {
        return Err(format!("{CUSTOM_APP} must be the last app."));
    }
    Ok(())
}

pub fn migrate_app_order(order: &mut Vec<String>) {
    let leagues = ["NBA", "NFL", "MLB", "NHL"];
    let mut previous: Vec<String> = APP_IDS
        .iter()
        .filter(|id| !leagues.contains(id) && **id != CUSTOM_APP)
        .map(|id| (*id).into())
        .collect();
    previous.extend(leagues.iter().map(|id| (*id).to_string()));
    previous.push(CUSTOM_APP.into());
    if *order == previous {
        *order = APP_IDS.iter().map(|id| (*id).into()).collect();
        return;
    }
    let old: Vec<&str> = APP_IDS
        .iter()
        .copied()
        .filter(|id| !leagues.contains(id))
        .collect();
    if order.iter().map(String::as_str).eq(old.iter().copied()) {
        *order = APP_IDS.iter().map(|id| (*id).into()).collect();
        return;
    }
    if order.len() == old.len()
        && old
            .iter()
            .all(|id| order.iter().filter(|app| app.as_str() == *id).count() == 1)
        && order.last().map(String::as_str) == Some(CUSTOM_APP)
    {
        let index = order.len() - 1;
        order.splice(index..index, leagues.into_iter().map(String::from));
    }
}

pub fn migrate_legacy(raw: &str) -> Result<Document, String> {
    let legacy: LegacyDocument = serde_json::from_str(raw)
        .map_err(|error| format!("Invalid legacy customization document: {error}"))?;
    if legacy.version != 1 {
        return Err(format!(
            "Unsupported legacy customization version: {}.",
            legacy.version
        ));
    }
    let mut config = Config {
        theme: legacy.config.theme,
        density: legacy.config.density,
        app_tile_shape: legacy.config.app_tile_shape,
        text_size: default_text_size(),
        reduced_motion: false,
        interface_sounds: default_interface_sounds(),
        sound_level: default_sound_level(),
        home_columns: default_home_columns(),
        home: legacy
            .config
            .home
            .into_iter()
            .map(|row| HomeRow {
                id: row.id.clone(),
                component: row.id,
                visible: row.visible,
                span: default_span(),
            })
            .collect(),
        widgets: legacy.config.widgets,
        order: legacy.config.order,
    };
    migrate_app_order(&mut config.order);
    validate(&config)?;
    Ok(Document {
        version: 3,
        revision: legacy.revision,
        config,
    })
}

pub fn migrate_v2(raw: &str) -> Result<Document, String> {
    let v2: V2Document = serde_json::from_str(raw)
        .map_err(|error| format!("Invalid customization document: {error}"))?;
    if v2.version != 2 {
        return Err(format!(
            "Unsupported customization version: {}.",
            v2.version
        ));
    }
    let mut config = Config {
        theme: v2.config.theme,
        density: v2.config.density,
        app_tile_shape: v2.config.app_tile_shape,
        text_size: default_text_size(),
        reduced_motion: false,
        interface_sounds: default_interface_sounds(),
        sound_level: default_sound_level(),
        home_columns: default_home_columns(),
        home: v2
            .config
            .home
            .into_iter()
            .map(|row| HomeRow {
                id: row.id,
                component: row.component,
                visible: row.visible,
                span: default_span(),
            })
            .collect(),
        widgets: v2.config.widgets,
        order: v2.config.order,
    };
    migrate_app_order(&mut config.order);
    validate(&config)?;
    Ok(Document {
        version: 3,
        revision: v2.revision,
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn reorder<T>(items: &mut Vec<T>, from: usize, to: usize) {
        let item = items.remove(from);
        items.insert(to, item);
    }

    fn spotify(config: &mut Config) -> &mut Widget {
        config.widgets.get_mut("spotify").unwrap()
    }

    fn widget(enabled: bool, placement: &str) -> Widget {
        Widget {
            kind: "spotify".into(),
            enabled,
            placement: placement.into(),
            visibility: "available".into(),
            position: None,
            size: "medium".into(),
            variant: "glass".into(),
            presence: vec!["home".into(), "idle".into()],
            order: 0,
            settings: BTreeMap::new(),
        }
    }

    fn legacy_document(config: &Config, revision: u64) -> Value {
        let mut config = serde_json::to_value(config).unwrap();
        let config = config.as_object_mut().unwrap();
        for field in [
            "homeColumns",
            "textSize",
            "reducedMotion",
            "interfaceSounds",
            "soundLevel",
        ] {
            config.remove(field);
        }
        for row in config["home"].as_array_mut().unwrap() {
            let row = row.as_object_mut().unwrap();
            row.remove("component");
            row.remove("span");
        }
        json!({ "version": 1, "revision": revision, "config": config })
    }

    fn v2_document(config: &Config, revision: u64) -> Value {
        let mut config = serde_json::to_value(config).unwrap();
        let config = config.as_object_mut().unwrap();
        for field in [
            "homeColumns",
            "textSize",
            "reducedMotion",
            "interfaceSounds",
            "soundLevel",
        ] {
            config.remove(field);
        }
        for row in config["home"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("span");
        }
        json!({ "version": 2, "revision": revision, "config": config })
    }

    #[test]
    fn default_leagues_follow_steam_and_previous_defaults_migrate() {
        let expected = Config::default().order;
        let steam = expected.iter().position(|id| id == "Steam").unwrap();
        assert_eq!(
            &expected[steam + 1..steam + 5],
            &["NBA", "NFL", "MLB", "NHL"]
        );
        let mut previous = expected.clone();
        previous.retain(|id| !["NBA", "NFL", "MLB", "NHL"].contains(&id.as_str()));
        migrate_app_order(&mut previous);
        assert_eq!(previous, expected);
        previous.retain(|id| !["NBA", "NFL", "MLB", "NHL", "Custom"].contains(&id.as_str()));
        previous.extend(["NBA", "NFL", "MLB", "NHL", "Custom"].map(String::from));
        migrate_app_order(&mut previous);
        assert_eq!(previous, expected);
    }

    #[test]
    fn default_config_is_valid() {
        assert_eq!(validate(&Config::default()), Ok(()));
    }

    #[test]
    fn reading_and_sound_preferences_validate_and_default_for_older_documents() {
        let mut config = Config::default();
        config.text_size = "larger".into();
        config.reduced_motion = true;
        config.interface_sounds = "roku".into();
        config.sound_level = "soft".into();
        assert_eq!(validate(&config), Ok(()));

        let mut value = serde_json::to_value(&config).unwrap();
        value.as_object_mut().unwrap().remove("textSize");
        value.as_object_mut().unwrap().remove("reducedMotion");
        value.as_object_mut().unwrap().remove("interfaceSounds");
        value.as_object_mut().unwrap().remove("soundLevel");
        let document = json!({ "version": 3, "revision": 4, "config": value });
        let decoded: Document = serde_json::from_value(document).unwrap();
        assert_eq!(decoded.config.text_size, "standard");
        assert!(!decoded.config.reduced_motion);
        assert_eq!(decoded.config.interface_sounds, "real");
        assert_eq!(decoded.config.sound_level, "standard");
    }

    #[test]
    fn unknown_reading_and_sound_choices_are_rejected() {
        let mut config = Config::default();
        config.text_size = "huge".into();
        assert_eq!(validate(&config), Err("Unknown text size: huge.".into()));

        let mut config = Config::default();
        config.interface_sounds = "cinema".into();
        assert_eq!(
            validate(&config),
            Err("Unknown interface sound set: cinema.".into())
        );

        let mut config = Config::default();
        config.sound_level = "maximum".into();
        assert_eq!(
            validate(&config),
            Err("Unknown sound level: maximum.".into())
        );
    }

    #[test]
    fn default_document_round_trips_as_camel_case() {
        let document = Document {
            version: 3,
            revision: 7,
            config: Config::default(),
        };
        let json = serde_json::to_string(&document).unwrap();
        assert!(json.contains("\"appTileShape\":\"wide\""));
        assert!(json.contains("\"textSize\":\"standard\""));
        assert!(json.contains("\"reducedMotion\":false"));
        assert!(json.contains("\"interfaceSounds\":\"real\""));
        assert!(json.contains("\"soundLevel\":\"standard\""));
        assert!(json.contains("\"homeColumns\":1"));
        assert!(json.contains(
            "\"home\":[{\"id\":\"hero\",\"component\":\"hero\",\"visible\":true,\"span\":2}"
        ));
        assert!(json.contains("\"type\":\"spotify\""));
        assert_eq!(serde_json::from_str::<Document>(&json).unwrap(), document);
    }

    #[test]
    fn widget_positions_and_visibility_round_trip_and_validate() {
        let mut config = Config::default();
        spotify(&mut config).visibility = "playing".into();
        spotify(&mut config).position = Some(WidgetPosition { x: 125, y: 1000 });
        assert!(validate(&config).is_ok());
        let raw = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&raw).unwrap(), config);
        spotify(&mut config).position.as_mut().unwrap().x = 1001;
        assert!(validate(&config).is_err());
        spotify(&mut config).position = None;
        spotify(&mut config).visibility = "sometimes".into();
        assert!(validate(&config).is_err());
    }

    #[test]
    fn old_widget_settings_keep_the_available_visibility_default() {
        let mut raw = serde_json::to_value(Config::default()).unwrap();
        raw["widgets"]["spotify"] = serde_json::json!({ "enabled": true, "placement": "bar" });
        assert_eq!(
            serde_json::from_value::<Config>(raw).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn multiple_widget_instances_round_trip_and_validate() {
        let mut config = Config::default();
        config.widgets.insert(
            "clock".into(),
            Widget {
                kind: "clock".into(),
                enabled: true,
                placement: "top-right".into(),
                visibility: "always".into(),
                position: None,
                size: "small".into(),
                variant: "minimal".into(),
                presence: vec!["home".into(), "music".into(), "idle".into()],
                order: 1,
                settings: BTreeMap::from([("hour12".into(), Value::Bool(true))]),
            },
        );
        assert_eq!(validate(&config), Ok(()));
        let raw = serde_json::to_string(&config).unwrap();
        assert!(raw.contains("\"clock\":{\"type\":\"clock\""));
        assert_eq!(serde_json::from_str::<Config>(&raw).unwrap(), config);
    }

    #[test]
    fn invalid_widget_fields_are_rejected() {
        let mut config = Config::default();
        spotify(&mut config).size = "huge".into();
        assert_eq!(
            validate(&config),
            Err("Unknown widget size for spotify: huge.".into())
        );

        let mut config = Config::default();
        spotify(&mut config).presence = vec!["sometimes".into()];
        assert_eq!(
            validate(&config),
            Err("Unknown widget presence for spotify: sometimes.".into())
        );

        let mut config = Config::default();
        spotify(&mut config).variant = "not a variant".into();
        assert_eq!(
            validate(&config),
            Err("Invalid widget variant for spotify.".into())
        );

        let mut config = Config::default();
        config
            .widgets
            .insert("clock.primary".into(), widget(true, "top-right"));
        assert_eq!(
            validate(&config),
            Err("Invalid widget id: clock.primary.".into())
        );
    }

    #[test]
    fn widgets_can_be_removed_entirely() {
        let mut config = Config::default();
        config.widgets.clear();
        assert_eq!(validate(&config), Ok(()));
    }

    #[test]
    fn reordered_configuration_is_valid() {
        let mut config = Config::default();
        config.home.reverse();
        config.home[0].visible = false;
        config.home_columns = 2;
        config.home[1].span = 1;
        reorder(&mut config.order, 9, 1);
        config.order.swap(0, 1);
        config.theme = "sapphire".into();
        config.density = "compact".into();
        config.app_tile_shape = "square".into();
        spotify(&mut config).placement = "bottom-left".into();
        assert_eq!(config.order.last().map(String::as_str), Some(CUSTOM_APP));
        assert_eq!(validate(&config), Ok(()));
    }

    #[test]
    fn renamed_instance_keeps_its_component() {
        let mut config = Config::default();
        config.home[0].id = "hero-main_2".into();
        assert_eq!(validate(&config), Ok(()));
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(
            "{\"id\":\"hero-main_2\",\"component\":\"hero\",\"visible\":true,\"span\":2}"
        ));
    }

    #[test]
    fn duplicate_component_is_rejected() {
        let mut config = Config::default();
        config.home[6] = HomeRow {
            id: "hero-alt".into(),
            component: "hero".into(),
            visible: true,
            span: 2,
        };
        assert_eq!(
            validate(&config),
            Err("Duplicate home component: hero.".into())
        );
    }

    #[test]
    fn duplicate_instance_id_is_rejected() {
        let mut config = Config::default();
        config.home[6] = HomeRow {
            id: "hero".into(),
            component: "recentApps".into(),
            visible: true,
            span: 2,
        };
        assert_eq!(
            validate(&config),
            Err("Duplicate home row id: hero.".into())
        );
    }

    #[test]
    fn invalid_home_columns_and_spans_are_rejected() {
        let mut config = Config::default();
        config.home_columns = 0;
        assert_eq!(
            validate(&config),
            Err("Home columns must be 1 or 2, got 0.".into())
        );

        let mut config = Config::default();
        config.home_columns = 3;
        assert_eq!(
            validate(&config),
            Err("Home columns must be 1 or 2, got 3.".into())
        );

        let mut config = Config::default();
        config.home[0].span = 0;
        assert_eq!(
            validate(&config),
            Err("Home row hero has an invalid span: 0.".into())
        );

        let mut config = Config::default();
        config.home[2].id = "apps-secondary".into();
        config.home[2].span = 3;
        assert_eq!(
            validate(&config),
            Err("Home row apps-secondary has an invalid span: 3.".into())
        );
    }

    #[test]
    fn invalid_instance_ids_are_rejected() {
        let mut config = Config::default();
        config.home[0].id.clear();
        assert_eq!(
            validate(&config),
            Err("Home row id cannot be empty.".into())
        );

        let mut config = Config::default();
        config.home[0].id = "a".repeat(65);
        assert_eq!(
            validate(&config),
            Err("Home row id must be at most 64 characters.".into())
        );

        let mut config = Config::default();
        config.home[0].id = "hero.primary".into();
        assert_eq!(
            validate(&config),
            Err("Invalid home row id: hero.primary.".into())
        );
    }

    #[test]
    fn missing_component_is_rejected() {
        let mut config = Config::default();
        config.home.retain(|row| row.component != "popular");
        assert_eq!(
            validate(&config),
            Err("Missing home component: popular.".into())
        );
    }

    #[test]
    fn unknown_component_is_rejected() {
        let mut config = Config::default();
        config.home[3].component = "trending".into();
        assert_eq!(
            validate(&config),
            Err("Unknown home component: trending.".into())
        );
    }

    #[test]
    fn duplicate_app_is_rejected() {
        let mut config = Config::default();
        config.order[1] = "Hulu".into();
        assert_eq!(validate(&config), Err("Duplicate app: Hulu.".into()));
    }

    #[test]
    fn missing_app_is_rejected() {
        let mut config = Config::default();
        config.order.retain(|app| app != "Max");
        assert_eq!(validate(&config), Err("Missing app: Max.".into()));
    }

    #[test]
    fn custom_app_must_be_last() {
        let mut config = Config::default();
        let last = config.order.len() - 1;
        reorder(&mut config.order, last, 0);
        assert_eq!(
            validate(&config),
            Err("Custom must be the last app.".into())
        );
    }

    #[test]
    fn unknown_choices_are_rejected() {
        let mut config = Config::default();
        config.theme = "midnight".into();
        assert_eq!(validate(&config), Err("Unknown theme: midnight.".into()));

        let mut config = Config::default();
        config.density = "spacious".into();
        assert_eq!(validate(&config), Err("Unknown density: spacious.".into()));

        let mut config = Config::default();
        config.app_tile_shape = "tall".into();
        assert_eq!(
            validate(&config),
            Err("Unknown app tile shape: tall.".into())
        );

        let mut config = Config::default();
        spotify(&mut config).placement = "center".into();
        assert_eq!(
            validate(&config),
            Err("Unknown widget placement for spotify: center.".into())
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let config = serde_json::to_value(Config::default()).unwrap();
        let mut value = config.as_object().unwrap().clone();
        value.insert("accent".into(), "violet".into());
        assert!(serde_json::from_value::<Config>(serde_json::Value::Object(value)).is_err());
    }

    #[test]
    fn legacy_migration_preserves_configuration_and_revision() {
        let mut config = Config::default();
        config.home.reverse();
        config.home[0].visible = false;
        reorder(&mut config.order, 9, 1);
        config.order.swap(0, 1);
        config.theme = "ember".into();
        config.density = "compact".into();
        config.app_tile_shape = "square".into();
        config
            .widgets
            .insert("spotify".into(), widget(false, "bottom-right"));

        let raw = legacy_document(&config, 12).to_string();
        assert_eq!(
            migrate_legacy(&raw).unwrap(),
            Document {
                version: 3,
                revision: 12,
                config
            }
        );
    }

    #[test]
    fn v2_migration_preserves_configuration_and_revision_with_new_defaults() {
        let mut config = Config::default();
        config.home_columns = 2;
        config.home[2].span = 1;
        config.home.reverse();
        config.home[0].visible = false;
        reorder(&mut config.order, 9, 1);
        config.order.swap(0, 1);
        config.theme = "ember".into();
        config.density = "compact".into();
        config.app_tile_shape = "square".into();
        config
            .widgets
            .insert("spotify".into(), widget(false, "bottom-right"));

        let mut expected = config.clone();
        expected.home_columns = 1;
        for row in &mut expected.home {
            row.span = 2;
        }

        let raw = v2_document(&config, 21).to_string();
        assert_eq!(
            migrate_v2(&raw).unwrap(),
            Document {
                version: 3,
                revision: 21,
                config: expected
            }
        );
    }

    #[test]
    fn v2_migration_rejects_other_versions_and_unknown_fields() {
        let mut document = v2_document(&Config::default(), 3);
        document["version"] = json!(3);
        assert_eq!(
            migrate_v2(&document.to_string()).unwrap_err(),
            "Unsupported customization version: 3."
        );

        let mut document = v2_document(&Config::default(), 3);
        document["extra"] = json!(true);
        assert!(migrate_v2(&document.to_string()).is_err());

        let mut document = v2_document(&Config::default(), 3);
        document["config"]["homeColumns"] = json!(2);
        assert!(migrate_v2(&document.to_string()).is_err());

        let mut document = v2_document(&Config::default(), 3);
        document["config"]["accent"] = json!("violet");
        assert!(migrate_v2(&document.to_string()).is_err());

        let mut document = v2_document(&Config::default(), 3);
        document["config"]["home"][0]["span"] = json!(1);
        assert!(migrate_v2(&document.to_string()).is_err());

        let document = legacy_document(&Config::default(), 3);
        assert!(migrate_v2(&document.to_string()).is_err());
    }

    #[test]
    fn v2_migration_rejects_invalid_configuration() {
        let mut document = v2_document(&Config::default(), 3);
        document["config"]["theme"] = json!("midnight");
        assert_eq!(
            migrate_v2(&document.to_string()).unwrap_err(),
            "Unknown theme: midnight."
        );

        let mut document = v2_document(&Config::default(), 3);
        document["config"]["home"].as_array_mut().unwrap().pop();
        assert_eq!(
            migrate_v2(&document.to_string()).unwrap_err(),
            "Missing home component: recentApps."
        );
    }

    #[test]
    fn legacy_migration_rejects_unknown_fields() {
        let mut document = legacy_document(&Config::default(), 3);
        document["extra"] = json!(true);
        assert!(migrate_legacy(&document.to_string()).is_err());

        let mut document = legacy_document(&Config::default(), 3);
        document["config"]["home"][0]["component"] = json!("hero");
        assert!(migrate_legacy(&document.to_string()).is_err());

        let mut document = legacy_document(&Config::default(), 3);
        document["config"]["homeColumns"] = json!(2);
        assert!(migrate_legacy(&document.to_string()).is_err());

        let mut document = legacy_document(&Config::default(), 3);
        document["config"]["home"][0]["span"] = json!(1);
        assert!(migrate_legacy(&document.to_string()).is_err());
    }

    #[test]
    fn legacy_migration_rejects_other_versions_and_invalid_configuration() {
        let mut document = legacy_document(&Config::default(), 3);
        document["version"] = json!(2);
        assert_eq!(
            migrate_legacy(&document.to_string()).unwrap_err(),
            "Unsupported legacy customization version: 2."
        );

        let mut document = legacy_document(&Config::default(), 3);
        document["config"]["theme"] = json!("midnight");
        assert_eq!(
            migrate_legacy(&document.to_string()).unwrap_err(),
            "Unknown theme: midnight."
        );

        let mut document = legacy_document(&Config::default(), 3);
        document["config"]["home"].as_array_mut().unwrap().pop();
        assert_eq!(
            migrate_legacy(&document.to_string()).unwrap_err(),
            "Missing home component: recentApps."
        );
    }
}
