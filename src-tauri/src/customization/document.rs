use serde::{Deserialize, Serialize};

const THEMES: [&str; 6] = ["violet-night", "sapphire", "mint", "ember", "graphite", "onyx"];
const DENSITIES: [&str; 2] = ["comfortable", "compact"];
const TILE_SHAPES: [&str; 2] = ["wide", "square"];
const COMPONENTS: [&str; 7] = ["hero", "continue", "apps", "popular", "featured", "recent", "recentApps"];
const APP_IDS: [&str; 12] = [
    "YouTube",
    "Spotify",
    "Stremio",
    "Steam",
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
const PLACEMENTS: [&str; 4] = ["bar", "top-right", "bottom-right", "bottom-left"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub revision: u64,
    pub config: Config,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub theme: String,
    pub density: String,
    pub app_tile_shape: String,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Widgets {
    pub spotify: Widget,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Widget {
    pub enabled: bool,
    pub placement: String,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "violet-night".into(),
            density: "comfortable".into(),
            app_tile_shape: "wide".into(),
            home: COMPONENTS
                .iter()
                .map(|component| HomeRow {
                    id: (*component).into(),
                    component: (*component).into(),
                    visible: true,
                })
                .collect(),
            widgets: Widgets {
                spotify: Widget { enabled: true, placement: "bar".into() },
            },
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
        return Err(format!("Unknown app tile shape: {}.", config.app_tile_shape));
    }
    if !PLACEMENTS.contains(&config.widgets.spotify.placement.as_str()) {
        return Err(format!(
            "Unknown Spotify placement: {}.",
            config.widgets.spotify.placement
        ));
    }
    validate_home(&config.home)?;
    validate_order(&config.order)
}

fn validate_home(home: &[HomeRow]) -> Result<(), String> {
    let mut ids: Vec<&str> = Vec::with_capacity(home.len());
    let mut components: Vec<&str> = Vec::with_capacity(home.len());
    for row in home {
        validate_home_id(&row.id)?;
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
    match COMPONENTS.iter().find(|component| !components.contains(component)) {
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
    if !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
        return Err(format!("Invalid home row id: {id}."));
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

pub fn migrate_legacy(raw: &str) -> Result<Document, String> {
    let legacy: LegacyDocument = serde_json::from_str(raw)
        .map_err(|error| format!("Invalid legacy customization document: {error}"))?;
    if legacy.version != 1 {
        return Err(format!("Unsupported legacy customization version: {}.", legacy.version));
    }
    let config = Config {
        theme: legacy.config.theme,
        density: legacy.config.density,
        app_tile_shape: legacy.config.app_tile_shape,
        home: legacy
            .config
            .home
            .into_iter()
            .map(|row| HomeRow {
                id: row.id.clone(),
                component: row.id,
                visible: row.visible,
            })
            .collect(),
        widgets: legacy.config.widgets,
        order: legacy.config.order,
    };
    validate(&config)?;
    Ok(Document { version: 2, revision: legacy.revision, config })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn reorder<T>(items: &mut Vec<T>, from: usize, to: usize) {
        let item = items.remove(from);
        items.insert(to, item);
    }

    fn legacy_document(config: &Config, revision: u64) -> Value {
        let mut config = serde_json::to_value(config).unwrap();
        for row in config["home"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("component");
        }
        json!({ "version": 1, "revision": revision, "config": config })
    }

    #[test]
    fn default_config_is_valid() {
        assert_eq!(validate(&Config::default()), Ok(()));
    }

    #[test]
    fn default_document_round_trips_as_camel_case() {
        let document = Document { version: 2, revision: 7, config: Config::default() };
        let json = serde_json::to_string(&document).unwrap();
        assert!(json.contains("\"appTileShape\":\"wide\""));
        assert!(json.contains("\"home\":[{\"id\":\"hero\",\"component\":\"hero\",\"visible\":true}"));
        assert!(!json.contains("\"type\""));
        assert_eq!(serde_json::from_str::<Document>(&json).unwrap(), document);
    }

    #[test]
    fn reordered_configuration_is_valid() {
        let mut config = Config::default();
        config.home.reverse();
        config.home[0].visible = false;
        reorder(&mut config.order, 9, 1);
        config.order.swap(0, 1);
        config.theme = "sapphire".into();
        config.density = "compact".into();
        config.app_tile_shape = "square".into();
        config.widgets.spotify.placement = "bottom-left".into();
        assert_eq!(config.order.last().map(String::as_str), Some(CUSTOM_APP));
        assert_eq!(validate(&config), Ok(()));
    }

    #[test]
    fn renamed_instance_keeps_its_component() {
        let mut config = Config::default();
        config.home[0].id = "hero-main_2".into();
        assert_eq!(validate(&config), Ok(()));
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("{\"id\":\"hero-main_2\",\"component\":\"hero\",\"visible\":true}"));
    }

    #[test]
    fn duplicate_component_is_rejected() {
        let mut config = Config::default();
        config.home[6] = HomeRow {
            id: "hero-alt".into(),
            component: "hero".into(),
            visible: true,
        };
        assert_eq!(validate(&config), Err("Duplicate home component: hero.".into()));
    }

    #[test]
    fn duplicate_instance_id_is_rejected() {
        let mut config = Config::default();
        config.home[6] = HomeRow {
            id: "hero".into(),
            component: "recentApps".into(),
            visible: true,
        };
        assert_eq!(validate(&config), Err("Duplicate home row id: hero.".into()));
    }

    #[test]
    fn invalid_instance_ids_are_rejected() {
        let mut config = Config::default();
        config.home[0].id.clear();
        assert_eq!(validate(&config), Err("Home row id cannot be empty.".into()));

        let mut config = Config::default();
        config.home[0].id = "a".repeat(65);
        assert_eq!(
            validate(&config),
            Err("Home row id must be at most 64 characters.".into())
        );

        let mut config = Config::default();
        config.home[0].id = "hero.primary".into();
        assert_eq!(validate(&config), Err("Invalid home row id: hero.primary.".into()));
    }

    #[test]
    fn missing_component_is_rejected() {
        let mut config = Config::default();
        config.home.retain(|row| row.component != "popular");
        assert_eq!(validate(&config), Err("Missing home component: popular.".into()));
    }

    #[test]
    fn unknown_component_is_rejected() {
        let mut config = Config::default();
        config.home[3].component = "trending".into();
        assert_eq!(validate(&config), Err("Unknown home component: trending.".into()));
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
        reorder(&mut config.order, 11, 0);
        assert_eq!(validate(&config), Err("Custom must be the last app.".into()));
    }

    #[test]
    fn unknown_choices_are_rejected() {
        let mut config = Config::default();
        config.theme = "aurora".into();
        assert_eq!(validate(&config), Err("Unknown theme: aurora.".into()));

        let mut config = Config::default();
        config.density = "spacious".into();
        assert_eq!(validate(&config), Err("Unknown density: spacious.".into()));

        let mut config = Config::default();
        config.app_tile_shape = "tall".into();
        assert_eq!(validate(&config), Err("Unknown app tile shape: tall.".into()));

        let mut config = Config::default();
        config.widgets.spotify.placement = "center".into();
        assert_eq!(validate(&config), Err("Unknown Spotify placement: center.".into()));
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
        config.widgets.spotify = Widget { enabled: false, placement: "bottom-right".into() };

        let raw = legacy_document(&config, 12).to_string();
        assert_eq!(
            migrate_legacy(&raw).unwrap(),
            Document { version: 2, revision: 12, config }
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
        document["config"]["theme"] = json!("aurora");
        assert_eq!(
            migrate_legacy(&document.to_string()).unwrap_err(),
            "Unknown theme: aurora."
        );

        let mut document = legacy_document(&Config::default(), 3);
        document["config"]["home"].as_array_mut().unwrap().pop();
        assert_eq!(
            migrate_legacy(&document.to_string()).unwrap_err(),
            "Missing home component: recentApps."
        );
    }
}
