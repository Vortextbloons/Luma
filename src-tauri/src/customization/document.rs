use serde::{Deserialize, Serialize};

const THEMES: [&str; 6] = ["violet-night", "sapphire", "mint", "ember", "graphite", "onyx"];
const DENSITIES: [&str; 2] = ["comfortable", "compact"];
const TILE_SHAPES: [&str; 2] = ["wide", "square"];
const ROW_IDS: [&str; 7] = ["hero", "continue", "apps", "popular", "featured", "recent", "recentApps"];
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

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "violet-night".into(),
            density: "comfortable".into(),
            app_tile_shape: "wide".into(),
            home: ROW_IDS
                .iter()
                .map(|id| HomeRow { id: (*id).into(), visible: true })
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
    let mut seen: Vec<&str> = Vec::with_capacity(home.len());
    for row in home {
        if !ROW_IDS.contains(&row.id.as_str()) {
            return Err(format!("Unknown home row: {}.", row.id));
        }
        if seen.contains(&row.id.as_str()) {
            return Err(format!("Duplicate home row: {}.", row.id));
        }
        seen.push(row.id.as_str());
    }
    match ROW_IDS.iter().find(|id| !seen.contains(id)) {
        Some(id) => Err(format!("Missing home row: {id}.")),
        None => Ok(()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reorder<T>(items: &mut Vec<T>, from: usize, to: usize) {
        let item = items.remove(from);
        items.insert(to, item);
    }

    #[test]
    fn default_config_is_valid() {
        assert_eq!(validate(&Config::default()), Ok(()));
    }

    #[test]
    fn default_document_round_trips_as_camel_case() {
        let document = Document { version: 1, revision: 7, config: Config::default() };
        let json = serde_json::to_string(&document).unwrap();
        assert!(json.contains("\"appTileShape\":\"wide\""));
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
    fn duplicate_home_row_is_rejected() {
        let mut config = Config::default();
        config.home[6] = HomeRow { id: "hero".into(), visible: true };
        assert_eq!(validate(&config), Err("Duplicate home row: hero.".into()));
    }

    #[test]
    fn missing_home_row_is_rejected() {
        let mut config = Config::default();
        config.home.retain(|row| row.id != "popular");
        assert_eq!(validate(&config), Err("Missing home row: popular.".into()));
    }

    #[test]
    fn unknown_home_row_is_rejected() {
        let mut config = Config::default();
        config.home[3].id = "trending".into();
        assert_eq!(validate(&config), Err("Unknown home row: trending.".into()));
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
}
