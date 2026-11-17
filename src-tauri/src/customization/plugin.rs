use super::{
    document::{self, Config, Document, HomeRow},
    Error, Store,
};
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tauri::{Emitter, Manager};

mod archive;

pub const API_VERSION: u32 = 1;
pub const EXECUTABLE_API_VERSION: u32 = 2;
pub const SCHEMA_RESOURCE: &str = "luma://plugins/schema";
pub const EXAMPLE_RESOURCE: &str = "luma://plugins/example";
const MAX_ARCHIVE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_FILES: usize = 256;
const MAX_ENTRIES: usize = 512;
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 200;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_SETTINGS: usize = 24;
const MAX_SETTING_OPTIONS: usize = 20;
const MAX_SOURCE_FILES: usize = 64;
const MAX_PATH_BYTES: usize = 120;
const MAX_MCP_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PERMISSIONS: usize = 8;
const MAX_NETWORK_ORIGINS: usize = 8;
const MAX_ORIGIN_BYTES: usize = 128;
const MAX_REMOTE_ACTIONS: usize = 3;
const MAX_REMOTE_TITLE: usize = 48;
const BUILTIN_THEMES: [&str; 6] = [
    "violet-night",
    "sapphire",
    "mint",
    "ember",
    "graphite",
    "onyx",
];
const DENSITIES: [&str; 2] = ["comfortable", "compact"];
const TILE_SHAPES: [&str; 2] = ["wide", "square"];
const TEXT_SIZES: [&str; 3] = ["standard", "large", "larger"];
const COMPONENTS: [&str; 7] = [
    "hero",
    "continue",
    "apps",
    "popular",
    "featured",
    "recent",
    "recentApps",
];
const PLACEMENTS: [&str; 5] = ["bar", "top-left", "top-right", "bottom-right", "bottom-left"];
pub const FIELDS: [&str; 9] = [
    "theme",
    "density",
    "appTileShape",
    "textSize",
    "reducedMotion",
    "homeColumns",
    "home",
    "widgets",
    "order",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub settings: Vec<SettingDefinition>,
    #[serde(default)]
    pub contributes: Contributions,
    #[serde(default)]
    pub runtime: Runtime,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub network: Option<NetworkGrants>,
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub source: Vec<String>,
    #[serde(default)]
    pub remote: Option<RemoteContribution>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "settings.read")]
    SettingsRead,
    #[serde(rename = "network.get")]
    NetworkGet,
    #[serde(rename = "playback.read")]
    PlaybackRead,
    #[serde(rename = "playback.control")]
    PlaybackControl,
}

pub const PERMISSIONS: [Permission; 4] = [
    Permission::SettingsRead,
    Permission::NetworkGet,
    Permission::PlaybackRead,
    Permission::PlaybackControl,
];

impl Permission {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::SettingsRead => "settings.read",
            Self::NetworkGet => "network.get",
            Self::PlaybackRead => "playback.read",
            Self::PlaybackControl => "playback.control",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        PERMISSIONS
            .iter()
            .copied()
            .find(|permission| permission.label() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    LauncherOpen,
    Startup,
    Always,
}

impl Trigger {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::LauncherOpen => "launcher-open",
            Self::Startup => "startup",
            Self::Always => "always",
        }
    }

    pub(crate) fn supports(self, kind: RuntimeKind) -> bool {
        match kind {
            RuntimeKind::IsolatedUi => matches!(self, Self::LauncherOpen),
            RuntimeKind::Background => true,
            RuntimeKind::Declarative | RuntimeKind::MainPage => false,
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        [Self::LauncherOpen, Self::Startup, Self::Always]
            .into_iter()
            .find(|trigger| trigger.label() == value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteContribution {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub actions: Vec<RemoteAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteAction {
    #[serde(rename = "playback.toggle")]
    PlaybackToggle,
    #[serde(rename = "playback.next")]
    PlaybackNext,
    #[serde(rename = "playback.previous")]
    PlaybackPrevious,
}

impl RemoteAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::PlaybackToggle => "playback.toggle",
            Self::PlaybackNext => "playback.next",
            Self::PlaybackPrevious => "playback.previous",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        [
            Self::PlaybackToggle,
            Self::PlaybackNext,
            Self::PlaybackPrevious,
        ]
        .into_iter()
        .find(|action| action.label() == value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub state: String,
    pub reason: Option<String>,
    pub restarts: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemotePanel {
    pub id: String,
    pub name: String,
    pub title: String,
    pub actions: Vec<RemoteAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkGrants {
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contributions {
    #[serde(default)]
    pub theme: Option<Bound<String>>,
    #[serde(default)]
    pub density: Option<Bound<String>>,
    #[serde(default)]
    pub app_tile_shape: Option<Bound<String>>,
    #[serde(default)]
    pub text_size: Option<Bound<String>>,
    #[serde(default)]
    pub reduced_motion: Option<Bound<bool>>,
    #[serde(default)]
    pub home_columns: Option<Bound<u8>>,
    #[serde(default)]
    pub home: Option<HomeContribution>,
    #[serde(default)]
    pub widgets: Option<WidgetContribution>,
    #[serde(default)]
    pub order: Option<Vec<String>>,
}

impl Contributions {
    fn fields(&self) -> BTreeSet<&'static str> {
        let mut fields = BTreeSet::new();
        if self.theme.is_some() {
            fields.insert("theme");
        }
        if self.density.is_some() {
            fields.insert("density");
        }
        if self.app_tile_shape.is_some() {
            fields.insert("appTileShape");
        }
        if self.text_size.is_some() {
            fields.insert("textSize");
        }
        if self.reduced_motion.is_some() {
            fields.insert("reducedMotion");
        }
        if self.home_columns.is_some() {
            fields.insert("homeColumns");
        }
        if self.home.as_ref().is_some_and(|home| {
            home.order.is_some()
                || home.rows.values().any(|row| {
                    row.visible.is_some() || row.span.is_some()
                })
        }) {
            fields.insert("home");
        }
        if self.widgets.as_ref().is_some_and(|widgets| {
            widgets.spotify_enabled.is_some() || widgets.spotify_placement.is_some()
        }) {
            fields.insert("widgets");
        }
        if self.order.is_some() {
            fields.insert("order");
        }
        fields
    }

    fn is_empty(&self) -> bool {
        self.fields().is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HomeContribution {
    #[serde(default)]
    pub order: Option<Vec<String>>,
    #[serde(default)]
    pub rows: BTreeMap<String, HomeRowContribution>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HomeRowContribution {
    #[serde(default)]
    pub visible: Option<Bound<bool>>,
    #[serde(default)]
    pub span: Option<Bound<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidgetContribution {
    #[serde(default)]
    pub spotify_enabled: Option<Bound<bool>>,
    #[serde(default)]
    pub spotify_placement: Option<Bound<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bound<T> {
    Setting { setting: String },
    Value(T),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Runtime {
    #[serde(default)]
    pub kind: RuntimeKind,
    #[serde(default)]
    pub entry: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeKind {
    #[default]
    Declarative,
    IsolatedUi,
    Background,
    MainPage,
}

impl RuntimeKind {
    fn is_declarative(self) -> bool {
        matches!(self, Self::Declarative)
    }

    pub(crate) fn is_executable(self) -> bool {
        matches!(self, Self::IsolatedUi | Self::Background)
    }

    pub(crate) fn has_surface(self) -> bool {
        matches!(self, Self::IsolatedUi)
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Declarative => "declarative",
            Self::IsolatedUi => "isolated-ui",
            Self::Background => "background",
            Self::MainPage => "main-page",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingDefinition {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub kind: SettingKind,
    pub default: Value,
    #[serde(default)]
    pub options: Vec<SettingOption>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub integer: bool,
    #[serde(default = "default_max_length")]
    pub max_length: usize,
    #[serde(default)]
    pub min_length: usize,
}

fn default_max_length() -> usize {
    200
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingKind {
    Boolean,
    Enum,
    Number,
    String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSetting {
    pub key: String,
    pub qualified_key: String,
    pub label: String,
    pub description: String,
    pub kind: SettingKind,
    pub value: Value,
    pub default: Value,
    pub valid: bool,
    pub options: Vec<SettingOption>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub integer: bool,
    pub max_length: usize,
    pub min_length: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub description: String,
    pub author: String,
    pub enabled: bool,
    pub runtime: String,
    pub executable: bool,
    pub execution_reason: Option<String>,
    pub permissions: Vec<String>,
    pub network: Vec<String>,
    pub granted_permissions: Vec<String>,
    pub granted_network: Vec<String>,
    pub trigger: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_status: Option<RuntimeStatus>,
    pub entry: Option<String>,
    pub surface_openable: bool,
    pub controls: Vec<String>,
    pub declared_controls: Vec<String>,
    pub detached_fields: Vec<String>,
    pub conflicts: Vec<String>,
    pub can_enable: bool,
    pub enable_error: Option<String>,
    pub disabled_reason: Option<String>,
    pub settings: Vec<PluginSetting>,
    pub source: String,
    pub installed_at: String,
    pub size_bytes: u64,
    pub content_hash: String,
    pub source_files: Vec<String>,
    pub assets: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginsState {
    pub plugins: Vec<PluginRecord>,
    pub effective: Config,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginSnapshot {
    #[serde(default)]
    pub plugins: Vec<PluginSnapshotEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginSnapshotEntry {
    pub id: String,
    pub version: String,
    pub enabled: bool,
    #[serde(default)]
    pub detached: Vec<String>,
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedWrite {
    pub base: Config,
    pub detach: Vec<Detach>,
    pub force: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Detach {
    pub plugin: String,
    pub field: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) enum PluginAction {
    Enable { plugin_id: String, reattach: bool },
    Settings { plugin_id: String, values: BTreeMap<String, Value> },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPreview {
    pub id: String,
    pub name: String,
    pub action: String,
}

#[derive(Clone)]
struct StoredPlugin {
    manifest: Manifest,
    enabled: bool,
    detached: BTreeSet<String>,
    disabled_reason: Option<String>,
    source: String,
    installed_at: String,
    size_bytes: u64,
    content_hash: String,
    settings: BTreeMap<String, Value>,
}

impl StoredPlugin {
    fn active_fields(&self) -> BTreeSet<&'static str> {
        let mut fields = self.manifest.contributes.fields();
        for field in &self.detached {
            fields.remove(field.as_str());
        }
        fields
    }

    fn declared_fields(&self) -> BTreeSet<&'static str> {
        self.manifest.contributes.fields()
    }

    fn execution_reason(&self) -> Option<String> {
        match self.manifest.runtime.kind {
            RuntimeKind::Declarative => None,
            RuntimeKind::IsolatedUi | RuntimeKind::Background => {
                (self.manifest.api_version != EXECUTABLE_API_VERSION).then(|| {
                    "This package targets an executable runtime with an older plugin API. Install a compatible version.".to_string()
                })
            }
            RuntimeKind::MainPage => Some(
                "This plugin declares the main-page runtime. Luma never runs third-party code inside the main window, so it cannot be enabled."
                    .to_string(),
            ),
        }
    }

    fn resolved_settings(&self) -> Vec<PluginSetting> {
        self.manifest
            .settings
            .iter()
            .map(|definition| {
                let stored = self.settings.get(&definition.key);
                let valid = stored.is_some_and(|value| validate_setting_value(definition, value).is_ok());
                let value = if valid {
                    stored.cloned().unwrap_or_else(|| definition.default.clone())
                } else {
                    definition.default.clone()
                };
                PluginSetting {
                    key: definition.key.clone(),
                    qualified_key: format!("{}.{}", self.manifest.id, definition.key),
                    label: definition.label.clone(),
                    description: definition.description.clone(),
                    kind: definition.kind,
                    value,
                    default: definition.default.clone(),
                    valid: stored.is_none() || valid,
                    options: definition.options.clone(),
                    min: definition.min,
                    max: definition.max,
                    integer: definition.integer,
                    max_length: definition.max_length,
                    min_length: definition.min_length,
                }
            })
            .collect()
    }

    fn setting_value(&self, definition: &SettingDefinition) -> Value {
        self.settings
            .get(&definition.key)
            .filter(|value| validate_setting_value(definition, value).is_ok())
            .cloned()
            .unwrap_or_else(|| definition.default.clone())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Grant {
    pub permissions: BTreeSet<Permission>,
    pub network: Vec<String>,
    pub trigger: Trigger,
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimePackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub kind: RuntimeKind,
    pub entry: String,
    pub permissions: BTreeSet<Permission>,
    pub network: Vec<String>,
    pub settings: BTreeMap<String, Value>,
    root: PathBuf,
}

impl RuntimePackage {
    pub(crate) fn allows(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    pub(crate) fn read(&self, path: &str) -> Result<Vec<u8>, Error> {
        let relative = if path.is_empty() {
            self.entry.as_str()
        } else {
            path
        };
        validate_relative_path(relative)?;
        let root = fs::canonicalize(&self.root).map_err(|error| Error::new("plugin_missing", error))?;
        let target = fs::canonicalize(
            root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)),
        )
        .map_err(|error| Error::new("plugin_missing", error))?;
        if !target.starts_with(&root) || !target.is_file() {
            return Err(Error::new(
                "plugin_missing",
                "That file is not part of the installed package.",
            ));
        }
        fs::read(target).map_err(|error| Error::new("plugin_missing", error))
    }
}

fn load_grant(connection: &Connection, plugin_id: &str) -> Result<Option<Grant>, Error> {
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT permissions, network, trigger FROM plugin_grants WHERE plugin_id = ?1",
            [plugin_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((permissions, network, trigger)) = row else {
        return Ok(None);
    };
    let permissions: Vec<String> = serde_json::from_str(&permissions)
        .map_err(|error| Error::new("plugin_storage", error))?;
    let network: Vec<String> =
        serde_json::from_str(&network).map_err(|error| Error::new("plugin_storage", error))?;
    Ok(Some(Grant {
        permissions: permissions
            .iter()
            .filter_map(|permission| Permission::parse(permission))
            .collect(),
        network,
        trigger: Trigger::parse(&trigger)
            .ok_or_else(|| Error::new("plugin_storage", "The stored runtime trigger is invalid."))?,
    }))
}

pub(crate) fn runtime_package(
    store: &Store,
    plugin_id: &str,
) -> Result<RuntimePackage, Error> {
    let connection = store.connection()?;
    let plugins = load_plugins(&connection)?;
    let (_, plugin) = plugins
        .iter()
        .find(|(id, _)| id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
    if !plugin.enabled {
        return Err(Error::new("plugin_disabled", "That plugin is disabled."));
    }
    if !plugin.manifest.runtime.kind.is_executable()
        || plugin.manifest.api_version != EXECUTABLE_API_VERSION
        || plugin.execution_reason().is_some()
    {
        return Err(Error::new(
            "plugin_unsupported",
            "That plugin does not provide a supported executable runtime.",
        ));
    }
    let Some(grant) = load_grant(&connection, plugin_id)? else {
        return Err(Error::new(
            "plugin_denied",
            "This plugin has no approved runtime grant. Enable it in Settings > Plugins.",
        ));
    };
    let entry = plugin
        .manifest
        .runtime
        .entry
        .clone()
        .ok_or_else(|| Error::new("plugin_invalid", "The package does not declare a runtime entry."))?;
    let root = install_root(store)?
        .join(&plugin.manifest.id)
        .join(&plugin.manifest.version);
    if !root.is_dir() {
        return Err(Error::new(
            "plugin_missing",
            "The installed package files are missing. Reinstall the plugin.",
        ));
    }
    let declared_permissions: BTreeSet<Permission> =
        plugin.manifest.permissions.iter().copied().collect();
    let declared_network = plugin
        .manifest
        .network
        .as_ref()
        .map(|network| network.allow.clone())
        .unwrap_or_default();
    Ok(RuntimePackage {
        id: plugin.manifest.id.clone(),
        name: plugin.manifest.name.clone(),
        version: plugin.manifest.version.clone(),
        api_version: plugin.manifest.api_version,
        kind: plugin.manifest.runtime.kind,
        entry,
        permissions: declared_permissions
            .intersection(&grant.permissions)
            .copied()
            .collect(),
        network: grant
            .network
            .into_iter()
            .filter(|origin| declared_network.contains(origin))
            .collect(),
        settings: plugin
            .manifest
            .settings
            .iter()
            .map(|definition| (definition.key.clone(), plugin.setting_value(definition)))
            .collect(),
        root,
    })
}

pub(crate) fn granted_plugins(store: &Store) -> Result<Vec<(String, Grant)>, Error> {
    let connection = store.connection()?;
    let plugins = load_plugins(&connection)?;
    let mut granted = Vec::new();
    for (id, plugin) in plugins {
        if !plugin.enabled
            || !plugin.manifest.runtime.kind.is_executable()
            || plugin.execution_reason().is_some()
        {
            continue;
        }
        if let Some(grant) = load_grant(&connection, &id)? {
            if grant.trigger.supports(plugin.manifest.runtime.kind) {
                granted.push((id, grant));
            }
        }
    }
    Ok(granted)
}

pub(super) fn create(connection: &Connection) -> Result<(), Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS plugins (
            id TEXT PRIMARY KEY,
            manifest TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            detached TEXT NOT NULL DEFAULT '[]',
            disabled_reason TEXT,
            source TEXT NOT NULL,
            installed_at TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            content_hash TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS plugin_settings (
            plugin_id TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            PRIMARY KEY (plugin_id, key)
        );
        CREATE TABLE IF NOT EXISTS plugin_grants (
            plugin_id TEXT PRIMARY KEY,
            version TEXT NOT NULL,
            permissions TEXT NOT NULL,
            network TEXT NOT NULL,
            trigger TEXT NOT NULL,
            granted_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

pub(super) fn migrate(connection: &mut Connection) -> Result<(), Error> {
    create(connection)?;
    let transaction = connection.transaction()?;
    let has_reason: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('plugins') WHERE name = 'disabled_reason')",
        [],
        |row| row.get(0),
    )?;
    if !has_reason {
        transaction.execute("ALTER TABLE plugins ADD COLUMN disabled_reason TEXT", [])?;
    }
    let notices = reconcile(&transaction)?;
    transaction.commit()?;
    if !notices.is_empty() {
        eprintln!("{}", notices.join(" "));
    }
    Ok(())
}

pub(super) fn reconcile(connection: &Connection) -> Result<Vec<String>, Error> {
    let mut plugins = load_plugins(connection)?;
    let base = super::Store::current(connection)?
        .map(|document| document.config)
        .unwrap_or_default();
    let mut owners: BTreeMap<&'static str, String> = BTreeMap::new();
    let mut notices = Vec::new();
    let mut changed = Vec::new();
    for (id, plugin) in plugins.iter_mut() {
        if !plugin.enabled || plugin.execution_reason().is_some() {
            continue;
        }
        if plugin.manifest.runtime.kind.is_executable() && load_grant(connection, id)?.is_none() {
            plugin.enabled = false;
            plugin.disabled_reason = Some(
                "Disabled because its runtime permission grant is missing. Review and enable it again."
                    .into(),
            );
            changed.push(id.clone());
            notices.push(format!(
                "{} needs its runtime permissions reviewed again.",
                plugin.manifest.name
            ));
            continue;
        }
        if let Err(error) = apply_plugins(&base, &[(id.clone(), plugin.clone())]) {
            plugin.enabled = false;
            plugin.disabled_reason = Some(format!(
                "Disabled because its contribution could not be applied: {}",
                error.message
            ));
            notices.push(format!(
                "{} was disabled because its contribution is invalid.",
                plugin.manifest.name
            ));
            changed.push(id.clone());
        }
    }
    for (id, plugin) in plugins.iter_mut() {
        if plugin.enabled && plugin.execution_reason().is_some() {
            plugin.enabled = false;
            plugin.disabled_reason = plugin.execution_reason();
            changed.push(id.clone());
            notices.push(format!(
                "{} uses an unsupported runtime and was left disabled.",
                plugin.manifest.name
            ));
            continue;
        }
        if !plugin.enabled || plugin.execution_reason().is_some() {
            continue;
        }
        let mut conflicts = Vec::new();
        for field in plugin.active_fields() {
            if let Some(owner) = owners.get(field) {
                conflicts.push(format!("{field} with {owner}"));
            }
        }
        if conflicts.is_empty() {
            for field in plugin.active_fields() {
                owners.insert(field, id.clone());
            }
            continue;
        }
        plugin.enabled = false;
        plugin.disabled_reason = Some(format!(
            "Disabled at startup because it conflicts with another plugin: {}.",
            conflicts.join(", ")
        ));
        notices.push(format!(
            "{} was disabled because its controls overlap another plugin.",
            plugin.manifest.name
        ));
        changed.push(id.clone());
    }
    for id in changed {
        let reason = plugins
            .iter()
            .find(|(candidate, _)| candidate == &id)
            .and_then(|(_, plugin)| plugin.disabled_reason.clone());
        connection.execute(
            "UPDATE plugins SET enabled = 0, disabled_reason = ?1 WHERE id = ?2",
            params![reason, id],
        )?;
    }
    Ok(notices)
}

fn load_plugins(connection: &Connection) -> Result<Vec<(String, StoredPlugin)>, Error> {
    let mut statement = connection.prepare(
        "SELECT id, manifest, enabled, detached, disabled_reason, source, installed_at, size_bytes, content_hash
         FROM plugins ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, String>(8)?,
        ))
    })?;
    let mut plugins = Vec::new();
    for row in rows {
        let (
            id,
            manifest_json,
            enabled,
            detached,
            disabled_reason,
            source,
            installed_at,
            size_bytes,
            content_hash,
        ) = row?;
        let manifest: Manifest = serde_json::from_str(&manifest_json)
            .map_err(|error| Error::new("plugin_storage", error))?;
        let detached: BTreeSet<String> = serde_json::from_str(&detached)
            .map_err(|error| Error::new("plugin_storage", error))?;
        let settings = settings_map(connection, &id)?;
        plugins.push((
            id,
            StoredPlugin {
                manifest,
                enabled: enabled != 0,
                detached,
                disabled_reason,
                source,
                installed_at,
                size_bytes: size_bytes.max(0) as u64,
                content_hash,
                settings,
            },
        ));
    }
    Ok(plugins)
}

fn settings_map(connection: &Connection, plugin_id: &str) -> Result<BTreeMap<String, Value>, Error> {
    let mut statement =
        connection.prepare("SELECT key, value FROM plugin_settings WHERE plugin_id = ?1")?;
    let rows = statement.query_map([plugin_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut settings = BTreeMap::new();
    for row in rows {
        let (key, raw) = row?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| Error::new("plugin_storage", error))?;
        settings.insert(key, value);
    }
    Ok(settings)
}

pub(super) fn snapshot(connection: &Connection) -> Result<PluginSnapshot, Error> {
    let mut plugins = Vec::new();
    for (id, plugin) in load_plugins(connection)? {
        plugins.push(PluginSnapshotEntry {
            id,
            version: plugin.manifest.version.clone(),
            enabled: plugin.enabled,
            detached: plugin.detached.iter().cloned().collect(),
            settings: plugin
                .manifest
                .settings
                .iter()
                .map(|definition| {
                    (
                        definition.key.clone(),
                        plugin.setting_value(definition),
                    )
                })
                .collect(),
        });
    }
    Ok(PluginSnapshot { plugins })
}

pub(super) fn state(connection: &Connection, base: &Config) -> Result<PluginsState, Error> {
    let effective = apply(connection, base)?;
    let plugins = load_plugins(connection)?;
    let mut enabled_fields: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for (id, plugin) in &plugins {
        if !plugin.enabled || plugin.execution_reason().is_some() {
            continue;
        }
        for field in plugin.active_fields() {
            enabled_fields.entry(field).or_default().push(id.clone());
        }
    }
    let mut records = Vec::new();
    for (id, plugin) in plugins {
        let declared = plugin.declared_fields();
        let active = plugin.active_fields();
        let mut conflicts = Vec::new();
        for field in &active {
            if let Some(owners) = enabled_fields.get(field) {
                for owner in owners {
                    if owner != &id {
                        conflicts.push(format!("{field} with {owner}"));
                    }
                }
            }
        }
        conflicts.sort();
        conflicts.dedup();
        let execution_reason = plugin.execution_reason();
        let enable_error = if plugin.enabled {
            None
        } else if let Some(reason) = &execution_reason {
            Some(reason.clone())
        } else if !conflicts.is_empty() {
            Some(format!(
                "This plugin controls fields that another enabled plugin already controls: {}.",
                conflicts.join(", ")
            ))
        } else {
            None
        };
        let grant = load_grant(connection, &id)?;
        let executable = plugin.manifest.runtime.kind.is_executable();
        records.push(PluginRecord {
            id,
            name: plugin.manifest.name.clone(),
            version: plugin.manifest.version.clone(),
            api_version: plugin.manifest.api_version,
            description: plugin.manifest.description.clone(),
            author: plugin.manifest.author.clone(),
            enabled: plugin.enabled,
            runtime: plugin.manifest.runtime.kind.label().into(),
            executable,
            execution_reason,
            permissions: plugin
                .manifest
                .permissions
                .iter()
                .map(|permission| permission.label().to_string())
                .collect(),
            network: plugin
                .manifest
                .network
                .as_ref()
                .map(|network| network.allow.clone())
                .unwrap_or_default(),
            granted_permissions: grant
                .as_ref()
                .map(|grant| {
                    grant
                        .permissions
                        .iter()
                        .map(|permission| permission.label().to_string())
                        .collect()
                })
                .unwrap_or_default(),
            granted_network: grant
                .as_ref()
                .map(|grant| grant.network.clone())
                .unwrap_or_default(),
            trigger: grant
                .as_ref()
                .map(|grant| grant.trigger.label().to_string()),
            runtime_status: None,
            entry: plugin.manifest.runtime.entry.clone(),
            surface_openable: plugin.enabled
                && executable
                && plugin.manifest.runtime.kind.has_surface()
                && grant.is_some(),
            controls: active.iter().map(|field| (*field).into()).collect(),
            declared_controls: declared.iter().map(|field| (*field).into()).collect(),
            detached_fields: plugin.detached.iter().cloned().collect(),
            conflicts,
            can_enable: !plugin.enabled && enable_error.is_none(),
            enable_error,
            disabled_reason: plugin.disabled_reason.clone(),
            settings: plugin.resolved_settings(),
            source: plugin.source.clone(),
            installed_at: plugin.installed_at.clone(),
            size_bytes: plugin.size_bytes,
            content_hash: plugin.content_hash.clone(),
            source_files: plugin.manifest.source.clone(),
            assets: plugin.manifest.assets.clone(),
        });
    }
    Ok(PluginsState {
        plugins: records,
        effective,
    })
}

pub(super) fn apply(connection: &Connection, base: &Config) -> Result<Config, Error> {
    let mut config = base.clone();
    for (_, plugin) in load_plugins(connection)? {
        if !plugin.enabled || plugin.execution_reason().is_some() {
            continue;
        }
        apply_plugin(&mut config, &plugin)?;
    }
    document::validate(&config).map_err(|message| Error::new("plugin_effective_invalid", message))?;
    Ok(config)
}

fn apply_plugin(config: &mut Config, plugin: &StoredPlugin) -> Result<(), Error> {
    let contributes = &plugin.manifest.contributes;
    let active = plugin.active_fields();
    if active.contains("theme") {
        if let Some(bound) = &contributes.theme {
            config.theme = resolve_string(plugin, bound, "theme")?;
        }
    }
    if active.contains("density") {
        if let Some(bound) = &contributes.density {
            config.density = resolve_string(plugin, bound, "density")?;
        }
    }
    if active.contains("appTileShape") {
        if let Some(bound) = &contributes.app_tile_shape {
            config.app_tile_shape = resolve_string(plugin, bound, "appTileShape")?;
        }
    }
    if active.contains("textSize") {
        if let Some(bound) = &contributes.text_size {
            config.text_size = resolve_string(plugin, bound, "textSize")?;
        }
    }
    if active.contains("reducedMotion") {
        if let Some(bound) = &contributes.reduced_motion {
            config.reduced_motion = resolve_bool(plugin, bound, "reducedMotion")?;
        }
    }
    if active.contains("homeColumns") {
        if let Some(bound) = &contributes.home_columns {
            config.home_columns = resolve_u8(plugin, bound, "homeColumns")?;
        }
    }
    if active.contains("home") {
        if let Some(home) = &contributes.home {
            let mut rows = config.home.clone();
            if let Some(order) = &home.order {
                let mut by_component: BTreeMap<String, HomeRow> = rows
                    .into_iter()
                    .map(|row| (row.component.clone(), row))
                    .collect();
                let mut ordered = Vec::with_capacity(by_component.len());
                for component in order {
                    if let Some(row) = by_component.remove(component) {
                        ordered.push(row);
                    }
                }
                ordered.extend(by_component.into_values());
                rows = ordered;
            }
            for (component, contribution) in &home.rows {
                let Some(row) = rows.iter_mut().find(|row| &row.component == component) else {
                    continue;
                };
                if let Some(bound) = &contribution.visible {
                    row.visible = resolve_bool(plugin, bound, "home")?;
                }
                if let Some(bound) = &contribution.span {
                    row.span = resolve_u8(plugin, bound, "home")?;
                }
            }
            config.home = rows;
        }
    }
    if active.contains("widgets") {
        if let Some(widgets) = &contributes.widgets {
            if let Some(bound) = &widgets.spotify_enabled {
                config.widgets.spotify.enabled = resolve_bool(plugin, bound, "widgets")?;
            }
            if let Some(bound) = &widgets.spotify_placement {
                config.widgets.spotify.placement =
                    resolve_string(plugin, bound, "widgets")?;
                config.widgets.spotify.position = None;
            }
        }
    }
    if active.contains("order") {
        if let Some(order) = &contributes.order {
            config.order = order.clone();
        }
    }
    Ok(())
}

fn definition<'a>(plugin: &'a StoredPlugin, key: &str) -> Result<&'a SettingDefinition, Error> {
    plugin
        .manifest
        .settings
        .iter()
        .find(|definition| definition.key == key)
        .ok_or_else(|| {
            Error::new(
                "plugin_invalid",
                format!("{} references an unknown setting: {key}.", plugin.manifest.name),
            )
        })
}

fn resolve_string(
    plugin: &StoredPlugin,
    bound: &Bound<String>,
    field: &str,
) -> Result<String, Error> {
    match bound {
        Bound::Value(value) => Ok(value.clone()),
        Bound::Setting { setting } => {
            let definition = definition(plugin, setting)?;
            let value = plugin.setting_value(definition);
            value.as_str().map(str::to_string).ok_or_else(|| {
                Error::new(
                    "plugin_settings",
                    format!("Setting {setting} for {field} must be a string."),
                )
            })
        }
    }
}

fn resolve_bool(plugin: &StoredPlugin, bound: &Bound<bool>, field: &str) -> Result<bool, Error> {
    match bound {
        Bound::Value(value) => Ok(*value),
        Bound::Setting { setting } => {
            let definition = definition(plugin, setting)?;
            plugin.setting_value(definition).as_bool().ok_or_else(|| {
                Error::new(
                    "plugin_settings",
                    format!("Setting {setting} for {field} must be a boolean."),
                )
            })
        }
    }
}

fn resolve_u8(plugin: &StoredPlugin, bound: &Bound<u8>, field: &str) -> Result<u8, Error> {
    match bound {
        Bound::Value(value) => Ok(*value),
        Bound::Setting { setting } => {
            let definition = definition(plugin, setting)?;
            plugin
                .setting_value(definition)
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(|| {
                    Error::new(
                        "plugin_settings",
                        format!("Setting {setting} for {field} must be a whole number."),
                    )
                })
        }
    }
}

pub(super) fn prepare(
    connection: &Connection,
    base: &Config,
    effective: &Config,
    requested: &Config,
) -> Result<PreparedWrite, Error> {
    document::validate(requested).map_err(|message| Error::new("validation", message))?;
    let mut prepared = requested.clone();
    let mut detach = Vec::new();
    let owners = enabled_owners(connection)?;
    let mut force = false;
    for field in FIELDS {
        let Some(ids) = owners.get(field) else {
            continue;
        };
        if field_changed(effective, requested, field) {
            for id in ids {
                detach.push(Detach {
                    plugin: id.clone(),
                    field,
                });
                force = true;
            }
        } else {
            prepared = copy_field(prepared, base, field);
        }
    }
    document::validate(&prepared).map_err(|message| Error::new("validation", message))?;
    Ok(PreparedWrite {
        base: prepared,
        detach,
        force,
    })
}

fn enabled_owners(connection: &Connection) -> Result<BTreeMap<&'static str, Vec<String>>, Error> {
    let mut owners: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for (id, plugin) in load_plugins(connection)? {
        if !plugin.enabled || plugin.execution_reason().is_some() {
            continue;
        }
        for field in plugin.active_fields() {
            owners.entry(field).or_default().push(id.clone());
        }
    }
    Ok(owners)
}

fn copy_field(config: Config, source: &Config, field: &str) -> Config {
    let mut config = config;
    match field {
        "theme" => config.theme = source.theme.clone(),
        "density" => config.density = source.density.clone(),
        "appTileShape" => config.app_tile_shape = source.app_tile_shape.clone(),
        "textSize" => config.text_size = source.text_size.clone(),
        "reducedMotion" => config.reduced_motion = source.reduced_motion,
        "homeColumns" => config.home_columns = source.home_columns,
        "home" => config.home = source.home.clone(),
        "widgets" => config.widgets = source.widgets.clone(),
        "order" => config.order = source.order.clone(),
        _ => {}
    }
    config
}

fn field_changed(a: &Config, b: &Config, field: &str) -> bool {
    match field {
        "theme" => a.theme != b.theme,
        "density" => a.density != b.density,
        "appTileShape" => a.app_tile_shape != b.app_tile_shape,
        "textSize" => a.text_size != b.text_size,
        "reducedMotion" => a.reduced_motion != b.reduced_motion,
        "homeColumns" => a.home_columns != b.home_columns,
        "home" => a.home != b.home,
        "widgets" => a.widgets != b.widgets,
        "order" => a.order != b.order,
        _ => false,
    }
}

pub(super) fn apply_prepared(connection: &Connection, prepared: &PreparedWrite) -> Result<(), Error> {
    for detach in &prepared.detach {
        let detached: Option<String> =
            connection.query_row("SELECT detached FROM plugins WHERE id = ?1", [&detach.plugin], |row| row.get(0)).optional()?;
        let Some(raw) = detached else {
            continue;
        };
        let mut fields: BTreeSet<String> = serde_json::from_str(&raw).unwrap_or_default();
        fields.insert(detach.field.into());
        connection.execute(
            "UPDATE plugins SET detached = ?1 WHERE id = ?2",
            params![serde_json::to_string(&fields).map_err(|error| Error::new("plugin_storage", error))?, detach.plugin],
        )?;
    }
    Ok(())
}

pub(super) fn prepare_enable(
    connection: &Connection,
    base: &Config,
    plugin_id: &str,
) -> Result<(Config, PluginAction), Error> {
    let plugins = load_plugins(connection)?;
    let (_, plugin) = plugins
        .iter()
        .find(|(id, _)| id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
    if let Some(reason) = plugin.execution_reason() {
        return Err(Error::new("plugin_unsupported", reason));
    }
    if plugin.manifest.runtime.kind.is_executable() {
        return Err(Error::new(
            "plugin_runtime",
            "Executable plugins are enabled through their runtime permission review, not a layout preview.",
        ));
    }
    if !plugin.enabled
        && plugin.manifest.contributes.fields().is_empty()
        && plugin.manifest.remote.is_none()
    {
        return Err(Error::new(
            "plugin_invalid",
            "This plugin does not declare any supported appearance, Home, or remote contribution.",
        ));
    }
    let reattach = plugin.enabled && !plugin.detached.is_empty();
    let mut next = plugins.clone();
    let (_, current) = next
        .iter_mut()
        .find(|(id, _)| id == plugin_id)
        .expect("checked above");
    current.enabled = true;
    current.detached.clear();
    current.disabled_reason = None;
    let mut owners: BTreeMap<&'static str, String> = BTreeMap::new();
    for (id, candidate) in &next {
        if !candidate.enabled || candidate.execution_reason().is_some() {
            continue;
        }
        for field in candidate.active_fields() {
            if let Some(owner) = owners.get(field) {
                if owner != id {
                    return Err(Error::new(
                        "plugin_conflict",
                        format!(
                            "{} cannot be enabled because {field} is already controlled by {owner}.",
                            plugin.manifest.name
                        ),
                    ));
                }
            } else {
                owners.insert(field, id.clone());
            }
        }
    }
    let config = apply_plugins(base, &next)?;
    Ok((
        config,
        PluginAction::Enable {
            plugin_id: plugin_id.into(),
            reattach,
        },
    ))
}

fn apply_plugins(base: &Config, plugins: &[(String, StoredPlugin)]) -> Result<Config, Error> {
    let mut config = base.clone();
    for (_, plugin) in plugins {
        if !plugin.enabled || plugin.execution_reason().is_some() {
            continue;
        }
        apply_plugin(&mut config, plugin)?;
    }
    document::validate(&config).map_err(|message| Error::new("plugin_effective_invalid", message))?;
    Ok(config)
}

pub(super) fn prepare_settings(
    connection: &Connection,
    base: &Config,
    plugin_id: &str,
    values: &Map<String, Value>,
) -> Result<(Config, PluginAction), Error> {
    let plugins = load_plugins(connection)?;
    let (_, plugin) = plugins
        .iter()
        .find(|(id, _)| id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
    let mut merged = plugin.settings.clone();
    for (key, value) in values {
        let definition = plugin
            .manifest
            .settings
            .iter()
            .find(|definition| &definition.key == key)
            .ok_or_else(|| {
                Error::new(
                    "plugin_settings",
                    format!("{} does not declare a setting named {key}.", plugin.manifest.name),
                )
            })?;
        validate_setting_value(definition, value)?;
        merged.insert(key.clone(), value.clone());
    }
    let mut next = plugins.clone();
    let (_, current) = next
        .iter_mut()
        .find(|(id, _)| id == plugin_id)
        .expect("checked above");
    current.settings = merged.clone();
    let config = apply_plugins(base, &next)?;
    Ok((
        config,
        PluginAction::Settings {
            plugin_id: plugin_id.into(),
            values: merged,
        },
    ))
}

pub(super) fn apply_action(
    connection: &Connection,
    current: &Document,
    action: &PluginAction,
) -> Result<Config, Error> {
    match action {
        PluginAction::Enable {
            plugin_id,
            reattach,
        } => {
            let plugins = load_plugins(connection)?;
            let (_, plugin) = plugins
                .iter()
                .find(|(id, _)| id == plugin_id)
                .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
            if let Some(reason) = plugin.execution_reason() {
                return Err(Error::new("plugin_unsupported", reason));
            }
            if plugin.manifest.runtime.kind.is_executable() {
                return Err(Error::new(
                    "plugin_runtime",
                    "Executable plugins are enabled through their runtime permission review.",
                ));
            }
            let mut owners: BTreeMap<&'static str, String> = BTreeMap::new();
            for (id, candidate) in &plugins {
                if !candidate.enabled || candidate.execution_reason().is_some() || id == plugin_id
                {
                    continue;
                }
                for field in candidate.active_fields() {
                    owners.insert(field, id.clone());
                }
            }
            for field in plugin.manifest.contributes.fields() {
                if let Some(owner) = owners.get(field) {
                    return Err(Error::new(
                        "plugin_conflict",
                        format!(
                            "{} cannot be enabled because {field} is already controlled by {owner}.",
                            plugin.manifest.name
                        ),
                    ));
                }
            }
            connection.execute(
                "UPDATE plugins SET enabled = 1, detached = '[]', disabled_reason = NULL WHERE id = ?1",
                [plugin_id],
            )?;
            if *reattach {
                connection.execute(
                    "UPDATE plugins SET detached = '[]' WHERE id = ?1",
                    [plugin_id],
                )?;
            }
            let _ = current;
            apply(connection, &current.config)
        }
        PluginAction::Settings { plugin_id, values } => {
            let plugins = load_plugins(connection)?;
            let (_, plugin) = plugins
                .iter()
                .find(|(id, _)| id == plugin_id)
                .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
            for (key, value) in values {
                let definition = plugin
                    .manifest
                    .settings
                    .iter()
                    .find(|definition| &definition.key == key)
                    .ok_or_else(|| {
                        Error::new(
                            "plugin_settings",
                            format!("{} does not declare a setting named {key}.", plugin.manifest.name),
                        )
                    })?;
                validate_setting_value(definition, value)?;
            }
            connection.execute("DELETE FROM plugin_settings WHERE plugin_id = ?1", [plugin_id])?;
            for (key, value) in values {
                connection.execute(
                    "INSERT INTO plugin_settings (plugin_id, key, value) VALUES (?1, ?2, ?3)",
                    params![
                        plugin_id,
                        key,
                        serde_json::to_string(value)
                            .map_err(|error| Error::new("plugin_storage", error))?
                    ],
                )?;
            }
            apply(connection, &current.config)
        }
    }
}

pub(super) fn save_settings(
    connection: &Connection,
    plugin_id: &str,
    values: &Map<String, Value>,
) -> Result<(), Error> {
    let plugins = load_plugins(connection)?;
    let (_, plugin) = plugins
        .iter()
        .find(|(id, _)| id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
    for (key, value) in values {
        let definition = plugin
            .manifest
            .settings
            .iter()
            .find(|definition| &definition.key == key)
            .ok_or_else(|| {
                Error::new(
                    "plugin_settings",
                    format!("{} does not declare a setting named {key}.", plugin.manifest.name),
                )
            })?;
        validate_setting_value(definition, value)?;
    }
    for (key, value) in values {
        connection.execute(
            "INSERT OR REPLACE INTO plugin_settings (plugin_id, key, value) VALUES (?1, ?2, ?3)",
            params![
                plugin_id,
                key,
                serde_json::to_string(value)
                    .map_err(|error| Error::new("plugin_storage", error))?
            ],
        )?;
    }
    Ok(())
}

pub(super) fn disable(connection: &Connection, plugin_id: &str) -> Result<(), Error> {
    let changed = connection.execute(
        "UPDATE plugins SET enabled = 0, detached = '[]', disabled_reason = NULL WHERE id = ?1",
        [plugin_id],
    )?;
    if changed == 0 {
        return Err(Error::new("plugin_missing", "That plugin is not installed."));
    }
    connection.execute(
        "DELETE FROM plugin_grants WHERE plugin_id = ?1",
        [plugin_id],
    )?;
    Ok(())
}

pub(super) fn remove_row(connection: &Connection, plugin_id: &str, delete_data: bool) -> Result<(), Error> {
    if delete_data {
        connection.execute("DELETE FROM plugin_settings WHERE plugin_id = ?1", [plugin_id])?;
    }
    connection.execute("DELETE FROM plugin_grants WHERE plugin_id = ?1", [plugin_id])?;
    let changed = connection.execute("DELETE FROM plugins WHERE id = ?1", [plugin_id])?;
    if changed == 0 {
        return Err(Error::new("plugin_missing", "That plugin is not installed."));
    }
    Ok(())
}

pub(super) fn restore_snapshot(
    connection: &Connection,
    snapshot: &PluginSnapshot,
) -> Result<Vec<String>, Error> {
    let installed = load_plugins(connection)?;
    let mut notices = Vec::new();
    for (id, plugin) in &installed {
        let Some(saved) = snapshot.plugins.iter().find(|entry| &entry.id == id) else {
            if plugin.enabled {
                notices.push(format!(
                    "{} was not enabled in that saved version, so it is now disabled.",
                    plugin.manifest.name
                ));
                connection.execute(
                    "UPDATE plugins SET enabled = 0, detached = '[]', disabled_reason = ?1 WHERE id = ?2",
                    params![
                        "This saved version did not have this plugin enabled.",
                        id
                    ],
                )?;
            } else {
                connection.execute(
                    "UPDATE plugins SET enabled = 0, detached = '[]' WHERE id = ?1",
                    [id],
                )?;
            }
            continue;
        };
        if let Some(reason) = plugin.execution_reason() {
            connection.execute(
                "UPDATE plugins SET enabled = 0, detached = '[]', disabled_reason = ?1 WHERE id = ?2",
                params![reason, id],
            )?;
            continue;
        }
        if plugin.manifest.runtime.kind.is_executable() {
            notices.push(format!(
                "{} uses an executable runtime and must be enabled again from its permission review.",
                plugin.manifest.name
            ));
            connection.execute(
                "UPDATE plugins SET enabled = 0, detached = '[]', disabled_reason = ?1 WHERE id = ?2",
                params![
                    "This saved version did not include the current runtime grant. Review its permissions to enable it.",
                    id
                ],
            )?;
            connection.execute(
                "DELETE FROM plugin_grants WHERE plugin_id = ?1",
                [id],
            )?;
            continue;
        }
        if saved.version != plugin.manifest.version {
            notices.push(format!(
                "{} changed version since that saved version, so it was left disabled.",
                plugin.manifest.name
            ));
            connection.execute(
                "UPDATE plugins SET enabled = 0, detached = '[]', disabled_reason = ?1 WHERE id = ?2",
                params![
                    format!(
                        "This saved version used {} {}. The installed version is {}.",
                        plugin.manifest.name, saved.version, plugin.manifest.version
                    ),
                    id
                ],
            )?;
            continue;
        }
        let valid_settings: BTreeMap<String, Value> = saved
            .settings
            .iter()
            .filter(|(key, value)| {
                plugin
                    .manifest
                    .settings
                    .iter()
                    .find(|definition| &definition.key == *key)
                    .is_some_and(|definition| validate_setting_value(definition, value).is_ok())
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        connection.execute("DELETE FROM plugin_settings WHERE plugin_id = ?1", [id])?;
        for (key, value) in &valid_settings {
            connection.execute(
                "INSERT INTO plugin_settings (plugin_id, key, value) VALUES (?1, ?2, ?3)",
                params![
                    id,
                    key,
                    serde_json::to_string(value)
                        .map_err(|error| Error::new("plugin_storage", error))?
                ],
            )?;
        }
        let detached: Vec<String> = if saved.enabled {
            saved
                .detached
                .iter()
                .filter(|field| plugin.manifest.contributes.fields().contains(field.as_str()))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        connection.execute(
            "UPDATE plugins SET enabled = ?1, detached = ?2, disabled_reason = NULL WHERE id = ?3",
            params![
                i64::from(saved.enabled),
                serde_json::to_string(&detached)
                    .map_err(|error| Error::new("plugin_storage", error))?,
                id
            ],
        )?;
    }
    for saved in &snapshot.plugins {
        if !installed.iter().any(|(id, _)| id == &saved.id) {
            notices.push(format!(
                "{} {} from that saved version is not installed.",
                saved.id, saved.version
            ));
        }
    }
    reconcile(connection)?;
    Ok(notices)
}

pub(super) fn validate_submission(value: &Value) -> Result<PreparedPackage, Error> {
    let submission: Submission = serde_json::from_value(value.clone())
        .map_err(|error| Error::new("validation", format!("Invalid plugin submission: {error}")))?;
    let manifest: Manifest = serde_json::from_value(submission.manifest.clone())
        .map_err(|error| Error::new("validation", format!("Invalid plugin manifest: {error}")))?;
    validate_manifest(&manifest)?;
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    for file in submission.files {
        validate_relative_path(&file.path)?;
        if file.path == "luma-extension.json" {
            return Err(Error::new(
                "validation",
                "Do not include luma-extension.json in files; submit the manifest separately.",
            ));
        }
        let bytes = match (file.text, file.base64) {
            (Some(text), None) => text.into_bytes(),
            (None, Some(encoded)) => base64::engine::general_purpose::STANDARD
                .decode(encoded.as_bytes())
                .map_err(|error| Error::new("validation", format!("Invalid base64 in {}: {error}", file.path)))?,
            _ => {
                return Err(Error::new(
                    "validation",
                    format!("File {} must use exactly one of text or base64.", file.path),
                ))
            }
        };
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(Error::new(
                "validation",
                format!("File {} is larger than {MAX_FILE_BYTES} bytes.", file.path),
            ));
        }
        total += bytes.len() as u64;
        if total > MAX_MCP_BYTES {
            return Err(Error::new(
                "validation",
                format!("MCP plugin files exceed {MAX_MCP_BYTES} bytes. Use Import in Settings for larger packages."),
            ));
        }
        if files.insert(file.path.clone(), bytes).is_some() {
            return Err(Error::new(
                "validation",
                "Plugin files contain a duplicate path.",
            ));
        }
    }
    finalize(manifest, files)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Submission {
    manifest: Value,
    files: Vec<SubmissionFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubmissionFile {
    path: String,
    text: Option<String>,
    base64: Option<String>,
}

fn finalize(manifest: Manifest, files: BTreeMap<String, Vec<u8>>) -> Result<PreparedPackage, Error> {
    if files.len() > MAX_FILES {
        return Err(Error::new(
            "validation",
            format!("A plugin can contain at most {MAX_FILES} files."),
        ));
    }
    let mut seen = HashSet::new();
    for path in files.keys() {
        if !seen.insert(path.to_lowercase()) {
            return Err(Error::new(
                "validation",
                format!("Package contains a duplicate or case-colliding path: {path}."),
            ));
        }
        let mut parent = path.as_str();
        while let Some(index) = parent.rfind('/') {
            parent = &parent[..index];
            if files.contains_key(parent) {
                return Err(Error::new(
                    "validation",
                    format!("Package uses {parent} as both a file and a directory."),
                ));
            }
        }
    }
    for declared in manifest
        .assets
        .iter()
        .chain(manifest.source.iter())
        .chain(manifest.runtime.entry.iter())
    {
        validate_relative_path(declared)?;
        if !files.contains_key(declared) {
            return Err(Error::new(
                "validation",
                format!("The manifest declares {declared}, but that file is missing."),
            ));
        }
    }
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|error| Error::new("validation", error))?;
    let hash = content_hash(&manifest_json, &files);
    let size = files.values().map(|bytes| bytes.len() as u64).sum::<u64>();
    Ok(PreparedPackage {
        manifest,
        manifest_json,
        files,
        hash,
        size,
    })
}

pub(super) struct PreparedPackage {
    pub manifest: Manifest,
    pub manifest_json: String,
    pub files: BTreeMap<String, Vec<u8>>,
    pub hash: String,
    pub size: u64,
}

fn content_hash(manifest_json: &str, files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut digest = Sha256::new();
    digest.update((manifest_json.len() as u64).to_le_bytes());
    digest.update(manifest_json.as_bytes());
    for (path, bytes) in files {
        digest.update((path.len() as u64).to_le_bytes());
        digest.update(path.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    format!("{:x}", digest.finalize())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), Error> {
    validate_id(&manifest.id)?;
    if manifest.name.trim().is_empty() || manifest.name.len() > 80 {
        return Err(Error::new(
            "validation",
            "Plugin name must be between 1 and 80 characters.",
        ));
    }
    validate_version(&manifest.version)?;
    if !matches!(manifest.api_version, API_VERSION | EXECUTABLE_API_VERSION) {
        return Err(Error::new(
            "incompatible",
            format!(
                "This plugin targets Luma plugin API {}, but this Luma version supports API {API_VERSION} and {EXECUTABLE_API_VERSION}.",
                manifest.api_version
            ),
        ));
    }
    if manifest.description.len() > 500 {
        return Err(Error::new(
            "validation",
            "Plugin description must be at most 500 characters.",
        ));
    }
    if manifest.author.len() > 80 {
        return Err(Error::new("validation", "Plugin author must be at most 80 characters."));
    }
    if manifest.settings.len() > MAX_SETTINGS {
        return Err(Error::new(
            "validation",
            format!("A plugin can declare at most {MAX_SETTINGS} settings."),
        ));
    }
    let mut keys = HashSet::new();
    for definition in &manifest.settings {
        validate_setting_definition(definition)?;
        if !keys.insert(definition.key.clone()) {
            return Err(Error::new(
                "validation",
                format!("Duplicate plugin setting: {}.", definition.key),
            ));
        }
    }
    if manifest.assets.len() > MAX_SOURCE_FILES || manifest.source.len() > MAX_SOURCE_FILES {
        return Err(Error::new(
            "validation",
            format!("A plugin can declare at most {MAX_SOURCE_FILES} assets and source files."),
        ));
    }
    for path in manifest.assets.iter().chain(manifest.source.iter()) {
        validate_relative_path(path)?;
    }
    if let Some(remote) = &manifest.remote {
        validate_remote(remote)?;
        if manifest.api_version == EXECUTABLE_API_VERSION {
            return Err(Error::new(
                "validation",
                "Remote panels are declarative contributions and cannot be declared by an executable plugin.",
            ));
        }
    }
    if manifest.api_version == EXECUTABLE_API_VERSION {
        validate_executable_manifest(manifest)?;
    } else {
        if !manifest.runtime.kind.is_declarative() {
            return Err(Error::new(
                "incompatible",
                "An executable plugin must use API version 2.",
            ));
        }
        if !manifest.permissions.is_empty() || manifest.network.is_some() {
            return Err(Error::new(
                "validation",
                "API version 1 plugins cannot declare runtime permissions or network access.",
            ));
        }
        if manifest.runtime.entry.is_some() {
            return Err(Error::new(
                "validation",
                "A declarative plugin cannot declare runtime.entry.",
            ));
        }
        validate_contributions(manifest)?;
        if manifest.contributes.is_empty() && manifest.remote.is_none() {
            return Err(Error::new(
                "validation",
                "A declarative plugin must contribute at least one supported appearance, Home, or remote field.",
            ));
        }
    }
    Ok(())
}

fn validate_remote(remote: &RemoteContribution) -> Result<(), Error> {
    let title = remote.title.trim();
    if title.chars().count() > MAX_REMOTE_TITLE || title.chars().any(char::is_control) {
        return Err(Error::new(
            "validation",
            format!("A remote panel title must be at most {MAX_REMOTE_TITLE} characters and contain no control characters."),
        ));
    }
    if remote.actions.len() > MAX_REMOTE_ACTIONS {
        return Err(Error::new(
            "validation",
            format!("A remote panel can declare at most {MAX_REMOTE_ACTIONS} actions."),
        ));
    }
    let mut seen = HashSet::new();
    for action in &remote.actions {
        if !seen.insert(action.label()) {
            return Err(Error::new(
                "validation",
                format!("Duplicate remote action: {}.", action.label()),
            ));
        }
    }
    Ok(())
}

fn validate_executable_manifest(manifest: &Manifest) -> Result<(), Error> {
    if !matches!(
        manifest.runtime.kind,
        RuntimeKind::IsolatedUi | RuntimeKind::Background
    ) {
        return Err(Error::new(
            "incompatible",
            "API version 2 is reserved for the isolated-ui and background runtimes.",
        ));
    }
    if !manifest.contributes.is_empty() {
        return Err(Error::new(
            "validation",
            "An executable plugin cannot declare appearance or Home contributions.",
        ));
    }
    let Some(entry) = &manifest.runtime.entry else {
        return Err(Error::new(
            "validation",
            "An executable plugin must declare runtime.entry.",
        ));
    };
    validate_relative_path(entry)?;
    if !entry.to_ascii_lowercase().ends_with(".html") {
        return Err(Error::new(
            "validation",
            "An executable runtime entry must be an HTML file.",
        ));
    }
    if manifest.permissions.len() > MAX_PERMISSIONS {
        return Err(Error::new(
            "validation",
            format!("A plugin can declare at most {MAX_PERMISSIONS} runtime permissions."),
        ));
    }
    let mut permissions = HashSet::new();
    for permission in &manifest.permissions {
        if !permissions.insert(*permission) {
            return Err(Error::new(
                "validation",
                format!("Duplicate runtime permission: {}.", permission.label()),
            ));
        }
    }
    let network = manifest.network.as_ref();
    if !permissions.contains(&Permission::NetworkGet) {
        if network.is_some() {
            return Err(Error::new(
                "validation",
                "A network allowlist requires the network.get permission.",
            ));
        }
        return Ok(());
    }
    let Some(network) = network else {
        return Err(Error::new(
            "validation",
            "The network.get permission requires a network.allow list.",
        ));
    };
    if network.allow.is_empty() || network.allow.len() > MAX_NETWORK_ORIGINS {
        return Err(Error::new(
            "validation",
            format!("network.allow must contain 1 to {MAX_NETWORK_ORIGINS} HTTPS origins."),
        ));
    }
    let mut origins = HashSet::new();
    for origin in &network.allow {
        validate_network_origin(origin)?;
        if !origins.insert(origin) {
            return Err(Error::new(
                "validation",
                format!("Duplicate network origin: {origin}."),
            ));
        }
    }
    Ok(())
}

fn validate_network_origin(origin: &str) -> Result<(), Error> {
    if origin.is_empty() || origin.len() > MAX_ORIGIN_BYTES {
        return Err(Error::new(
            "validation",
            format!("A network origin must be 1 to {MAX_ORIGIN_BYTES} characters."),
        ));
    }
    let url = url::Url::parse(origin).map_err(|_| {
        Error::new(
            "validation",
            format!("Network origin must be an HTTPS origin: {origin}."),
        )
    })?;
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::new(
            "validation",
            format!("Network origin must be an HTTPS origin without a path or credentials: {origin}."),
        ));
    }
    let Some(host) = url.host_str() else {
        return Err(Error::new(
            "validation",
            format!("Network origin has no host: {origin}."),
        ));
    };
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Err(Error::new(
            "validation",
            "Network origins cannot be IP addresses.",
        ));
    }
    let host = host.to_ascii_lowercase();
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".home.arpa")
        || !host.contains('.')
    {
        return Err(Error::new(
            "validation",
            format!("Network origin must be a public host name: {origin}."),
        ));
    }
    Ok(())
}

fn validate_setting_definition(definition: &SettingDefinition) -> Result<(), Error> {
    if !valid_key(&definition.key) {
        return Err(Error::new(
            "validation",
            format!("Invalid setting key: {}.", definition.key),
        ));
    }
    if definition.label.trim().is_empty() || definition.label.len() > 80 {
        return Err(Error::new(
            "validation",
            format!("Setting {} needs a label of 1 to 80 characters.", definition.key),
        ));
    }
    if definition.description.len() > 240 {
        return Err(Error::new(
            "validation",
            format!("Setting {} description must be at most 240 characters.", definition.key),
        ));
    }
    validate_setting_value(definition, &definition.default)?;
    Ok(())
}

fn validate_setting_value(definition: &SettingDefinition, value: &Value) -> Result<(), Error> {
    match definition.kind {
        SettingKind::Boolean => {
            if !value.is_boolean() {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be true or false.", definition.key),
                ));
            }
        }
        SettingKind::Enum => {
            if definition.options.is_empty() || definition.options.len() > MAX_SETTING_OPTIONS {
                return Err(Error::new(
                    "validation",
                    format!("Setting {} must declare 1 to {MAX_SETTING_OPTIONS} options.", definition.key),
                ));
            }
            let mut seen = HashSet::new();
            for option in &definition.options {
                if option.value.is_empty() || option.label.trim().is_empty() {
                    return Err(Error::new(
                        "validation",
                        format!("Setting {} has an empty option.", definition.key),
                    ));
                }
                if !seen.insert(option.value.clone()) {
                    return Err(Error::new(
                        "validation",
                        format!("Setting {} has duplicate options.", definition.key),
                    ));
                }
            }
            let Some(value) = value.as_str() else {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be one of its options.", definition.key),
                ));
            };
            if !definition.options.iter().any(|option| option.value == value) {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be one of its options.", definition.key),
                ));
            }
        }
        SettingKind::Number => {
            let Some(number) = value.as_f64() else {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be a number.", definition.key),
                ));
            };
            if !number.is_finite() {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be a finite number.", definition.key),
                ));
            }
            if definition.integer && number.fract() != 0.0 {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be a whole number.", definition.key),
                ));
            }
            if definition.min.is_some_and(|min| number < min) {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} is below its minimum.", definition.key),
                ));
            }
            if definition.max.is_some_and(|max| number > max) {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} is above its maximum.", definition.key),
                ));
            }
            if definition.min.is_some_and(|min| !min.is_finite())
                || definition.max.is_some_and(|max| !max.is_finite())
            {
                return Err(Error::new(
                    "validation",
                    format!("Setting {} has an invalid range.", definition.key),
                ));
            }
            if definition
                .min
                .zip(definition.max)
                .is_some_and(|(min, max)| min > max)
            {
                return Err(Error::new(
                    "validation",
                    format!("Setting {} has an invalid range.", definition.key),
                ));
            }
        }
        SettingKind::String => {
            let Some(value) = value.as_str() else {
                return Err(Error::new(
                    "plugin_settings",
                    format!("Setting {} must be text.", definition.key),
                ));
            };
            if value.chars().count() < definition.min_length
                || value.chars().count() > definition.max_length
            {
                return Err(Error::new(
                    "plugin_settings",
                    format!(
                        "Setting {} must be {} to {} characters.",
                        definition.key, definition.min_length, definition.max_length
                    ),
                ));
            }
            if definition.max_length == 0 || definition.max_length > 500 {
                return Err(Error::new(
                    "validation",
                    format!("Setting {} has an invalid length limit.", definition.key),
                ));
            }
        }
    }
    Ok(())
}

fn validate_contributions(manifest: &Manifest) -> Result<(), Error> {
    let contributes = &manifest.contributes;
    if let Some(bound) = &contributes.theme {
        validate_string_bound(manifest, bound, "theme", &BUILTIN_THEMES)?;
    }
    if let Some(bound) = &contributes.density {
        validate_string_bound(manifest, bound, "density", &DENSITIES)?;
    }
    if let Some(bound) = &contributes.app_tile_shape {
        validate_string_bound(manifest, bound, "appTileShape", &TILE_SHAPES)?;
    }
    if let Some(bound) = &contributes.text_size {
        validate_string_bound(manifest, bound, "textSize", &TEXT_SIZES)?;
    }
    if let Some(bound) = &contributes.reduced_motion {
        validate_bool_bound(manifest, bound, "reducedMotion")?;
    }
    if let Some(bound) = &contributes.home_columns {
        validate_u8_bound(manifest, bound, "homeColumns", 1, 2)?;
    }
    if let Some(home) = &contributes.home {
        if let Some(order) = &home.order {
            validate_order(order)?;
        }
        for (component, row) in &home.rows {
            if !COMPONENTS.contains(&component.as_str()) {
                return Err(Error::new(
                    "validation",
                    format!("Unknown Home component in contribution: {component}."),
                ));
            }
            if let Some(bound) = &row.visible {
                validate_bool_bound(manifest, bound, "home")?;
            }
            if let Some(bound) = &row.span {
                validate_u8_bound(manifest, bound, "home", 1, 2)?;
            }
        }
    }
    if let Some(widgets) = &contributes.widgets {
        if let Some(bound) = &widgets.spotify_enabled {
            validate_bool_bound(manifest, bound, "widgets")?;
        }
        if let Some(bound) = &widgets.spotify_placement {
            validate_string_bound(manifest, bound, "widgets", &PLACEMENTS)?;
        }
    }
    if let Some(order) = &contributes.order {
        validate_order(order)?;
    }
    Ok(())
}

fn validate_string_bound(
    manifest: &Manifest,
    bound: &Bound<String>,
    field: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    match bound {
        Bound::Value(value) => {
            if !allowed.contains(&value.as_str()) {
                return Err(Error::new(
                    "validation",
                    format!("{field} has an unsupported value: {value}."),
                ));
            }
        }
        Bound::Setting { setting } => {
            let definition = manifest
                .settings
                .iter()
                .find(|definition| &definition.key == setting)
                .ok_or_else(|| {
                    Error::new(
                        "validation",
                        format!("{field} references unknown setting: {setting}."),
                    )
                })?;
            if definition.kind != SettingKind::Enum {
                return Err(Error::new(
                    "validation",
                    format!("{field} must bind to an enum setting, not {setting}."),
                ));
            }
            if definition
                .options
                .iter()
                .any(|option| !allowed.contains(&option.value.as_str()))
            {
                return Err(Error::new(
                    "validation",
                    format!("{field} binding {setting} has an unsupported option."),
                ));
            }
        }
    }
    Ok(())
}

fn validate_bool_bound(
    manifest: &Manifest,
    bound: &Bound<bool>,
    field: &str,
) -> Result<(), Error> {
    if let Bound::Setting { setting } = bound {
        let definition = manifest
            .settings
            .iter()
            .find(|definition| &definition.key == setting)
            .ok_or_else(|| {
                Error::new(
                    "validation",
                    format!("{field} references unknown setting: {setting}."),
                )
            })?;
        if definition.kind != SettingKind::Boolean {
            return Err(Error::new(
                "validation",
                format!("{field} must bind to a boolean setting."),
            ));
        }
    }
    Ok(())
}

fn validate_u8_bound(
    manifest: &Manifest,
    bound: &Bound<u8>,
    field: &str,
    min: u8,
    max: u8,
) -> Result<(), Error> {
    match bound {
        Bound::Value(value) => {
            if *value < min || *value > max {
                return Err(Error::new(
                    "validation",
                    format!("{field} must be between {min} and {max}."),
                ));
            }
        }
        Bound::Setting { setting } => {
            let definition = manifest
                .settings
                .iter()
                .find(|definition| &definition.key == setting)
                .ok_or_else(|| {
                    Error::new(
                        "validation",
                        format!("{field} references unknown setting: {setting}."),
                    )
                })?;
            if definition.kind != SettingKind::Number
                || !definition.integer
                || definition.min.unwrap_or(f64::from(min)) < f64::from(min)
                || definition.max.unwrap_or(f64::from(max)) > f64::from(max)
            {
                return Err(Error::new(
                    "validation",
                    format!("{field} must bind to an integer setting limited to {min} through {max}."),
                ));
            }
        }
    }
    Ok(())
}

fn validate_order(order: &[String]) -> Result<(), Error> {
    if order.len() != COMPONENTS.len() {
        return Err(Error::new(
            "validation",
            "A Home order contribution must contain all seven components.",
        ));
    }
    let mut seen = HashSet::new();
    for component in order {
        if !COMPONENTS.contains(&component.as_str()) || !seen.insert(component.clone()) {
            return Err(Error::new(
                "validation",
                "A Home order contribution must contain each component exactly once.",
            ));
        }
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), Error> {
    if id.len() < 3 || id.len() > 96 {
        return Err(Error::new(
            "validation",
            "Plugin id must be between 3 and 96 characters.",
        ));
    }
    let lowercase = id.to_ascii_lowercase();
    if id != lowercase {
        return Err(Error::new(
            "validation",
            "Plugin id must be lowercase.",
        ));
    }
    let id = lowercase.as_str();
    if id.starts_with("core.") || id == "core" {
        return Err(Error::new(
            "validation",
            "The core namespace is reserved for Luma.",
        ));
    }
    let segments: Vec<&str> = id.split('.').collect();
    if segments.len() < 2
        || segments.iter().any(|segment| {
            segment.is_empty()
                || segment.len() > 32
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        })
    {
        return Err(Error::new(
            "validation",
            "Plugin id must be a lowercase vendor-qualified id such as com.example.pack.",
        ));
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<(), Error> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 || version.len() > 32 {
        return Err(Error::new(
            "validation",
            "Plugin version must use major.minor.patch.",
        ));
    }
    for part in parts {
        if part.is_empty()
            || part.len() > 9
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(Error::new(
                "validation",
                "Plugin version must use numeric major.minor.patch components without leading zeros.",
            ));
        }
    }
    Ok(())
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key.as_bytes()[0].is_ascii_lowercase()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

pub(super) fn validate_relative_path(path: &str) -> Result<(), Error> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES {
        return Err(Error::new(
            "validation",
            format!("Package path must be 1 to {MAX_PATH_BYTES} characters."),
        ));
    }
    if path.starts_with('/') || path.starts_with('\\') || path.contains('\\') {
        return Err(Error::new(
            "validation",
            format!("Package path must use forward slashes and be relative: {path}."),
        ));
    }
    if path.contains('\0') || path.chars().any(char::is_control) {
        return Err(Error::new(
            "validation",
            format!("Package path contains an invalid character: {path}."),
        ));
    }
    if path.contains(':')
        || path.contains('?')
        || path.contains('*')
        || path.contains('"')
        || path.contains('<')
        || path.contains('>')
        || path.contains('|')
    {
        return Err(Error::new(
            "validation",
            format!("Package path contains a Windows-reserved character: {path}."),
        ));
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(Error::new(
                "validation",
                format!("Package path contains an invalid segment: {path}."),
            ));
        }
        if segment.ends_with('.') || segment.ends_with(' ') {
            return Err(Error::new(
                "validation",
                format!("Package path ends a segment with a dot or space: {path}."),
            ));
        }
        let stem = segment.split('.').next().unwrap_or(segment);
        let upper = stem.to_ascii_uppercase();
        if matches!(
            upper.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5"
                | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4"
                | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
        ) {
            return Err(Error::new(
                "validation",
                format!("Package path uses a reserved Windows name: {path}."),
            ));
        }
    }
    let lower = path.to_ascii_lowercase();
    let executable = lower.rsplit('.').next().unwrap_or("");
    if matches!(
        executable,
        "exe" | "dll" | "msi" | "bat" | "cmd" | "ps1" | "scr" | "com" | "cpl"
    ) {
        return Err(Error::new(
            "validation",
            format!("Executable files cannot be packaged in a declarative plugin: {path}."),
        ));
    }
    Ok(())
}

fn install_root(store: &Store) -> Result<PathBuf, Error> {
    store
        .plugin_root()
        .ok_or_else(|| Error::new("storage", "Plugin storage is unavailable."))
}

pub(super) fn import_archive(
    store: &Store,
    bytes: &[u8],
    source: &str,
) -> Result<ImportOutcome, Error> {
    let package = archive::read(bytes)?;
    install(store, package, source)
}

pub(super) fn stage_submission(
    store: &Store,
    submission: &Value,
    source: &str,
) -> Result<ImportOutcome, Error> {
    let package = validate_submission(submission)?;
    install(store, package, source)
}

fn install(
    store: &Store,
    package: PreparedPackage,
    source: &str,
) -> Result<ImportOutcome, Error> {
    let root = install_root(store)?;
    let mut connection = store.connection()?;
    let existing: Option<(String, String, i64)> = connection
        .query_row(
            "SELECT manifest, content_hash, enabled FROM plugins WHERE id = ?1",
            [&package.manifest.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let mut existing_version = None;
    if let Some((raw, stored_hash, enabled)) = existing {
        let manifest: Manifest = serde_json::from_str(&raw)
            .map_err(|error| Error::new("plugin_storage", error))?;
        existing_version = Some(manifest.version.clone());
        if manifest.version == package.manifest.version {
            if stored_hash == package.hash {
                return Ok(ImportOutcome {
                    plugin: plugin_record(&connection, &package.manifest.id)?,
                    already_installed: true,
                    replaced_version: None,
                });
            }
            return Err(Error::new(
                "immutable_version",
                format!(
                    "{} {} is already installed with different content. Increment the version before importing.",
                    package.manifest.name, package.manifest.version
                ),
            ));
        }
        if source == "mcp" && enabled != 0 {
            return Err(Error::new(
                "plugin_enabled",
                format!(
                    "{} is enabled. Disable it in Luma > Settings > Plugins before staging an update.",
                    package.manifest.name
                ),
            ));
        }
    }
    let staging = write_staging(&root, &package)?;
    let target = root.join(&package.manifest.id).join(&package.manifest.version);
    if let Err(error) = finalize_staging(&staging, &target) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let result = (|| -> Result<Document, Error> {
        let transaction = connection.transaction()?;
        let was_enabled: bool = transaction
            .query_row(
                "SELECT enabled FROM plugins WHERE id = ?1",
                [&package.manifest.id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some_and(|enabled| enabled != 0);
        let installed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        if existing_version.is_some() {
            transaction.execute(
                "UPDATE plugins SET manifest = ?1, enabled = 0, detached = '[]', disabled_reason = ?2, source = ?3, installed_at = ?4, size_bytes = ?5, content_hash = ?6 WHERE id = ?7",
                params![
                    package.manifest_json,
                    format!(
                        "Updated to version {}. Review it before enabling.",
                        package.manifest.version
                    ),
                    source,
                    installed_at,
                    package.size as i64,
                    package.hash,
                    package.manifest.id
                ],
            )?;
            let existing = settings_map(&transaction, &package.manifest.id)?;
            transaction.execute(
                "DELETE FROM plugin_settings WHERE plugin_id = ?1",
                [&package.manifest.id],
            )?;
            for (key, value) in existing {
                if package
                    .manifest
                    .settings
                    .iter()
                    .any(|definition| definition.key == key)
                {
                    transaction.execute(
                        "INSERT INTO plugin_settings (plugin_id, key, value) VALUES (?1, ?2, ?3)",
                        params![
                            package.manifest.id,
                            key,
                            serde_json::to_string(&value)
                                .map_err(|error| Error::new("plugin_storage", error))?
                        ],
                    )?;
                }
            }
            if let Some(grant) = load_grant(&transaction, &package.manifest.id)? {
                let declared: BTreeSet<Permission> =
                    package.manifest.permissions.iter().copied().collect();
                let allowed_network = package
                    .manifest
                    .network
                    .as_ref()
                    .map(|network| network.allow.as_slice())
                    .unwrap_or(&[]);
                let declared_network: BTreeSet<&String> = allowed_network.iter().collect();
                let granted_network: BTreeSet<&String> = grant.network.iter().collect();
                let compatible = declared == grant.permissions
                    && declared_network == granted_network
                    && grant
                        .trigger
                        .supports(package.manifest.runtime.kind);
                if compatible {
                    transaction.execute(
                        "UPDATE plugin_grants SET version = ?1 WHERE plugin_id = ?2",
                        params![package.manifest.version, package.manifest.id],
                    )?;
                } else {
                    transaction.execute(
                        "DELETE FROM plugin_grants WHERE plugin_id = ?1",
                        [&package.manifest.id],
                    )?;
                }
            }
        } else {
            transaction.execute(
                "INSERT INTO plugins (id, manifest, enabled, detached, disabled_reason, source, installed_at, size_bytes, content_hash) VALUES (?1, ?2, 0, '[]', NULL, ?3, ?4, ?5, ?6)",
                params![
                    package.manifest.id,
                    package.manifest_json,
                    source,
                    installed_at,
                    package.size as i64,
                    package.hash
                ],
            )?;
        }
        let current = Store::current(&transaction)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before installing a plugin.",
            )
        })?;
        let config = apply(&transaction, &current.config)?;
        let force = was_enabled || config != current.config;
        let document = Store::write(&transaction, &current, current.config.clone(), force)?;
        transaction.commit()?;
        Ok(document)
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&target);
        return Err(error);
    }
    if let Some(version) = existing_version.clone() {
        if version != package.manifest.version {
            let old = root.join(&package.manifest.id).join(version);
            let _ = fs::remove_dir_all(old);
        }
    }
    Ok(ImportOutcome {
        plugin: plugin_record(&connection, &package.manifest.id)?,
        already_installed: false,
        replaced_version: existing_version,
    })
}

fn plugin_record(connection: &Connection, plugin_id: &str) -> Result<PluginRecord, Error> {
    let current = Store::current(connection)?.ok_or_else(|| {
        Error::new("not_initialized", "Customization is not initialized.")
    })?;
    let state = state(connection, &current.config)?;
    state
        .plugins
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))
}

fn write_staging(root: &Path, package: &PreparedPackage) -> Result<PathBuf, Error> {
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|error| Error::new("storage", error))?;
    let staging = root.join(format!(
        ".staging-{}",
        nonce.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
    ));
    fs::create_dir_all(&staging).map_err(|error| Error::new("storage", error))?;
    fs::write(staging.join("luma-extension.json"), &package.manifest_json)
        .map_err(|error| Error::new("storage", error))?;
    for (path, bytes) in &package.files {
        let target = staging.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::new("storage", error))?;
        }
        fs::write(target, bytes).map_err(|error| Error::new("storage", error))?;
    }
    Ok(staging)
}

fn finalize_staging(staging: &Path, target: &Path) -> Result<(), Error> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::new("storage", error))?;
    }
    if target.exists() {
        fs::remove_dir_all(target).map_err(|error| Error::new("storage", error))?;
    }
    fs::rename(staging, target).map_err(|error| Error::new("storage", error))
}

pub(super) fn delete_package(store: &Store, plugin_id: &str, version: &str) {
    if let Ok(root) = install_root(store) {
        let directory = root.join(plugin_id);
        let _ = fs::remove_dir_all(directory.join(version));
        let _ = fs::remove_dir(directory);
    }
}

#[derive(Debug)]
pub(super) struct ImportOutcome {
    pub plugin: PluginRecord,
    pub already_installed: bool,
    pub replaced_version: Option<String>,
}

pub(super) fn export_archive(store: &Store, plugin_id: &str) -> Result<(String, Vec<u8>), Error> {
    let connection = store.connection()?;
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT manifest, content_hash FROM plugins WHERE id = ?1",
            [plugin_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((manifest_json, _)) = row else {
        return Err(Error::new("plugin_missing", "That plugin is not installed."));
    };
    let manifest: Manifest = serde_json::from_str(&manifest_json)
        .map_err(|error| Error::new("plugin_storage", error))?;
    let root = install_root(store)?;
    let directory = root.join(plugin_id).join(&manifest.version);
    if !directory.is_dir() {
        return Err(Error::new(
            "plugin_missing",
            "The installed plugin files are missing. Reinstall the plugin.",
        ));
    }
    let mut files = BTreeMap::new();
    collect_files(&directory, &directory, &mut files)?;
    if files.len() > MAX_FILES {
        return Err(Error::new(
            "storage",
            format!("A plugin can contain at most {MAX_FILES} files."),
        ));
    }
    if files.values().map(|bytes| bytes.len() as u64).sum::<u64>() > MAX_TOTAL_BYTES {
        return Err(Error::new(
            "storage",
            format!("A plugin can export at most {MAX_TOTAL_BYTES} bytes."),
        ));
    }
    for declared in manifest
        .assets
        .iter()
        .chain(manifest.source.iter())
        .chain(manifest.runtime.entry.iter())
    {
        if !files.contains_key(declared) {
            return Err(Error::new(
                "plugin_missing",
                format!(
                    "The installed plugin is missing {declared}. Reinstall it before exporting."
                ),
            ));
        }
    }
    let bytes = archive::write(&manifest, &files)?;
    Ok((
        format!("{}-{}.luma-extension.zip", manifest.id, manifest.version),
        bytes,
    ))
}

fn collect_files(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), Error> {
    for entry in fs::read_dir(directory).map_err(|error| Error::new("storage", error))? {
        let entry = entry.map_err(|error| Error::new("storage", error))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "luma-extension.json" {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| Error::new("storage", error))?
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        validate_relative_path(&relative)?;
        let bytes = fs::read(&path).map_err(|error| Error::new("storage", error))?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(Error::new("storage", "A plugin file is too large to export."));
        }
        files.insert(relative, bytes);
    }
    Ok(())
}

pub(super) fn open_dialog(app: &tauri::AppHandle) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .add_filter("Luma plugin", &["zip"])
        .blocking_pick_file()
        .and_then(|path| path.into_path().ok())
}

pub(super) fn save_target(app: &tauri::AppHandle, name: &str) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .add_filter("Luma plugin", &["zip"])
        .set_file_name(name)
        .blocking_save_file()
        .and_then(|path| path.into_path().ok())
}

pub(super) fn state_for_app(app: &tauri::AppHandle) -> Result<PluginsState, Error> {
    let store = app.state::<Store>();
    let document = store.read()?.ok_or_else(|| {
        Error::new("not_initialized", "Open Luma once before managing plugins.")
    })?;
    let mut state = {
        let connection = store.connection()?;
        state(&connection, &document.config)?
    };
    for record in &mut state.plugins {
        record.runtime_status = crate::extension_spike::runtime_status(app, &record.id);
    }
    Ok(state)
}

pub(crate) fn disable_on_crash(
    app: &tauri::AppHandle,
    plugin_id: &str,
    reason: &str,
) -> Result<(), Error> {
    let store = app.state::<Store>();
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let current = Store::current(&transaction)?
        .ok_or_else(|| Error::new("not_initialized", "Customization is not initialized."))?;
    let name: Option<String> = transaction
        .query_row(
            "SELECT manifest FROM plugins WHERE id = ?1",
            [plugin_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|raw| {
            serde_json::from_str::<Manifest>(&raw)
                .map(|manifest| manifest.name)
                .unwrap_or_else(|_| plugin_id.to_string())
        });
    let changed = transaction.execute(
        "UPDATE plugins SET enabled = 0, detached = '[]', disabled_reason = ?1 WHERE id = ?2",
        params![reason, plugin_id],
    )?;
    if changed == 0 {
        return Err(Error::new("plugin_missing", "That plugin is not installed."));
    }
    transaction.execute(
        "DELETE FROM plugin_grants WHERE plugin_id = ?1",
        [plugin_id],
    )?;
    Store::write(&transaction, &current, current.config.clone(), true)?;
    transaction.commit()?;
    drop(connection);
    if let Ok(Some(snapshot)) = store.snapshot() {
        super::publish(app, &snapshot.document);
    }
    notice(
        app,
        format!(
            "{} was disabled after repeated runtime failures. Open Settings > Plugins for details.",
            name.unwrap_or_else(|| plugin_id.to_string())
        ),
    );
    Ok(())
}

pub(crate) fn remote_panels(app: &tauri::AppHandle) -> Result<Vec<RemotePanel>, Error> {
    let store = app.state::<Store>();
    let connection = store.connection()?;
    remote_panels_for(&connection)
}

fn remote_panels_for(connection: &Connection) -> Result<Vec<RemotePanel>, Error> {
    let mut panels = Vec::new();
    for (id, plugin) in load_plugins(connection)? {
        if !plugin.enabled
            || plugin.manifest.runtime.kind != RuntimeKind::Declarative
            || plugin.execution_reason().is_some()
        {
            continue;
        }
        let Some(remote) = &plugin.manifest.remote else {
            continue;
        };
        let title = remote.title.trim();
        panels.push(RemotePanel {
            id,
            name: plugin.manifest.name.clone(),
            title: if title.is_empty() {
                plugin.manifest.name.clone()
            } else {
                title.to_string()
            },
            actions: remote.actions.clone(),
        });
    }
    panels.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(panels)
}

pub(crate) fn remote_action(
    app: &tauri::AppHandle,
    plugin_id: &str,
    action: &str,
) -> Result<RemoteAction, Error> {
    let store = app.state::<Store>();
    let connection = store.connection()?;
    remote_action_for(&connection, plugin_id, action)
}

fn remote_action_for(
    connection: &Connection,
    plugin_id: &str,
    action: &str,
) -> Result<RemoteAction, Error> {
    let plugins = load_plugins(connection)?;
    let (_, plugin) = plugins
        .iter()
        .find(|(id, _)| id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
    if !plugin.enabled
        || plugin.manifest.runtime.kind != RuntimeKind::Declarative
        || plugin.execution_reason().is_some()
    {
        return Err(Error::new(
            "plugin_disabled",
            "That plugin is not enabled for remote control.",
        ));
    }
    let declared = plugin
        .manifest
        .remote
        .as_ref()
        .is_some_and(|remote| remote.actions.iter().any(|declared| declared.label() == action));
    if !declared {
        return Err(Error::new(
            "denied",
            "That plugin did not declare this remote action.",
        ));
    }
    RemoteAction::parse(action)
        .ok_or_else(|| Error::new("denied", "Unknown remote action."))
}

pub(crate) fn notice(app: &tauri::AppHandle, message: impl ToString) {
    let _ = app.emit_to("main", "customization-notice", message.to_string());
}

pub(crate) fn notify_plugins(app: &tauri::AppHandle) {
    crate::background::refresh_plugin_menu(app);
    if let Ok(state) = state_for_app(app) {
        let _ = app.emit_to("main", "customization-plugins-changed", state);
    }
}

pub fn schema_document() -> Value {
    let example = example_manifest();
    serde_json::json!({
        "schemaVersion": 1,
        "manifestFile": "luma-extension.json",
        "apiVersion": API_VERSION,
        "executableApiVersion": EXECUTABLE_API_VERSION,
        "id": {
            "pattern": "lowercase vendor-qualified segments such as com.example.pack",
            "maxLength": 96
        },
        "version": "major.minor.patch",
        "fields": FIELDS,
        "settingTypes": ["boolean", "enum", "number", "string"],
        "binding": {
            "static": "Use the value directly.",
            "setting": { "setting": "settingKey" },
            "rules": [
                "Theme, density, appTileShape, textSize, and widgets.spotifyPlacement bind to enum settings whose options are all supported values.",
                "reducedMotion and home row visible and widgets.spotifyEnabled bind to boolean settings.",
                "homeColumns and home row span bind to integer number settings limited to 1 and 2."
            ]
        },
        "contributions": {
            "theme": "static theme or enum setting",
            "density": "static density or enum setting",
            "appTileShape": "static tile shape or enum setting",
            "textSize": "static text size or enum setting",
            "reducedMotion": "static boolean or boolean setting",
            "homeColumns": "static 1 or 2, or integer setting 1 through 2",
            "home": {
                "order": "all seven components once",
                "rows": {
                    "component": {
                        "visible": "static boolean or boolean setting",
                        "span": "static 1 or 2, or integer setting 1 through 2"
                    }
                }
            },
            "widgets": {
                "spotifyEnabled": "static boolean or boolean setting",
                "spotifyPlacement": "static placement or enum setting"
            },
            "order": "all twelve app ids exactly once, Custom last"
        },
        "remote": {
            "field": "remote",
            "apiVersion": API_VERSION,
            "shape": { "title": "optional 1-48 character panel title", "actions": ["playback.toggle", "playback.next", "playback.previous"] },
            "behavior": "A declarative package may contribute one host-rendered remote panel. Empty actions show now-playing only. Only enabled declarative packages are projected; extension code never reaches the remote."
        },
        "runtime": {
            "kinds": ["declarative", "isolated-ui", "background", "main-page"],
            "supported": ["declarative", "isolated-ui", "background"],
            "executable": {
                "apiVersion": EXECUTABLE_API_VERSION,
                "kinds": ["isolated-ui", "background"],
                "entry": "HTML file inside the package",
                "permissions": PERMISSIONS.iter().map(|permission| permission.label()).collect::<Vec<_>>(),
                "network": "network.get requires network.allow with 1 to 8 public HTTPS origins; the host resolves the name and rejects non-public addresses",
                "triggers": {
                    "isolated-ui": ["launcher-open"],
                    "background": ["launcher-open", "startup", "always"]
                },
                "behavior": "Executable packages install disabled. The user reviews the exact declared permissions, network origins, and trigger in Settings > Plugins; that consent is required before the runtime starts. Isolated-ui packages open in a host-owned window with a package-specific origin and a fixed bridge. Background packages run headless in the same isolated runtime. Disable, removal, and exit stop the runtime."
            },
            "unsupportedBehavior": "The main-page runtime installs disabled and can never be enabled. It is never executed inside the main window."
        },
        "limits": limits(),
        "example": example
    })
}

pub(crate) fn limits() -> Value {
    serde_json::json!({
        "archiveBytes": MAX_ARCHIVE_BYTES,
        "mcpSubmissionBytes": MAX_MCP_BYTES,
        "files": MAX_FILES,
        "entries": MAX_ENTRIES,
        "fileBytes": MAX_FILE_BYTES,
        "totalBytes": MAX_TOTAL_BYTES,
        "compressionRatio": MAX_COMPRESSION_RATIO,
        "manifestBytes": MAX_MANIFEST_BYTES,
        "settings": MAX_SETTINGS,
        "settingOptions": MAX_SETTING_OPTIONS,
        "sourceFiles": MAX_SOURCE_FILES,
        "networkOrigins": MAX_NETWORK_ORIGINS,
        "pathBytes": MAX_PATH_BYTES
    })
}

pub fn example_manifest() -> Value {
    serde_json::json!({
        "id": "com.example.cinema-lounge",
        "name": "Cinema Lounge",
        "version": "1.0.0",
        "apiVersion": API_VERSION,
        "description": "Warm two-column Home with a controllable featured row.",
        "author": "Example",
        "settings": [
            {
                "key": "theme",
                "label": "Theme",
                "description": "Palette used by the pack.",
                "type": "enum",
                "default": "ember",
                "options": [
                    { "value": "ember", "label": "Ember" },
                    { "value": "onyx", "label": "Onyx" }
                ]
            },
            {
                "key": "showFeatured",
                "label": "Show featured movies",
                "description": "Turn the featured row on or off.",
                "type": "boolean",
                "default": true
            },
            {
                "key": "columns",
                "label": "Home columns",
                "type": "number",
                "default": 2,
                "min": 1,
                "max": 2,
                "integer": true
            }
        ],
        "contributes": {
            "theme": { "setting": "theme" },
            "homeColumns": { "setting": "columns" },
            "home": {
                "rows": {
                    "featured": { "visible": { "setting": "showFeatured" } }
                }
            }
        },
        "runtime": { "kind": "declarative" },
        "assets": [],
        "source": []
    })
}

pub(super) fn command_import(app: tauri::AppHandle) -> Result<Option<ImportResult>, Error> {
    let Some(path) = open_dialog(&app) else {
        return Ok(None);
    };
    let metadata = fs::metadata(&path).map_err(|error| Error::new("storage", error))?;
    if metadata.len() == 0 || metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(Error::new(
            "validation",
            format!("A plugin archive must be between 1 byte and {MAX_ARCHIVE_BYTES} bytes."),
        ));
    }
    let bytes = fs::read(&path).map_err(|error| Error::new("storage", error))?;
    let store = app.state::<Store>();
    let outcome = import_archive(&store, &bytes, "import")?;
    let document = store.snapshot()?.ok_or_else(|| {
        Error::new("not_initialized", "Customization is not initialized.")
    })?;
    super::publish(&app, &document.document);
    Ok(Some(ImportResult {
        plugin: outcome.plugin,
        already_installed: outcome.already_installed,
        replaced_version: outcome.replaced_version,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub plugin: PluginRecord,
    pub already_installed: bool,
    pub replaced_version: Option<String>,
}

pub(super) fn command_export(app: tauri::AppHandle, plugin_id: String) -> Result<Option<String>, Error> {
    let store = app.state::<Store>();
    let (name, bytes) = export_archive(&store, &plugin_id)?;
    let Some(target) = save_target(&app, &name) else {
        return Ok(None);
    };
    let target = if target.extension().is_none() {
        target.with_extension("zip")
    } else {
        target
    };
    fs::write(&target, bytes).map_err(|error| Error::new("storage", error))?;
    Ok(target.file_name().map(|name| name.to_string_lossy().to_string()))
}

pub(super) fn command_preview(
    store: &Store,
    plugin_id: &str,
    expected_revision: u64,
) -> Result<crate::customization::Preview, Error> {
    store.preview_plugin(plugin_id, expected_revision)
}

pub(super) fn command_enable_runtime(
    store: &Store,
    plugin_id: &str,
    expected_revision: u64,
    permissions: Vec<String>,
    network: Vec<String>,
    trigger: Trigger,
) -> Result<(), Error> {
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let current = Store::current(&transaction)?.ok_or_else(|| {
        Error::new("not_initialized", "Customization is not initialized.")
    })?;
    if current.revision != expected_revision {
        return Err(Error::new("conflict", "Customization changed elsewhere."));
    }
    let plugins = load_plugins(&transaction)?;
    let (_, plugin) = plugins
        .iter()
        .find(|(id, _)| id == plugin_id)
        .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
    if let Some(reason) = plugin.execution_reason() {
        return Err(Error::new("plugin_unsupported", reason));
    }
    if !plugin.manifest.runtime.kind.is_executable() {
        return Err(Error::new(
            "plugin_runtime",
            "Only executable plugins use a runtime grant. Declarative plugins are enabled with Preview and Keep.",
        ));
    }
    if plugin.enabled {
        return Err(Error::new(
            "plugin_enabled",
            "Disable this plugin before changing its runtime permissions.",
        ));
    }
    if !trigger.supports(plugin.manifest.runtime.kind) {
        return Err(Error::new(
            "plugin_trigger",
            format!(
                "The {} runtime does not support the {} trigger.",
                plugin.manifest.runtime.kind.label(),
                trigger.label()
            ),
        ));
    }
    let mut granted = BTreeSet::new();
    for label in &permissions {
        let permission = Permission::parse(label).ok_or_else(|| {
            Error::new(
                "denied",
                format!("Unknown runtime permission: {label}."),
            )
        })?;
        if !granted.insert(permission) {
            return Err(Error::new(
                "validation",
                format!("Duplicate runtime permission: {}.", permission.label()),
            ));
        }
    }
    let declared: BTreeSet<Permission> = plugin.manifest.permissions.iter().copied().collect();
    if !granted.is_subset(&declared) {
        return Err(Error::new(
            "denied",
            "The requested runtime permissions were not declared by this package.",
        ));
    }
    let mut seen_network = HashSet::new();
    for origin in &network {
        if !seen_network.insert(origin.clone()) {
            return Err(Error::new(
                "validation",
                format!("Duplicate network origin: {origin}."),
            ));
        }
    }
    let declared_network = plugin
        .manifest
        .network
        .as_ref()
        .map(|network| network.allow.as_slice())
        .unwrap_or(&[]);
    if !network.iter().all(|origin| declared_network.contains(origin)) {
        return Err(Error::new(
            "denied",
            "The requested network origins were not declared by this package.",
        ));
    }
    if !network.is_empty() && !granted.contains(&Permission::NetworkGet) {
        return Err(Error::new(
            "validation",
            "A network allowlist requires the network.get permission.",
        ));
    }
    for (id, candidate) in &plugins {
        if !candidate.enabled
            || candidate.execution_reason().is_some()
            || id == plugin_id
            || !candidate.manifest.runtime.kind.is_declarative()
        {
            continue;
        }
        for field in candidate.active_fields() {
            if plugin.declared_fields().contains(field) {
                return Err(Error::new(
                    "plugin_conflict",
                    format!(
                        "{} cannot be enabled because {field} is already controlled by {id}.",
                        plugin.manifest.name
                    ),
                ));
            }
        }
    }
    transaction.execute(
        "UPDATE plugins SET enabled = 1, detached = '[]', disabled_reason = NULL WHERE id = ?1",
        [plugin_id],
    )?;
    transaction.execute(
        "INSERT OR REPLACE INTO plugin_grants (plugin_id, version, permissions, network, trigger, granted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            plugin_id,
            plugin.manifest.version,
            serde_json::to_string(
                &granted
                    .iter()
                    .map(|permission| permission.label().to_string())
                    .collect::<Vec<_>>()
            )
            .map_err(|error| Error::new("plugin_storage", error))?,
            serde_json::to_string(&network)
                .map_err(|error| Error::new("plugin_storage", error))?,
            trigger.label(),
            Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        ],
    )?;
    apply(&transaction, &current.config)?;
    Store::write(&transaction, &current, current.config.clone(), true)?;
    transaction.commit()?;
    Ok(())
}

pub(super) fn command_disable(
    store: &Store,
    plugin_id: &str,
    expected_revision: u64,
) -> Result<(), Error> {
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let current = Store::current(&transaction)?.ok_or_else(|| {
        Error::new("not_initialized", "Customization is not initialized.")
    })?;
    if current.revision != expected_revision {
        return Err(Error::new("conflict", "Customization changed elsewhere."));
    }
    disable(&transaction, plugin_id)?;
    Store::write(&transaction, &current, current.config.clone(), true)?;
    transaction.commit()?;
    Ok(())
}

pub(super) fn command_remove(
    store: &Store,
    plugin_id: &str,
    expected_revision: u64,
    delete_data: bool,
) -> Result<(), Error> {
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let current = Store::current(&transaction)?.ok_or_else(|| {
        Error::new("not_initialized", "Customization is not initialized.")
    })?;
    if current.revision != expected_revision {
        return Err(Error::new("conflict", "Customization changed elsewhere."));
    }
    let row: Option<(String, i64)> = transaction
        .query_row(
            "SELECT manifest, enabled FROM plugins WHERE id = ?1",
            [plugin_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((raw, _enabled)) = row else {
        return Err(Error::new("plugin_missing", "That plugin is not installed."));
    };
    let manifest: Manifest = serde_json::from_str(&raw)
        .map_err(|error| Error::new("plugin_storage", error))?;
    remove_row(&transaction, plugin_id, delete_data)?;
    Store::write(&transaction, &current, current.config.clone(), true)?;
    transaction.commit()?;
    delete_package(store, plugin_id, &manifest.version);
    Ok(())
}

pub(super) fn command_save_settings(
    store: &Store,
    plugin_id: &str,
    expected_revision: u64,
    values: Map<String, Value>,
) -> Result<(), Error> {
    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let current = Store::current(&transaction)?.ok_or_else(|| {
        Error::new("not_initialized", "Customization is not initialized.")
    })?;
    if current.revision != expected_revision {
        return Err(Error::new("conflict", "Customization changed elsewhere."));
    }
    save_settings(&transaction, plugin_id, &values)?;
    Store::write(&transaction, &current, current.config.clone(), true)?;
    transaction.commit()?;
    Ok(())
}

pub(super) fn snapshot_for_connection(
    connection: &Connection,
    document: &Document,
) -> Result<Document, Error> {
    Ok(Document {
        version: 3,
        revision: document.revision,
        config: apply(connection, &document.config)?,
    })
}

#[cfg(test)]
mod tests {
    use super::super::Commit;
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "luma-plugin-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn store() -> (Store, PathBuf) {
        let root = root();
        let store = Store::from_connection_with_root(
            Connection::open_in_memory().unwrap(),
            Some(root.clone()),
        )
        .unwrap();
        store.initialize(Config::default()).unwrap();
        (store, root)
    }

    fn manifest(id: &str, version: &str) -> Value {
        serde_json::json!({
            "id": id,
            "name": "Cinema Lounge",
            "version": version,
            "apiVersion": API_VERSION,
            "description": "Warm appearance with controllable Home rows.",
            "author": "Test",
            "settings": [
                {
                    "key": "theme",
                    "label": "Theme",
                    "description": "Palette used by the pack.",
                    "type": "enum",
                    "default": "ember",
                    "options": [
                        { "value": "ember", "label": "Ember" },
                        { "value": "onyx", "label": "Onyx" }
                    ]
                },
                {
                    "key": "showFeatured",
                    "label": "Show featured",
                    "type": "boolean",
                    "default": true
                },
                {
                    "key": "columns",
                    "label": "Columns",
                    "type": "number",
                    "default": 2,
                    "min": 1,
                    "max": 2,
                    "integer": true
                },
                {
                    "key": "label",
                    "label": "Label",
                    "type": "string",
                    "default": "Home",
                    "maxLength": 20
                }
            ],
            "contributes": {
                "theme": { "setting": "theme" },
                "homeColumns": { "setting": "columns" },
                "home": {
                    "rows": {
                        "featured": { "visible": { "setting": "showFeatured" } }
                    }
                }
            },
            "runtime": { "kind": "declarative" },
            "assets": [],
            "source": []
        })
    }

    fn submission(id: &str, version: &str) -> Value {
        serde_json::json!({ "manifest": manifest(id, version), "files": [] })
    }

    fn stage(store: &Store, id: &str, version: &str) -> PluginRecord {
        stage_submission(store, &submission(id, version), "mcp")
            .unwrap()
            .plugin
    }

    fn enable(store: &Store, id: &str) -> Document {
        let revision = store.read().unwrap().unwrap().revision;
        let preview = store.preview_plugin(id, revision).unwrap();
        store.keep(&preview.id).unwrap()
    }

    fn enable_runtime(
        store: &Store,
        id: &str,
        permissions: &[&str],
        network: &[&str],
        trigger: Trigger,
    ) {
        let revision = store.read().unwrap().unwrap().revision;
        command_enable_runtime(
            store,
            id,
            revision,
            permissions.iter().map(|value| value.to_string()).collect(),
            network.iter().map(|value| value.to_string()).collect(),
            trigger,
        )
        .unwrap();
    }

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn staged_plugins_are_disabled_and_have_no_activation_tool() {
        let (store, _root) = store();
        let record = stage(&store, "com.example.lounge", "1.0.0");
        assert!(!record.enabled);
        assert!(record.can_enable);
        assert!(record.controls.contains(&"theme".into()));
        assert_eq!(record.settings.len(), 4);
        assert_eq!(record.settings[0].value, serde_json::json!("ember"));
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config, Config::default());
        for name in [
            "luma_plugin_enable",
            "luma_plugin_keep",
            "luma_plugin_disable",
            "luma_plugin_remove",
            "luma_plugin_export",
        ] {
            assert!(!super::super::mcp::known_tool_for_test(name));
        }
        assert!(super::super::mcp::known_tool_for_test("luma_plugin_stage"));
        assert!(super::super::mcp::known_tool_for_test("luma_plugin_settings_stage"));
    }

    #[test]
    fn enable_disable_and_settings_preserve_the_underlying_configuration() {
        let (store, _root) = store();
        let original = store.read().unwrap().unwrap();
        stage(&store, "com.example.lounge", "1.0.0");
        let enabled = enable(&store, "com.example.lounge");
        assert!(enabled.revision > original.revision);
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config.theme, "ember");
        assert_eq!(effective.config.home_columns, 2);
        assert!(effective.config.home.iter().find(|row| row.component == "featured").unwrap().visible);

        let revision = store.read().unwrap().unwrap().revision;
        command_save_settings(
            &store,
            "com.example.lounge",
            revision,
            map(serde_json::json!({ "showFeatured": false, "columns": 1, "theme": "onyx" })),
        )
        .unwrap();
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config.theme, "onyx");
        assert_eq!(effective.config.home_columns, 1);
        assert!(!effective.config.home.iter().find(|row| row.component == "featured").unwrap().visible);

        let revision = store.read().unwrap().unwrap().revision;
        command_disable(&store, "com.example.lounge", revision).unwrap();
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config, original.config);
        let state = state_for_store(&store);
        assert!(!state.plugins[0].enabled);
        assert_eq!(state.plugins[0].settings[0].value, serde_json::json!("onyx"));
        enable(&store, "com.example.lounge");
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config.theme, "onyx");
        assert_eq!(effective.config.home_columns, 1);
        assert!(!effective.config.home.iter().find(|row| row.component == "featured").unwrap().visible);
    }

    fn state_for_store(store: &Store) -> PluginsState {
        let document = store.read().unwrap().unwrap();
        let connection = store.connection().unwrap();
        state(&connection, &document.config).unwrap()
    }

    #[test]
    fn manual_edits_detach_only_the_changed_field_and_keep_the_plugin_enabled() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let mut unrelated = store.read_effective().unwrap().unwrap().config;
        unrelated.interface_sounds = "roku".into();
        let revision = store.read().unwrap().unwrap().revision;
        store
            .commit(Commit {
                expected_revision: revision,
                config: unrelated,
            })
            .unwrap();
        let state = state_for_store(&store);
        assert!(state.plugins[0].controls.contains(&"theme".into()));
        assert!(state.plugins[0].detached_fields.is_empty());
        assert_eq!(store.read_effective().unwrap().unwrap().config.theme, "ember");
        let mut requested = store.read_effective().unwrap().unwrap().config;
        requested.theme = "sapphire".into();
        let revision = store.read().unwrap().unwrap().revision;
        let document = store
            .commit(Commit {
                expected_revision: revision,
                config: requested,
            })
            .unwrap();
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config.theme, "sapphire");
        assert_eq!(effective.config.home_columns, 2);
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert!(!state.plugins[0].controls.contains(&"theme".into()));
        assert!(state.plugins[0].controls.contains(&"homeColumns".into()));
        assert!(state.plugins[0].detached_fields.contains(&"theme".into()));
        assert_eq!(document.config.theme, "sapphire");
    }

    #[test]
    fn preview_reports_fields_that_keep_will_detach_from_a_plugin() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let mut requested = store.read_effective().unwrap().unwrap().config;
        requested.theme = "sapphire".into();
        let revision = store.read().unwrap().unwrap().revision;
        let preview = store
            .stage(Commit {
                expected_revision: revision,
                config: requested,
            })
            .unwrap();
        assert_eq!(preview.detached, vec!["theme"]);
        store.keep(&preview.id).unwrap();
        let state = state_for_store(&store);
        assert!(state.plugins[0].detached_fields.contains(&"theme".into()));
        assert_eq!(store.read_effective().unwrap().unwrap().config.theme, "sapphire");
    }

    #[test]
    fn conflicting_enabled_field_ownership_is_rejected_explicitly() {
        let (store, _root) = store();
        stage(&store, "com.example.one", "1.0.0");
        let mut second = manifest("com.example.two", "1.0.0");
        second["name"] = serde_json::json!("Second Lounge");
        stage_submission(
            &store,
            &serde_json::json!({ "manifest": second, "files": [] }),
            "mcp",
        )
        .unwrap();
        enable(&store, "com.example.one");
        let revision = store.read().unwrap().unwrap().revision;
        let error = store
            .preview_plugin("com.example.two", revision)
            .unwrap_err();
        assert_eq!(error.code, "plugin_conflict");
        let state = state_for_store(&store);
        let second = state.plugins.iter().find(|plugin| plugin.id == "com.example.two").unwrap();
        assert!(!second.enabled);
        assert!(!second.conflicts.is_empty());
    }

    #[test]
    fn executable_runtime_packages_install_disabled_and_offer_a_grant_review() {
        let (store, _root) = store();
        let mut code = manifest("com.example.code", "1.0.0");
        code["apiVersion"] = serde_json::json!(EXECUTABLE_API_VERSION);
        code["contributes"] = serde_json::json!({});
        code["permissions"] = serde_json::json!(["settings.read"]);
        code["runtime"] = serde_json::json!({ "kind": "isolated-ui", "entry": "ui/index.html" });
        let outcome = stage_submission(
            &store,
            &serde_json::json!({
                "manifest": code,
                "files": [
                    { "path": "ui/index.html", "text": "<!doctype html><title>Not run</title>" },
                    { "path": "ui/main.js", "text": "console.log('not run')" }
                ]
            }),
            "mcp",
        )
        .unwrap();
        assert!(!outcome.plugin.enabled);
        assert!(outcome.plugin.executable);
        assert!(outcome.plugin.can_enable);
        assert!(outcome.plugin.execution_reason.is_none());
        assert!(!outcome.plugin.surface_openable);
        assert!(outcome.plugin.granted_permissions.is_empty());
        assert_eq!(outcome.plugin.permissions, ["settings.read"]);
        assert_eq!(outcome.plugin.entry.as_deref(), Some("ui/index.html"));
        let revision = store.read().unwrap().unwrap().revision;
        let error = store.preview_plugin("com.example.code", revision).unwrap_err();
        assert_eq!(error.code, "plugin_runtime");
        assert_eq!(
            runtime_package(&store, "com.example.code").unwrap_err().code,
            "plugin_disabled"
        );
    }

    #[test]
    fn runtime_grants_are_explicit_bounded_and_required_before_execution() {
        let (store, _root) = store();
        let mut code = manifest("com.example.code", "1.0.0");
        code["apiVersion"] = serde_json::json!(EXECUTABLE_API_VERSION);
        code["contributes"] = serde_json::json!({});
        code["permissions"] = serde_json::json!(["settings.read", "network.get"]);
        code["network"] = serde_json::json!({ "allow": ["https://api.open-meteo.com"] });
        code["runtime"] = serde_json::json!({ "kind": "isolated-ui", "entry": "ui/index.html" });
        stage_submission(
            &store,
            &serde_json::json!({
                "manifest": code,
                "files": [{ "path": "ui/index.html", "text": "<!doctype html>" }]
            }),
            "mcp",
        )
        .unwrap();

        let revision = store.read().unwrap().unwrap().revision;
        assert_eq!(
            command_enable_runtime(
                &store,
                "com.example.code",
                revision,
                vec!["playback.read".into()],
                vec![],
                Trigger::LauncherOpen,
            )
            .unwrap_err()
            .code,
            "denied"
        );
        assert_eq!(
            command_enable_runtime(
                &store,
                "com.example.code",
                revision,
                vec!["network.get".into()],
                vec!["https://example.com".into()],
                Trigger::LauncherOpen,
            )
            .unwrap_err()
            .code,
            "denied"
        );
        assert_eq!(
            command_enable_runtime(
                &store,
                "com.example.code",
                revision,
                vec!["settings.read".into()],
                vec![],
                Trigger::Startup,
            )
            .unwrap_err()
            .code,
            "plugin_trigger"
        );
        command_enable_runtime(
            &store,
            "com.example.code",
            revision,
            vec!["settings.read".into(), "network.get".into()],
            vec!["https://api.open-meteo.com".into()],
            Trigger::LauncherOpen,
        )
        .unwrap();
        let state = state_for_store(&store);
        let record = &state.plugins[0];
        assert!(record.enabled);
        assert!(record.surface_openable);
        assert_eq!(record.granted_permissions, ["settings.read", "network.get"]);
        assert_eq!(record.granted_network, ["https://api.open-meteo.com"]);
        assert_eq!(record.trigger.as_deref(), Some("launcher-open"));
        let package = runtime_package(&store, "com.example.code").unwrap();
        assert!(package.allows(Permission::SettingsRead));
        assert!(package.allows(Permission::NetworkGet));
        assert_eq!(package.network, ["https://api.open-meteo.com"]);

        let revision = store.read().unwrap().unwrap().revision;
        command_disable(&store, "com.example.code", revision).unwrap();
        let state = state_for_store(&store);
        assert!(!state.plugins[0].enabled);
        assert!(!state.plugins[0].surface_openable);
        assert!(state.plugins[0].granted_permissions.is_empty());
        assert!(state.plugins[0].trigger.is_none());
    }

    #[test]
    fn background_runtime_accepts_startup_and_always_triggers() {
        let (store, _root) = store();
        let mut service = manifest("com.example.service", "1.0.0");
        service["apiVersion"] = serde_json::json!(EXECUTABLE_API_VERSION);
        service["contributes"] = serde_json::json!({});
        service["permissions"] = serde_json::json!(["settings.read"]);
        service["runtime"] = serde_json::json!({ "kind": "background", "entry": "worker.html" });
        stage_submission(
            &store,
            &serde_json::json!({
                "manifest": service,
                "files": [{ "path": "worker.html", "text": "<!doctype html>" }]
            }),
            "import",
        )
        .unwrap();
        assert!(state_for_store(&store).plugins[0].can_enable);
        enable_runtime(
            &store,
            "com.example.service",
            &["settings.read"],
            &[],
            Trigger::Always,
        );
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert_eq!(state.plugins[0].runtime, "background");
        assert!(!state.plugins[0].surface_openable);
        assert_eq!(state.plugins[0].trigger.as_deref(), Some("always"));
        assert_eq!(
            granted_plugins(&store).unwrap()[0].1.trigger,
            Trigger::Always
        );
    }

    #[test]
    fn executable_manifests_require_a_versioned_bounded_grant() {
        let (store, _root) = store();
        let runtime = |manifest: &mut Value| {
            manifest["apiVersion"] = serde_json::json!(EXECUTABLE_API_VERSION);
            manifest["contributes"] = serde_json::json!({});
            manifest["runtime"] = serde_json::json!({ "kind": "isolated-ui", "entry": "ui/index.html" });
        };
        let files = serde_json::json!([{ "path": "ui/index.html", "text": "<!doctype html>" }]);
        let stage_with = |manifest: Value| {
            stage_submission(
                &store,
                &serde_json::json!({ "manifest": manifest, "files": files }),
                "mcp",
            )
        };

        let mut undeclared_network = manifest("com.example.network", "1.0.0");
        runtime(&mut undeclared_network);
        undeclared_network["network"] = serde_json::json!({ "allow": ["https://api.open-meteo.com"] });
        assert_eq!(stage_with(undeclared_network).unwrap_err().code, "validation");

        let mut missing_network = manifest("com.example.missing-network", "1.0.0");
        runtime(&mut missing_network);
        missing_network["permissions"] = serde_json::json!(["network.get"]);
        assert_eq!(stage_with(missing_network).unwrap_err().code, "validation");

        for origin in [
            "http://api.open-meteo.com",
            "https://127.0.0.1",
            "https://localhost",
            "https://printer.local",
            "https://api.open-meteo.com:8443",
            "https://api.open-meteo.com/path",
        ] {
            let mut insecure = manifest("com.example.origin", "1.0.0");
            runtime(&mut insecure);
            insecure["permissions"] = serde_json::json!(["network.get"]);
            insecure["network"] = serde_json::json!({ "allow": [origin] });
            assert_eq!(
                stage_with(insecure).unwrap_err().code,
                "validation",
                "{origin} must not be an allowed network origin"
            );
        }

        let mut unknown_permission = manifest("com.example.permission", "1.0.0");
        runtime(&mut unknown_permission);
        unknown_permission["permissions"] = serde_json::json!(["network.any"]);
        assert_eq!(stage_with(unknown_permission).unwrap_err().code, "validation");

        let mut playback = manifest("com.example.playback", "1.0.0");
        runtime(&mut playback);
        playback["permissions"] = serde_json::json!(["playback.read", "playback.control"]);
        assert!(stage_with(playback).is_ok());
        enable_runtime(
            &store,
            "com.example.playback",
            &["playback.read", "playback.control"],
            &[],
            Trigger::LauncherOpen,
        );
        let package = runtime_package(&store, "com.example.playback").unwrap();
        assert!(package.allows(Permission::PlaybackRead));
        assert!(package.allows(Permission::PlaybackControl));
        assert!(!package.allows(Permission::NetworkGet));
        assert!(package.network.is_empty());

        let mut v1_grants = manifest("com.example.v1", "1.0.0");
        v1_grants["permissions"] = serde_json::json!(["settings.read"]);
        assert_eq!(stage_with(v1_grants).unwrap_err().code, "validation");
    }

    #[test]
    fn the_weather_example_installs_as_a_runtime_package_with_public_grants() {
        let (store, _root) = store();
        let submission = serde_json::json!({
            "manifest": serde_json::from_str::<Value>(include_str!(
                "../../../examples/plugins/com.example.weather/luma-extension.json"
            ))
            .unwrap(),
            "files": [
                {
                    "path": "ui/index.html",
                    "text": include_str!("../../../examples/plugins/com.example.weather/ui/index.html")
                },
                {
                    "path": "ui/page.js",
                    "text": include_str!("../../../examples/plugins/com.example.weather/ui/page.js")
                },
                {
                    "path": "ui/page.css",
                    "text": include_str!("../../../examples/plugins/com.example.weather/ui/page.css")
                }
            ]
        });
        let outcome = stage_submission(&store, &submission, "import").unwrap();
        assert_eq!(outcome.plugin.id, "com.example.weather");
        assert_eq!(outcome.plugin.api_version, EXECUTABLE_API_VERSION);
        assert_eq!(outcome.plugin.runtime, "isolated-ui");
        assert!(!outcome.plugin.enabled);
        assert!(outcome.plugin.executable);
        assert!(outcome.plugin.can_enable);
        assert_eq!(
            outcome.plugin.permissions,
            ["settings.read", "network.get"]
        );
        assert_eq!(outcome.plugin.network, ["https://api.open-meteo.com"]);
        assert!(granted_plugins(&store).unwrap().is_empty());

        enable_runtime(
            &store,
            "com.example.weather",
            &["settings.read", "network.get"],
            &["https://api.open-meteo.com"],
            Trigger::LauncherOpen,
        );
        assert_eq!(
            granted_plugins(&store).unwrap()[0].0,
            "com.example.weather"
        );

        let package = runtime_package(&store, "com.example.weather").unwrap();
        assert_eq!(package.api_version, EXECUTABLE_API_VERSION);
        assert_eq!(package.entry, "ui/index.html");
        assert!(package.allows(Permission::SettingsRead));
        assert!(package.allows(Permission::NetworkGet));
        assert!(!package.allows(Permission::PlaybackRead));
        assert_eq!(package.network, ["https://api.open-meteo.com"]);
        assert_eq!(package.settings.get("location"), Some(&serde_json::json!("seattle")));
        assert_eq!(package.settings.get("units"), Some(&serde_json::json!("c")));
        assert!(String::from_utf8(package.read("").unwrap())
            .unwrap()
            .starts_with("<!doctype html>"));
        assert!(String::from_utf8(package.read("ui/page.js").unwrap())
            .unwrap()
            .contains("extension_bridge"));
        assert_eq!(package.read("../luma-extension.json").unwrap_err().code, "validation");
        assert_eq!(package.read("ui/missing.js").unwrap_err().code, "plugin_missing");
    }

    #[test]
    fn compatible_updates_keep_the_runtime_grant_and_expansion_clears_it() {
        let (store, _root) = store();
        let package = |version: &str, permissions: Value| {
            serde_json::json!({
                "manifest": {
                    "id": "com.example.update",
                    "name": "Updating runtime",
                    "version": version,
                    "apiVersion": EXECUTABLE_API_VERSION,
                    "contributes": {},
                    "permissions": permissions,
                    "runtime": { "kind": "isolated-ui", "entry": "ui/index.html" }
                },
                "files": [{ "path": "ui/index.html", "text": format!("<!doctype html><p>{version}</p>") }]
            })
        };
        stage_submission(
            &store,
            &package("1.0.0", serde_json::json!(["settings.read"])),
            "import",
        )
        .unwrap();
        enable_runtime(
            &store,
            "com.example.update",
            &["settings.read"],
            &[],
            Trigger::LauncherOpen,
        );

        let outcome = stage_submission(
            &store,
            &package("1.1.0", serde_json::json!(["settings.read"])),
            "import",
        )
        .unwrap();
        assert!(!outcome.plugin.enabled);
        assert_eq!(outcome.plugin.granted_permissions, ["settings.read"]);
        assert_eq!(outcome.plugin.trigger.as_deref(), Some("launcher-open"));

        let outcome = stage_submission(
            &store,
            &package(
                "1.2.0",
                serde_json::json!(["settings.read", "playback.read"]),
            ),
            "import",
        )
        .unwrap();
        assert!(!outcome.plugin.enabled);
        assert!(outcome.plugin.granted_permissions.is_empty());
        assert!(outcome.plugin.trigger.is_none());
        assert!(outcome.plugin.can_enable);
    }

    #[test]
    fn an_enabled_executable_without_a_grant_is_disabled_at_startup() {
        let (store, _root) = store();
        let mut code = manifest("com.example.orphan", "1.0.0");
        code["apiVersion"] = serde_json::json!(EXECUTABLE_API_VERSION);
        code["contributes"] = serde_json::json!({});
        code["permissions"] = serde_json::json!(["settings.read"]);
        code["runtime"] = serde_json::json!({ "kind": "isolated-ui", "entry": "ui/index.html" });
        stage_submission(
            &store,
            &serde_json::json!({
                "manifest": code,
                "files": [{ "path": "ui/index.html", "text": "<!doctype html>" }]
            }),
            "import",
        )
        .unwrap();
        enable_runtime(
            &store,
            "com.example.orphan",
            &["settings.read"],
            &[],
            Trigger::LauncherOpen,
        );
        store
            .connection()
            .unwrap()
            .execute("DELETE FROM plugin_grants WHERE plugin_id = ?1", ["com.example.orphan"])
            .unwrap();
        reconcile(&store.connection().unwrap()).unwrap();
        let state = state_for_store(&store);
        assert!(!state.plugins[0].enabled);
        assert!(!state.plugins[0].surface_openable);
        assert!(state.plugins[0]
            .disabled_reason
            .as_deref()
            .unwrap()
            .contains("grant is missing"));
    }

    #[test]
    fn runtime_trigger_names_match_the_command_contract() {
        for (json, trigger) in [
            ("\"launcher-open\"", Trigger::LauncherOpen),
            ("\"startup\"", Trigger::Startup),
            ("\"always\"", Trigger::Always),
        ] {
            assert_eq!(serde_json::from_str::<Trigger>(json).unwrap(), trigger);
        }
        assert!(serde_json::from_str::<Trigger>("\"on-demand\"").is_err());
        assert!(Trigger::LauncherOpen.supports(RuntimeKind::IsolatedUi));
        assert!(!Trigger::Startup.supports(RuntimeKind::IsolatedUi));
        assert!(Trigger::Always.supports(RuntimeKind::Background));
        assert!(!Trigger::Always.supports(RuntimeKind::Declarative));
    }

    #[test]
    fn the_music_remote_example_projects_an_enabled_panel() {
        let (store, _root) = store();
        let manifest: Value = serde_json::from_str(include_str!(
            "../../../examples/plugins/com.example.music-remote/luma-extension.json"
        ))
        .unwrap();
        let outcome = stage_submission(
            &store,
            &serde_json::json!({ "manifest": manifest, "files": [] }),
            "import",
        )
        .unwrap();
        assert!(!outcome.plugin.enabled);
        assert!(outcome.plugin.can_enable);
        assert!(remote_panels_for(&store.connection().unwrap()).unwrap().is_empty());

        enable(&store, "com.example.music-remote");
        let panels = remote_panels_for(&store.connection().unwrap()).unwrap();
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].id, "com.example.music-remote");
        assert_eq!(panels[0].title, "Music controls");
        assert_eq!(
            panels[0]
                .actions
                .iter()
                .map(|action| action.label())
                .collect::<Vec<_>>(),
            ["playback.previous", "playback.toggle", "playback.next"]
        );
    }

    #[test]
    fn remote_panels_project_only_enabled_declarative_packages_and_recheck_actions() {
        let (store, _root) = store();
        let mut panel = manifest("com.example.remote", "1.0.0");
        panel["contributes"] = serde_json::json!({});
        panel["remote"] = serde_json::json!({
            "title": "  Lounge controls  ",
            "actions": ["playback.toggle", "playback.next"]
        });
        stage_submission(
            &store,
            &serde_json::json!({ "manifest": panel, "files": [] }),
            "import",
        )
        .unwrap();
        assert!(remote_panels_for(&store.connection().unwrap()).unwrap().is_empty());

        enable(&store, "com.example.remote");
        let panels = remote_panels_for(&store.connection().unwrap()).unwrap();
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].title, "Lounge controls");
        assert_eq!(
            panels[0]
                .actions
                .iter()
                .map(|action| action.label())
                .collect::<Vec<_>>(),
            ["playback.toggle", "playback.next"]
        );
        assert_eq!(
            remote_action_for(&store.connection().unwrap(), "com.example.remote", "playback.next")
                .unwrap(),
            RemoteAction::PlaybackNext
        );
        assert_eq!(
            remote_action_for(
                &store.connection().unwrap(),
                "com.example.remote",
                "playback.previous"
            )
            .unwrap_err()
            .code,
            "denied"
        );
        assert_eq!(
            remote_action_for(&store.connection().unwrap(), "com.example.missing", "playback.next")
                .unwrap_err()
                .code,
            "plugin_missing"
        );

        let revision = store.read().unwrap().unwrap().revision;
        command_disable(&store, "com.example.remote", revision).unwrap();
        assert!(remote_panels_for(&store.connection().unwrap()).unwrap().is_empty());
        assert_eq!(
            remote_action_for(&store.connection().unwrap(), "com.example.remote", "playback.next")
                .unwrap_err()
                .code,
            "plugin_disabled"
        );
    }

    #[test]
    fn remote_manifest_shape_is_bounded_and_executable_packages_cannot_declare_panels() {
        let (store, _root) = store();
        let mut duplicate = manifest("com.example.duplicate", "1.0.0");
        duplicate["remote"] = serde_json::json!({
            "actions": ["playback.next", "playback.next"]
        });
        assert_eq!(
            stage_submission(
                &store,
                &serde_json::json!({ "manifest": duplicate, "files": [] }),
                "import"
            )
            .unwrap_err()
            .code,
            "validation"
        );

        let mut executable = manifest("com.example.executable", "1.0.0");
        executable["apiVersion"] = serde_json::json!(EXECUTABLE_API_VERSION);
        executable["contributes"] = serde_json::json!({});
        executable["runtime"] = serde_json::json!({ "kind": "background", "entry": "worker.html" });
        executable["remote"] = serde_json::json!({ "actions": [] });
        assert_eq!(
            stage_submission(
                &store,
                &serde_json::json!({
                    "manifest": executable,
                    "files": [{ "path": "worker.html", "text": "<!doctype html>" }]
                }),
                "import"
            )
            .unwrap_err()
            .code,
            "validation"
        );

        let mut long_title = manifest("com.example.title", "1.0.0");
        long_title["remote"] = serde_json::json!({ "title": "x".repeat(49) });
        assert_eq!(
            stage_submission(
                &store,
                &serde_json::json!({ "manifest": long_title, "files": [] }),
                "import"
            )
            .unwrap_err()
            .code,
            "validation"
        );
    }


    #[test]
    fn mcp_settings_stage_uses_the_same_values_and_requires_keep() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let revision = store.read().unwrap().unwrap().revision;
        command_save_settings(
            &store,
            "com.example.lounge",
            revision,
            map(serde_json::json!({ "theme": "onyx" })),
        )
        .unwrap();
        let revision = store.read().unwrap().unwrap().revision;
        let values = BTreeMap::from([("showFeatured".to_string(), serde_json::json!(false))]);
        let preview = store
            .preview_plugin_settings("com.example.lounge", revision, &values)
            .unwrap();
        assert_eq!(preview.plugin.as_ref().unwrap().action, "settings");
        assert!(!preview.config.home.iter().find(|row| row.component == "featured").unwrap().visible);
        let kept = store.keep(&preview.id).unwrap();
        assert_eq!(kept.revision, revision + 1);
        let state = state_for_store(&store);
        assert_eq!(state.plugins[0].settings[0].value, serde_json::json!("onyx"));
        assert_eq!(state.plugins[0].settings[1].value, serde_json::json!(false));
    }

    #[test]
    fn plugin_state_and_settings_survive_reopening_storage() {
        let root = root();
        let path = root.join("customization.sqlite3");
        let revision;
        {
            let store =
                Store::from_connection_with_root(Connection::open(&path).unwrap(), Some(root.clone()))
                    .unwrap();
            store.initialize(Config::default()).unwrap();
            stage(&store, "com.example.lounge", "1.0.0");
            enable(&store, "com.example.lounge");
            revision = store.read().unwrap().unwrap().revision;
            command_save_settings(
                &store,
                "com.example.lounge",
                revision,
                map(serde_json::json!({ "showFeatured": false })),
            )
            .unwrap();
        }
        let store =
            Store::from_connection_with_root(Connection::open(&path).unwrap(), Some(root.clone()))
                .unwrap();
        let document = store.read_effective().unwrap().unwrap();
        assert!(document.revision > revision);
        assert_eq!(document.config.theme, "ember");
        assert!(!document.config.home.iter().find(|row| row.component == "featured").unwrap().visible);
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert_eq!(state.plugins[0].settings[1].value, serde_json::json!(false));
    }

    #[test]
    fn history_restore_restores_enabled_state_settings_and_detach_state() {
        let (store, _root) = store();
        let first = store.read().unwrap().unwrap();
        stage(&store, "com.example.lounge", "1.0.0");
        let enabled = enable(&store, "com.example.lounge");
        assert!(enabled.revision > first.revision);
        let revision = store.read().unwrap().unwrap().revision;
        command_save_settings(
            &store,
            "com.example.lounge",
            revision,
            map(serde_json::json!({ "showFeatured": false })),
        )
        .unwrap();
        let current = store.read().unwrap().unwrap();
        let (restored, notices) = store
            .restore_version(current.revision, first.revision)
            .unwrap();
        assert_eq!(restored.config, first.config);
        assert!(!notices.is_empty());
        let state = state_for_store(&store);
        assert!(state.plugins.is_empty() || !state.plugins[0].enabled);
        let (restored, _) = store
            .restore_version(restored.revision, enabled.revision)
            .unwrap();
        assert_eq!(restored.config, enabled.config);
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert_eq!(state.plugins[0].settings[1].value, serde_json::json!(true));
    }

    #[test]
    fn history_restore_keeps_field_detachment_explicit() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        let enabled = enable(&store, "com.example.lounge");
        let mut requested = store.read_effective().unwrap().unwrap().config;
        requested.theme = "sapphire".into();
        let revision = store.read().unwrap().unwrap().revision;
        store
            .commit(Commit {
                expected_revision: revision,
                config: requested,
            })
            .unwrap();
        let detached = store.read().unwrap().unwrap();
        assert!(state_for_store(&store).plugins[0]
            .detached_fields
            .contains(&"theme".into()));
        let (restored, _) = store
            .restore_version(detached.revision, enabled.revision)
            .unwrap();
        assert_eq!(restored.config, enabled.config);
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert!(state.plugins[0].detached_fields.is_empty());
        assert_eq!(store.read_effective().unwrap().unwrap().config.theme, "ember");
        let (restored, _) = store
            .restore_version(restored.revision, detached.revision)
            .unwrap();
        assert_eq!(restored.config.theme, "sapphire");
        assert!(state_for_store(&store).plugins[0]
            .detached_fields
            .contains(&"theme".into()));
    }

    #[test]
    fn reset_appearance_detaches_plugin_controls_and_keeps_settings() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let revision = store.read().unwrap().unwrap().revision;
        command_save_settings(
            &store,
            "com.example.lounge",
            revision,
            map(serde_json::json!({ "showFeatured": false, "theme": "onyx" })),
        )
        .unwrap();
        let revision = store.read().unwrap().unwrap().revision;
        store.reset_appearance(revision).unwrap();
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config, Config::default());
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert!(state.plugins[0].detached_fields.contains(&"theme".into()));
        assert_eq!(state.plugins[0].settings[1].value, serde_json::json!(false));
        assert_eq!(state.plugins[0].settings[0].value, serde_json::json!("onyx"));
    }

    #[test]
    fn removal_stops_contributions_and_missing_history_packages_do_not_look_enabled() {
        let (store, root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        let enabled = enable(&store, "com.example.lounge");
        let revision = store.read().unwrap().unwrap().revision;
        command_remove(&store, "com.example.lounge", revision, false).unwrap();
        assert_eq!(store.read_effective().unwrap().unwrap().config, Config::default());
        assert!(!root.join("com.example.lounge").join("1.0.0").exists());
        let current = store.read().unwrap().unwrap();
        let (restored, notices) = store
            .restore_version(current.revision, enabled.revision)
            .unwrap();
        assert_eq!(restored.config, enabled.config);
        assert!(!notices.is_empty());
        assert!(state_for_store(&store).plugins.is_empty());
    }

    #[test]
    fn reinstall_keeps_settings_only_when_removal_retains_data() {
        let (store, _root) = store();
        let id = "com.example.lounge";
        stage(&store, id, "1.0.0");
        let revision = store.read().unwrap().unwrap().revision;
        command_save_settings(&store, id, revision, map(serde_json::json!({
            "theme": "onyx", "showFeatured": false, "columns": 2, "label": "Lounge"
        }))).unwrap();
        let saved = settings_map(&store.connection().unwrap(), id).unwrap();
        let revision = store.read().unwrap().unwrap().revision;
        command_remove(&store, id, revision, false).unwrap();
        stage(&store, id, "1.0.0");
        let reinstalled = &state_for_store(&store).plugins[0];
        assert!(!reinstalled.enabled);
        assert_eq!(settings_map(&store.connection().unwrap(), id).unwrap(), saved);
        let revision = store.read().unwrap().unwrap().revision;
        command_remove(&store, id, revision, true).unwrap();
        stage(&store, id, "1.0.0");
        let reinstalled = &state_for_store(&store).plugins[0];
        assert!(reinstalled.settings.iter().all(|setting| setting.value == setting.default));
    }

    #[test]
    fn nonconflicting_plugins_coexist_and_disable_only_their_own_fields() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        let mut second = manifest("com.example.widgets", "1.0.0");
        second["name"] = serde_json::json!("Widget Pack");
        second["settings"] = serde_json::json!([]);
        second["contributes"] = serde_json::json!({
            "density": "compact",
            "widgets": { "spotifyEnabled": false, "spotifyPlacement": "bottom-left" }
        });
        stage_submission(
            &store,
            &serde_json::json!({ "manifest": second, "files": [] }),
            "mcp",
        )
        .unwrap();
        enable(&store, "com.example.lounge");
        enable(&store, "com.example.widgets");
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config.theme, "ember");
        assert_eq!(effective.config.home_columns, 2);
        assert_eq!(effective.config.density, "compact");
        assert!(!effective.config.widgets.spotify.enabled);
        assert_eq!(effective.config.widgets.spotify.placement, "bottom-left");
        let revision = store.read().unwrap().unwrap().revision;
        command_disable(&store, "com.example.widgets", revision).unwrap();
        let effective = store.read_effective().unwrap().unwrap();
        assert_eq!(effective.config.theme, "ember");
        assert_eq!(effective.config.home_columns, 2);
        assert_eq!(effective.config.density, "comfortable");
        assert!(effective.config.widgets.spotify.enabled);
    }

    #[test]
    fn mcp_cannot_replace_an_enabled_plugin_without_the_user_disabling_it() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let before = store.read_effective().unwrap().unwrap();
        let error = stage_submission(
            &store,
            &submission("com.example.lounge", "1.1.0"),
            "mcp",
        )
        .unwrap_err();
        assert_eq!(error.code, "plugin_enabled");
        let after = store.read_effective().unwrap().unwrap();
        assert_eq!(after, before);
        let state = state_for_store(&store);
        assert!(state.plugins[0].enabled);
        assert_eq!(state.plugins[0].version, "1.0.0");
    }

    #[test]
    fn failed_plugin_settings_write_preserves_values_and_effective_configuration() {
        let (store, _root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let revision = store.read().unwrap().unwrap().revision;
        command_save_settings(
            &store,
            "com.example.lounge",
            revision,
            map(serde_json::json!({ "theme": "onyx" })),
        )
        .unwrap();
        let before = store.read_effective().unwrap().unwrap();
        let revision = store.read().unwrap().unwrap().revision;
        store
            .connection()
            .unwrap()
            .execute_batch("CREATE TRIGGER reject_plugin_history BEFORE INSERT ON versions BEGIN SELECT RAISE(ABORT, 'disk full'); END;")
            .unwrap();
        assert_eq!(
            command_save_settings(
                &store,
                "com.example.lounge",
                revision,
                map(serde_json::json!({ "theme": "ember" })),
            )
            .unwrap_err()
            .code,
            "storage"
        );
        let after = store.read_effective().unwrap().unwrap();
        assert_eq!(after, before);
        let state = state_for_store(&store);
        assert_eq!(state.plugins[0].settings[0].value, serde_json::json!("onyx"));
    }

    #[test]
    fn archive_roundtrip_export_is_private_and_immutable() {
        let (store, _root) = store();
        let manifest_json = manifest("com.example.archive", "1.0.0");
        let files = BTreeMap::from([
            ("source/README.md".to_string(), b"Read me".to_vec()),
            ("assets/theme.txt".to_string(), b"ember".to_vec()),
        ]);
        let mut declared = manifest_json;
        declared["source"] = serde_json::json!(["source/README.md"]);
        declared["assets"] = serde_json::json!(["assets/theme.txt"]);
        let manifest: Manifest = serde_json::from_value(declared).unwrap();
        let bytes = archive::write(&manifest, &files).unwrap();
        let first = import_archive(&store, &bytes, "import").unwrap();
        assert!(!first.plugin.enabled);
        let second = import_archive(&store, &bytes, "import").unwrap();
        assert!(second.already_installed);
        drop(bytes);
        let (name, exported) = export_archive(&store, "com.example.archive").unwrap();
        assert_eq!(name, "com.example.archive-1.0.0.luma-extension.zip");
        let package = archive::read(&exported).unwrap();
        assert_eq!(package.files, files);
        assert_eq!(package.manifest.id, "com.example.archive");
        assert!(!String::from_utf8_lossy(&exported).contains("customization.sqlite3"));
        assert!(!String::from_utf8_lossy(&exported).contains("Users\\"));
        let mut changed = manifest;
        changed.name = "Changed".into();
        let changed_bytes = archive::write(&changed, &files).unwrap();
        assert_eq!(
            import_archive(&store, &changed_bytes, "import").unwrap_err().code,
            "immutable_version"
        );
    }

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn archive_accepts_one_top_level_folder_with_a_readable_manifest() {
        let mut manifest = manifest("com.example.folder", "1.0.0");
        manifest["source"] = serde_json::json!(["source/README.md"]);
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let bytes = zip_bytes(&[
            ("pack/luma-extension.json", manifest.as_slice()),
            ("pack/source/README.md", b"Read me"),
        ]);
        let package = archive::read(&bytes).unwrap();
        assert_eq!(package.manifest.id, "com.example.folder");
        assert!(package.files.contains_key("source/README.md"));
    }

    #[test]
    fn archive_validation_rejects_traversal_windows_paths_duplicates_and_symlinks() {
        let manifest = serde_json::to_vec(&manifest("com.example.bad", "1.0.0")).unwrap();
        assert!(archive::read(&zip_bytes(&[
            ("luma-extension.json", manifest.as_slice()),
            ("../evil.txt", b"x"),
        ]))
        .is_err());
        assert!(archive::read(&zip_bytes(&[
            ("luma-extension.json", manifest.as_slice()),
            ("folder\\evil.txt", b"x"),
        ]))
        .is_err());
        assert!(archive::read(&zip_bytes(&[
            ("luma-extension.json", manifest.as_slice()),
            ("A.txt", b"a"),
            ("a.txt", b"b"),
        ]))
        .is_err());
        let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file("luma-extension.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&manifest).unwrap();
        writer
            .add_symlink("link.txt", "target.txt", SimpleFileOptions::default())
            .unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert!(archive::read(&bytes).is_err());
    }

    #[test]
    fn archive_validation_rejects_missing_declared_files_and_compression_bombs() {
        let mut declared = manifest("com.example.missing", "1.0.0");
        declared["assets"] = serde_json::json!(["assets/icon.png"]);
        let missing_manifest = serde_json::to_vec(&declared).unwrap();
        assert!(archive::read(&zip_bytes(&[("luma-extension.json", missing_manifest.as_slice())])).is_err());

        let mut bomb = manifest("com.example.bomb", "1.0.0");
        bomb["source"] = serde_json::json!(["source/big.txt"]);
        let bomb_manifest = serde_json::to_vec(&bomb).unwrap();
        let big = vec![b'a'; 4 * 1024 * 1024];
        let bytes = zip_bytes(&[
            ("luma-extension.json", bomb_manifest.as_slice()),
            ("source/big.txt", big.as_slice()),
        ]);
        assert!(archive::read(&bytes).is_err());
    }

    #[test]
    fn archive_validation_rejects_too_many_files_and_oversized_files() {
        let count_manifest = serde_json::to_vec(&manifest("com.example.count", "1.0.0")).unwrap();
        let mut files = vec![("luma-extension.json".to_string(), count_manifest)];
        for index in 0..MAX_FILES + 1 {
            files.push((format!("source/file-{index}.txt"), b"x".to_vec()));
        }
        let entries: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect();
        assert!(archive::read(&zip_bytes(&entries)).is_err());

        let mut large = manifest("com.example.large", "1.0.0");
        large["source"] = serde_json::json!(["source/large.bin"]);
        let large_manifest = serde_json::to_vec(&large).unwrap();
        let oversized = vec![b'x'; (MAX_FILE_BYTES + 1) as usize];
        let bytes = zip_bytes(&[
            ("luma-extension.json", large_manifest.as_slice()),
            ("source/large.bin", oversized.as_slice()),
        ]);
        assert!(archive::read(&bytes).is_err());
    }

    #[test]
    fn failed_update_leaves_the_previous_enabled_package_intact() {
        let (store, root) = store();
        stage(&store, "com.example.lounge", "1.0.0");
        enable(&store, "com.example.lounge");
        let before = store.read_effective().unwrap().unwrap();
        let mut bad = manifest("com.example.lounge", "1.1.0");
        bad["assets"] = serde_json::json!(["assets/missing.png"]);
        let bytes = archive::write(
            &serde_json::from_value(bad).unwrap(),
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(import_archive(&store, &bytes, "import").is_err());
        let after = store.read_effective().unwrap().unwrap();
        assert_eq!(after.config, before.config);
        assert!(state_for_store(&store).plugins[0].enabled);
        assert!(root.join("com.example.lounge").join("1.0.0").is_dir());
    }
}

