pub mod document;
pub mod history;
pub mod mcp;
pub mod plugin;
pub mod server;

use document::{Config, Document};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};
use tauri::{Emitter, Manager};

#[derive(Debug, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}

impl Error {
    fn new(code: &'static str, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::new("storage", error)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Commit {
    pub expected_revision: u64,
    pub config: Config,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub base_revision: u64,
    pub config: Config,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<plugin::PluginPreview>,
    #[serde(skip)]
    pub prepared: Option<plugin::PreparedWrite>,
    #[serde(skip)]
    pub plugin_action: Option<plugin::PluginAction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewAction {
    pub preview_id: String,
}

#[derive(Clone)]
pub struct Snapshot {
    pub document: Document,
    pub plugins: plugin::PluginsState,
}

pub struct Store(
    Result<Mutex<Connection>, String>,
    Mutex<Option<Preview>>,
    Option<PathBuf>,
);

impl Store {
    pub fn open(app: &tauri::AppHandle) -> Self {
        Self::load(app).unwrap_or_else(|error| Self(Err(error), Mutex::new(None), None))
    }

    fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let directory = app
            .path()
            .app_config_dir()
            .map_err(|error| error.to_string())?;
        let data = app.path().app_data_dir().map_err(|error| error.to_string())?;
        let plugins = data.join("plugins");
        std::fs::create_dir_all(&plugins).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        Self::from_connection_with_root(
            Connection::open(directory.join("customization.sqlite3"))
                .map_err(|error| error.to_string())?,
            Some(plugins),
        )
        .map_err(|error| error.message)
    }

    fn from_connection(connection: Connection) -> Result<Self, Error> {
        Self::from_connection_with_root(connection, None)
    }

    fn from_connection_with_root(
        mut connection: Connection,
        root: Option<PathBuf>,
    ) -> Result<Self, Error> {
        connection.execute_batch("CREATE TABLE IF NOT EXISTS customization (id INTEGER PRIMARY KEY CHECK(id = 1), document TEXT NOT NULL, previous TEXT);")?;
        history::create(&connection)?;
        plugin::create(&connection)?;
        Self::migrate(&mut connection)?;
        Ok(Self(
            Ok(Mutex::new(connection)),
            Mutex::new(None),
            root,
        ))
    }

    fn plugin_root(&self) -> Option<PathBuf> {
        self.2.clone()
    }

    fn migrate(connection: &mut Connection) -> Result<(), Error> {
        let transaction = connection.transaction()?;
        let saved: Option<(String, Option<String>)> = transaction
            .query_row(
                "SELECT document, previous FROM customization WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let mut current = None;
        let mut previous = None;
        if let Some((raw, previous_raw)) = saved {
            let document = Self::decode(&raw)?;
            let previous_document = previous_raw.as_deref().map(Self::decode).transpose()?;
            let canonical = serde_json::to_string(&document)
                .map_err(|error| Error::new("invalid_document", error))?;
            let previous_canonical = previous_document
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| Error::new("invalid_document", error))?;
            if canonical != raw || previous_canonical != previous_raw {
                transaction.execute(
                    "UPDATE customization SET document = ?1, previous = ?2 WHERE id = 1",
                    params![canonical, previous_canonical],
                )?;
            }
            current = Some(document);
            previous = previous_document;
        }
        history::migrate(&transaction, current.as_ref(), previous.as_ref())?;
        transaction.commit()?;
        plugin::migrate(connection)?;
        Ok(())
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, Error> {
        self.0
            .as_ref()
            .map_err(|error| Error::new("storage", error))?
            .lock()
            .map_err(|error| Error::new("storage", error))
    }

    fn decode(value: &str) -> Result<Document, Error> {
        let raw: serde_json::Value =
            serde_json::from_str(value).map_err(|error| Error::new("invalid_document", error))?;
        let mut document: Document = match raw.get("version").and_then(serde_json::Value::as_u64) {
            Some(1) => document::migrate_legacy(value)
                .map_err(|error| Error::new("invalid_document", error))?,
            Some(2) => document::migrate_v2(value)
                .map_err(|error| Error::new("invalid_document", error))?,
            Some(3) => serde_json::from_value(raw)
                .map_err(|error| Error::new("invalid_document", error))?,
            Some(_) => {
                return Err(Error::new(
                    "unsupported_version",
                    "This customization document requires a different version of Luma.",
                ))
            }
            None => {
                return Err(Error::new(
                    "invalid_document",
                    "Missing customization document version.",
                ))
            }
        };
        if document.revision == 0 || document.revision > 9_007_199_254_740_991 {
            return Err(Error::new(
                "invalid_document",
                "Invalid customization revision.",
            ));
        }
        document::migrate_app_order(&mut document.config.order);
        document::validate(&document.config).map_err(|error| Error::new("validation", error))?;
        Ok(document)
    }

    fn current(connection: &Connection) -> Result<Option<Document>, Error> {
        let raw: Option<String> = connection
            .query_row(
                "SELECT document FROM customization WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        raw.as_deref().map(Self::decode).transpose()
    }

    pub fn read(&self) -> Result<Option<Document>, Error> {
        let connection = self.connection()?;
        Self::current(&connection)
    }

    pub fn read_effective(&self) -> Result<Option<Document>, Error> {
        let connection = self.connection()?;
        let Some(document) = Self::current(&connection)? else {
            return Ok(None);
        };
        Ok(Some(plugin::snapshot_for_connection(&connection, &document)?))
    }

    pub fn snapshot(&self) -> Result<Option<Snapshot>, Error> {
        let connection = self.connection()?;
        let Some(document) = Self::current(&connection)? else {
            return Ok(None);
        };
        let effective = plugin::snapshot_for_connection(&connection, &document)?;
        Ok(Some(Snapshot {
            document: effective,
            plugins: plugin::state(&connection, &document.config)?,
        }))
    }

    pub fn initialize(&self, config: Config) -> Result<Document, Error> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        if let Some(current) = Self::current(&transaction)? {
            return Ok(current);
        }
        document::validate(&config).map_err(|error| Error::new("validation", error))?;
        let document = Document {
            version: 3,
            revision: 1,
            config,
        };
        let raw =
            serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute(
            "INSERT INTO customization (id, document) VALUES (1, ?1)",
            [raw],
        )?;
        let effective = plugin::apply(&transaction, &document.config)?;
        history::record(
            &transaction,
            &document,
            &effective,
            &plugin::snapshot(&transaction)?,
        )?;
        transaction.commit()?;
        Ok(document)
    }

    pub fn commit(&self, request: Commit) -> Result<Document, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let document = self.change(request.expected_revision, |_| Ok(request.config))?;
        *preview = None;
        Ok(document)
    }

    pub fn edit(&self, edit: impl FnOnce(&mut Config)) -> Result<Document, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let document = self.change_current(None, |mut current| {
            edit(&mut current);
            Ok(current)
        })?;
        *preview = None;
        Ok(document)
    }

    fn change(
        &self,
        revision: u64,
        edit: impl FnOnce(Config) -> Result<Config, Error>,
    ) -> Result<Document, Error> {
        self.change_current(Some(revision), edit)
    }

    fn change_current(
        &self,
        revision: Option<u64>,
        edit: impl FnOnce(Config) -> Result<Config, Error>,
    ) -> Result<Document, Error> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once to migrate your existing preferences.",
            )
        })?;
        if revision.is_some_and(|revision| revision != current.revision) {
            return Err(Error::new("conflict", "Customization changed elsewhere. Read the current revision before applying your changes."));
        }
        let effective = plugin::apply(&transaction, &current.config)?;
        let requested = edit(effective.clone())?;
        let prepared = plugin::prepare(&transaction, &current.config, &effective, &requested)?;
        plugin::apply_prepared(&transaction, &prepared)?;
        let document = Self::write(&transaction, &current, prepared.base.clone(), prepared.force)?;
        transaction.commit()?;
        Ok(document)
    }

    fn write(
        transaction: &Connection,
        current: &Document,
        config: Config,
        force: bool,
    ) -> Result<Document, Error> {
        document::validate(&config).map_err(|error| Error::new("validation", error))?;
        if config == current.config && !force {
            return Ok(current.clone());
        }
        if current.revision >= 9_007_199_254_740_991 {
            return Err(Error::new(
                "revision_limit",
                "Customization revision limit reached.",
            ));
        }
        let document = Document {
            version: 3,
            revision: history::next_revision(transaction, current.revision)?,
            config,
        };
        let effective = plugin::apply(transaction, &document.config)?;
        let raw =
            serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute(
            "UPDATE customization SET previous = document, document = ?1 WHERE id = 1",
            [raw],
        )?;
        history::record(
            transaction,
            &document,
            &effective,
            &plugin::snapshot(transaction)?,
        )?;
        Ok(document)
    }

    pub fn history(&self) -> Result<Vec<history::HistoryEntry>, Error> {
        let connection = self.connection()?;
        history::list(&connection)
    }

    pub fn restore(&self, expected_revision: u64) -> Result<Document, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before restoring customization.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(Error::new("conflict", "Customization changed elsewhere."));
        }
        let previous = history::previous_entry(&transaction, current.revision)?
            .ok_or_else(|| Error::new("no_previous", "No previous customization is available."))?;
        plugin::restore_snapshot(&transaction, &previous.plugins)?;
        let document = Self::write(&transaction, &current, previous.config, true)?;
        transaction.commit()?;
        *preview = None;
        Ok(document)
    }

    pub fn restore_layout(&self, expected_revision: u64) -> Result<Document, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before restoring a layout.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(Error::new(
                "conflict",
                "The saved layout changed. Reload it before restoring.",
            ));
        }
        let previous = history::previous_entry(&transaction, current.revision)?
            .ok_or_else(|| Error::new("no_previous", "No previous layout is available."))?;
        let effective = plugin::apply(&transaction, &current.config)?;
        let mut requested = effective.clone();
        requested.home = previous.config.home;
        requested.home_columns = previous.config.home_columns;
        let prepared = plugin::prepare(&transaction, &current.config, &effective, &requested)?;
        plugin::apply_prepared(&transaction, &prepared)?;
        let document = Self::write(&transaction, &current, prepared.base.clone(), true)?;
        transaction.commit()?;
        *preview = None;
        Ok(document)
    }

    pub fn restore_version(
        &self,
        expected_revision: u64,
        revision: u64,
    ) -> Result<(Document, Vec<String>), Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before restoring customization.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(Error::new("conflict", "Customization changed elsewhere."));
        }
        let selected = history::find_entry(&transaction, revision)?.ok_or_else(|| {
            Error::new("no_version", "That saved version is no longer available.")
        })?;
        let notices = plugin::restore_snapshot(&transaction, &selected.plugins)?;
        let document = Self::write(&transaction, &current, selected.config, true)?;
        transaction.commit()?;
        *preview = None;
        Ok((document, notices))
    }

    pub fn reset_appearance(&self, expected_revision: u64) -> Result<Document, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before resetting appearance.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(Error::new("conflict", "Customization changed elsewhere."));
        }
        let defaults = Config::default();
        let effective = plugin::apply(&transaction, &current.config)?;
        let mut requested = effective.clone();
        requested.theme = defaults.theme;
        requested.density = defaults.density;
        requested.app_tile_shape = defaults.app_tile_shape;
        requested.text_size = defaults.text_size;
        requested.reduced_motion = defaults.reduced_motion;
        requested.home_columns = defaults.home_columns;
        requested.home = defaults.home;
        requested.widgets = defaults.widgets;
        let prepared = plugin::prepare(&transaction, &current.config, &effective, &requested)?;
        plugin::apply_prepared(&transaction, &prepared)?;
        let document = Self::write(&transaction, &current, prepared.base.clone(), true)?;
        transaction.commit()?;
        *preview = None;
        Ok(document)
    }

    pub fn preview(&self) -> Result<Option<Preview>, Error> {
        Ok(self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?
            .clone())
    }

    pub fn stage(&self, request: Commit) -> Result<Preview, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        if preview.is_some() {
            return Err(Error::new(
                "preview_busy",
                "Finish or cancel the current preview first.",
            ));
        }
        let connection = self.connection()?;
        let current = Self::current(&connection)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before previewing a layout.",
            )
        })?;
        if current.revision != request.expected_revision {
            return Err(Error::new(
                "conflict",
                "The saved layout changed. Reload it before previewing your draft.",
            ));
        }
        let effective = plugin::apply(&connection, &current.config)?;
        let prepared = plugin::prepare(&connection, &current.config, &effective, &request.config)?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| Error::new("storage", error))?;
        let candidate = Preview {
            id: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            base_revision: current.revision,
            config: request.config,
            plugin: None,
            prepared: Some(prepared),
            plugin_action: None,
        };
        *preview = Some(candidate.clone());
        Ok(candidate)
    }

    pub fn preview_plugin(&self, plugin_id: &str, expected_revision: u64) -> Result<Preview, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        if preview.is_some() {
            return Err(Error::new(
                "preview_busy",
                "Finish or cancel the current preview first.",
            ));
        }
        let connection = self.connection()?;
        let current = Self::current(&connection)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before previewing a plugin.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(Error::new(
                "conflict",
                "Customization changed elsewhere. Read it again before previewing this plugin.",
            ));
        }
        let (config, action) = plugin::prepare_enable(&connection, &current.config, plugin_id)?;
        let record = plugin::state(&connection, &current.config)?
            .plugins
            .into_iter()
            .find(|plugin| plugin.id == plugin_id)
            .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
        let reattach = matches!(
            action,
            plugin::PluginAction::Enable { reattach: true, .. }
        );
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| Error::new("storage", error))?;
        let candidate = Preview {
            id: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            base_revision: current.revision,
            config,
            plugin: Some(plugin::PluginPreview {
                id: record.id,
                name: record.name,
                action: if reattach { "reattach" } else { "enable" }.into(),
            }),
            prepared: None,
            plugin_action: Some(action),
        };
        *preview = Some(candidate.clone());
        Ok(candidate)
    }

    pub fn preview_plugin_settings(
        &self,
        plugin_id: &str,
        expected_revision: u64,
        values: &BTreeMap<String, serde_json::Value>,
    ) -> Result<Preview, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        if preview.is_some() {
            return Err(Error::new(
                "preview_busy",
                "Finish or cancel the current preview first.",
            ));
        }
        let connection = self.connection()?;
        let current = Self::current(&connection)?.ok_or_else(|| {
            Error::new(
                "not_initialized",
                "Open Luma once before previewing plugin settings.",
            )
        })?;
        if current.revision != expected_revision {
            return Err(Error::new(
                "conflict",
                "Customization changed elsewhere. Read it again before staging settings.",
            ));
        }
        let map: serde_json::Map<String, serde_json::Value> =
            values.iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        let (config, action) =
            plugin::prepare_settings(&connection, &current.config, plugin_id, &map)?;
        let record = plugin::state(&connection, &current.config)?
            .plugins
            .into_iter()
            .find(|plugin| plugin.id == plugin_id)
            .ok_or_else(|| Error::new("plugin_missing", "That plugin is not installed."))?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| Error::new("storage", error))?;
        let candidate = Preview {
            id: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            base_revision: current.revision,
            config,
            plugin: Some(plugin::PluginPreview {
                id: record.id,
                name: record.name,
                action: "settings".into(),
            }),
            prepared: None,
            plugin_action: Some(action),
        };
        *preview = Some(candidate.clone());
        Ok(candidate)
    }

    pub fn keep(&self, id: &str) -> Result<Document, Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        let candidate = preview
            .as_ref()
            .filter(|preview| preview.id == id)
            .ok_or_else(|| {
                Error::new(
                    "preview_mismatch",
                    "This preview is no longer active. Preview your draft again.",
                )
            })?
            .clone();
        if let Some(action) = &candidate.plugin_action {
            let mut connection = self.connection()?;
            let transaction = connection.transaction()?;
            let current = Self::current(&transaction)?.ok_or_else(|| {
                Error::new("not_initialized", "Customization is not initialized.")
            })?;
            if current.revision != candidate.base_revision {
                return Err(Error::new(
                    "preview_mismatch",
                    "Customization changed elsewhere. Preview the plugin again.",
                ));
            }
            plugin::apply_action(&transaction, &current, action)?;
            let document = Self::write(&transaction, &current, current.config.clone(), true)?;
            transaction.commit()?;
            *preview = None;
            return Ok(document);
        }
        let prepared = candidate.prepared.clone().ok_or_else(|| {
            Error::new("preview_mismatch", "This preview has no pending change.")
        })?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| {
            Error::new("not_initialized", "Customization is not initialized.")
        })?;
        if current.revision != candidate.base_revision {
            return Err(Error::new(
                "preview_mismatch",
                "Customization changed elsewhere. Preview your draft again.",
            ));
        }
        plugin::apply_prepared(&transaction, &prepared)?;
        let document = Self::write(&transaction, &current, prepared.base.clone(), true)?;
        transaction.commit()?;
        *preview = None;
        Ok(document)
    }

    pub fn cancel(&self, id: &str) -> Result<(), Error> {
        let mut preview = self
            .1
            .lock()
            .map_err(|error| Error::new("storage", error))?;
        if preview.as_ref().is_some_and(|preview| preview.id != id) {
            return Err(Error::new(
                "preview_mismatch",
                "This preview is no longer active.",
            ));
        }
        *preview = None;
        Ok(())
    }

    pub fn clear_preview(&self) {
        if let Ok(mut preview) = self.1.lock() {
            *preview = None;
        }
    }
}

pub fn publish(app: &tauri::AppHandle, document: &Document) {
    let store = app.state::<Store>();
    let (effective, plugins) = match store.snapshot() {
        Ok(Some(snapshot)) => (snapshot.document, Some(snapshot.plugins)),
        _ => (document.clone(), None),
    };
    let _ = crate::remote::server::sync_customization(app, &effective);
    let _ = app.emit_to("main", "customization-changed", &effective);
    if let Some(plugins) = plugins {
        let _ = app.emit_to("main", "customization-plugins-changed", plugins);
    }
    publish_preview(app);
}

fn effective(app: &tauri::AppHandle, document: Document) -> Result<Document, Error> {
    let store = app.state::<Store>();
    let connection = store.connection()?;
    plugin::snapshot_for_connection(&connection, &document)
}

fn publish_preview(app: &tauri::AppHandle) {
    if let Ok(preview) = app.state::<Store>().preview() {
        let _ = app.emit_to("main", "customization-preview-changed", preview);
    }
}

#[tauri::command]
pub async fn customization_preview_read(app: tauri::AppHandle) -> Result<Option<Preview>, Error> {
    app.state::<Store>().preview()
}

#[tauri::command]
pub async fn customization_preview(
    app: tauri::AppHandle,
    request: Commit,
) -> Result<Preview, Error> {
    if !app.get_webview_window("main").is_some_and(|window| {
        window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
    }) {
        return Err(Error::new(
            "launcher_unavailable",
            "Open Luma before previewing a layout.",
        ));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let preview = app.state::<Store>().stage(request)?;
        let visible = app.get_webview_window("main").is_some_and(|window| {
            window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
        });
        if !visible
            || app
                .emit_to("main", "customization-preview-changed", Some(&preview))
                .is_err()
        {
            let _ = app.state::<Store>().cancel(&preview.id);
            return Err(Error::new(
                "launcher_unavailable",
                "Luma closed before the preview could open.",
            ));
        }
        Ok(preview)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_preview_keep(
    app: tauri::AppHandle,
    request: PreviewAction,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().keep(&request.preview_id)?;
        publish(&app, &document);
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_preview_cancel(
    app: tauri::AppHandle,
    request: PreviewAction,
) -> Result<(), Error> {
    app.state::<Store>().cancel(&request.preview_id)?;
    publish_preview(&app);
    Ok(())
}

#[tauri::command]
pub async fn customization_restore(
    app: tauri::AppHandle,
    expected_revision: u64,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().restore(expected_revision)?;
        publish(&app, &document);
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_restore_layout(
    app: tauri::AppHandle,
    expected_revision: u64,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().restore_layout(expected_revision)?;
        publish(&app, &document);
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_history(
    app: tauri::AppHandle,
) -> Result<Vec<history::HistoryEntry>, Error> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Store>().history())
        .await
        .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_restore_version(
    app: tauri::AppHandle,
    expected_revision: u64,
    revision: u64,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let (document, notices) = app
            .state::<Store>()
            .restore_version(expected_revision, revision)?;
        publish(&app, &document);
        if !notices.is_empty() {
            plugin::notice(&app, notices.join(" "));
        }
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_reset_appearance(
    app: tauri::AppHandle,
    expected_revision: u64,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().reset_appearance(expected_revision)?;
        publish(&app, &document);
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_read(app: tauri::AppHandle) -> Result<Option<Document>, Error> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Store>().read_effective())
        .await
        .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_initialize(
    app: tauri::AppHandle,
    config: Config,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().initialize(config)?;
        publish(&app, &document);
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_commit(
    app: tauri::AppHandle,
    request: Commit,
) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().commit(request)?;
        publish(&app, &document);
        effective(&app, document)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_plugins_read(app: tauri::AppHandle) -> Result<plugin::PluginsState, Error> {
    plugin::state_for_app(&app)
}

#[tauri::command]
pub async fn customization_plugin_import(
    app: tauri::AppHandle,
) -> Result<Option<plugin::ImportResult>, Error> {
    tauri::async_runtime::spawn_blocking(move || plugin::command_import(app))
        .await
        .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_plugin_export(
    app: tauri::AppHandle,
    plugin_id: String,
) -> Result<Option<String>, Error> {
    tauri::async_runtime::spawn_blocking(move || plugin::command_export(app, plugin_id))
        .await
        .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_plugin_preview(
    app: tauri::AppHandle,
    plugin_id: String,
    expected_revision: u64,
) -> Result<Preview, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let preview = plugin::command_preview(&app.state::<Store>(), &plugin_id, expected_revision)?;
        publish_preview(&app);
        Ok::<_, Error>(preview)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_plugin_disable(
    app: tauri::AppHandle,
    plugin_id: String,
    expected_revision: u64,
) -> Result<plugin::PluginsState, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        plugin::command_disable(&app.state::<Store>(), &plugin_id, expected_revision)?;
        if let Ok(Some(snapshot)) = app.state::<Store>().snapshot() {
            publish(&app, &snapshot.document);
        }
        plugin::state_for_app(&app)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_plugin_remove(
    app: tauri::AppHandle,
    plugin_id: String,
    expected_revision: u64,
) -> Result<plugin::PluginsState, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        plugin::command_remove(&app.state::<Store>(), &plugin_id, expected_revision)?;
        if let Ok(Some(snapshot)) = app.state::<Store>().snapshot() {
            publish(&app, &snapshot.document);
        }
        plugin::state_for_app(&app)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_plugin_settings_save(
    app: tauri::AppHandle,
    plugin_id: String,
    expected_revision: u64,
    settings: BTreeMap<String, serde_json::Value>,
) -> Result<plugin::PluginsState, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let map: serde_json::Map<String, serde_json::Value> = settings.into_iter().collect();
        plugin::command_save_settings(&app.state::<Store>(), &plugin_id, expected_revision, map)?;
        if let Ok(Some(snapshot)) = app.state::<Store>().snapshot() {
            publish(&app, &snapshot.document);
        }
        plugin::state_for_app(&app)
    })
    .await
    .map_err(|error| Error::new("storage", error))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn saved_app_order_adds_leagues_without_resetting_existing_apps() {
        let mut config = Config::default();
        config.order.retain(|id| !["NBA", "NFL", "MLB", "NHL"].contains(&id.as_str()));
        config.order.swap(0, 4);
        let previous = config.order.clone();
        let raw = serde_json::to_string(&Document { version: 3, revision: 7, config }).unwrap();
        let document = Store::decode(&raw).unwrap();
        assert_eq!(document.revision, 7);
        assert_eq!(&document.config.order[..previous.len() - 1], &previous[..previous.len() - 1]);
        assert_eq!(&document.config.order[previous.len() - 1..], &["NBA", "NFL", "MLB", "NHL", "Custom"]);
        document::validate(&document.config).unwrap();
        assert_eq!(Store::decode(&serde_json::to_string(&document).unwrap()).unwrap(), document);
    }

    #[test]
    fn initialization_is_explicit_and_preserves_first_migration() {
        let store = store();
        assert!(store.read().unwrap().is_none());
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: 0,
                    config: Config::default()
                })
                .unwrap_err()
                .code,
            "not_initialized"
        );
        let mut config = Config::default();
        config.home.reverse();
        let first = store.initialize(config).unwrap();
        assert_eq!(store.initialize(Config::default()).unwrap(), first);
    }

    #[test]
    fn commits_reject_stale_revisions_and_restore_without_rewinding_revision() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut config = first.config.clone();
        config.theme = "onyx".into();
        let second = store
            .commit(Commit {
                expected_revision: 1,
                config,
            })
            .unwrap();
        assert_eq!(second.revision, 2);
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: 1,
                    config: first.config.clone()
                })
                .unwrap_err()
                .code,
            "conflict"
        );
        assert_eq!(store.read().unwrap().unwrap(), second);
        let restored = store.restore(2).unwrap();
        assert_eq!(restored.revision, 3);
        assert_eq!(restored.config, first.config);
    }

    #[test]
    fn layout_restore_preserves_unrelated_preferences() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut themed = first.config.clone();
        themed.theme = "onyx".into();
        let themed = store
            .commit(Commit {
                expected_revision: first.revision,
                config: themed,
            })
            .unwrap();
        let mut rearranged = themed.config.clone();
        rearranged.home.reverse();
        let rearranged = store
            .commit(Commit {
                expected_revision: themed.revision,
                config: rearranged,
            })
            .unwrap();
        let restored = store.restore_layout(rearranged.revision).unwrap();
        assert_eq!(restored.revision, rearranged.revision + 1);
        assert_eq!(restored.config.theme, "onyx");
        assert_eq!(restored.config.home, first.config.home);
        assert_eq!(restored.config.home_columns, first.config.home_columns);
        assert_eq!(restored.config.order, first.config.order);
        assert_eq!(store.history().unwrap()[0].config, restored.config);
    }

    #[test]
    fn history_retains_the_last_ten_documents_including_current() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let themes = [
            "violet-night",
            "sapphire",
            "mint",
            "ember",
            "graphite",
            "onyx",
        ];
        let mut document = first;
        for index in 0..11 {
            let mut config = document.config.clone();
            config.theme = themes[(index + 1) % themes.len()].into();
            document = store
                .commit(Commit {
                    expected_revision: document.revision,
                    config,
                })
                .unwrap();
        }
        let history = store.history().unwrap();
        assert_eq!(history.len(), 10);
        assert_eq!(document.revision, 12);
        assert_eq!(history[0].revision, document.revision);
        assert_eq!(history[0].config, document.config);
        assert!(history
            .windows(2)
            .all(|pair| pair[0].revision == pair[1].revision + 1));
        assert_eq!(history.last().unwrap().revision, document.revision - 9);
        assert!(history.iter().all(|entry| entry.saved_at.is_some()));
    }

    #[test]
    fn every_committed_write_path_records_one_history_entry() {
        let store = store();
        store.initialize(Config::default()).unwrap();
        assert_eq!(store.history().unwrap().len(), 1);
        let edited = store
            .edit(|config| config.density = "compact".into())
            .unwrap();
        assert_eq!(store.history().unwrap().len(), 2);
        assert_eq!(store.history().unwrap()[0].config, edited.config);
        let mut candidate = edited.config.clone();
        candidate.app_tile_shape = "square".into();
        candidate.home_columns = 2;
        let preview = store
            .stage(Commit {
                expected_revision: edited.revision,
                config: candidate,
            })
            .unwrap();
        let kept = store.keep(&preview.id).unwrap();
        assert_eq!(store.history().unwrap().len(), 3);
        assert_eq!(store.history().unwrap()[0].revision, kept.revision);
        let restored = store.restore(kept.revision).unwrap();
        assert_eq!(store.history().unwrap().len(), 4);
        let layout = store.restore_layout(restored.revision).unwrap();
        assert_eq!(layout.config.home_columns, 2);
        let history = store.history().unwrap();
        assert_eq!(history.len(), 5);
        assert_eq!(history[0].revision, layout.revision);
        assert_eq!(history[0].config, layout.config);
    }

    #[test]
    fn restoring_a_selected_version_creates_a_new_revision_without_deleting_the_replaced_state() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut config = first.config.clone();
        config.home_columns = 2;
        let second = store
            .commit(Commit {
                expected_revision: first.revision,
                config,
            })
            .unwrap();
        let mut config = second.config.clone();
        config.theme = "onyx".into();
        let third = store
            .commit(Commit {
                expected_revision: second.revision,
                config,
            })
            .unwrap();
        assert_eq!(
            store
                .restore_version(second.revision, first.revision)
                .unwrap_err()
                .code,
            "conflict"
        );
        assert_eq!(
            store.restore_version(third.revision, 99).unwrap_err().code,
            "no_version"
        );
        let restored = store
            .restore_version(third.revision, first.revision)
            .unwrap();
        assert_eq!(restored.revision, 4);
        assert_eq!(restored.config, first.config);
        let history = store.history().unwrap();
        assert_eq!(
            history
                .iter()
                .map(|entry| entry.revision)
                .collect::<Vec<_>>(),
            vec![4, 3, 2, 1]
        );
        assert_eq!(history[0].config, first.config);
        assert_eq!(history[1].config, third.config);
        let reverted = store
            .restore_version(restored.revision, third.revision)
            .unwrap();
        assert_eq!(reverted.config, third.config);
        assert_eq!(store.history().unwrap().len(), 5);
    }

    #[test]
    fn reset_appearance_only_reverts_visual_home_and_widget_defaults() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut config = first.config.clone();
        config.theme = "onyx".into();
        config.density = "compact".into();
        config.app_tile_shape = "square".into();
        config.text_size = "larger".into();
        config.reduced_motion = true;
        config.interface_sounds = "roku".into();
        config.sound_level = "soft".into();
        config.home_columns = 2;
        config.home.reverse();
        config.home[0].visible = false;
        config.home[1].span = 1;
        config.widgets.spotify = document::Widget {
            enabled: false,
            placement: "bottom-left".into(),
        };
        config.order.swap(0, 1);
        let customized = store
            .commit(Commit {
                expected_revision: first.revision,
                config,
            })
            .unwrap();
        let defaults = Config::default();
        assert_eq!(
            store
                .reset_appearance(customized.revision - 1)
                .unwrap_err()
                .code,
            "conflict"
        );
        let reset = store.reset_appearance(customized.revision).unwrap();
        assert_eq!(reset.revision, customized.revision + 1);
        assert_eq!(reset.config.theme, defaults.theme);
        assert_eq!(reset.config.density, defaults.density);
        assert_eq!(reset.config.app_tile_shape, defaults.app_tile_shape);
        assert_eq!(reset.config.text_size, defaults.text_size);
        assert_eq!(reset.config.reduced_motion, defaults.reduced_motion);
        assert_eq!(reset.config.home_columns, defaults.home_columns);
        assert_eq!(reset.config.home, defaults.home);
        assert_eq!(reset.config.widgets, defaults.widgets);
        assert_eq!(reset.config.order, customized.config.order);
        assert_eq!(reset.config.interface_sounds, "roku");
        assert_eq!(reset.config.sound_level, "soft");
        assert_eq!(store.history().unwrap()[0].config, reset.config);
        assert_eq!(store.reset_appearance(reset.revision).unwrap(), reset);
        assert_eq!(store.history().unwrap().len(), 3);
    }

    #[test]
    fn no_op_and_failed_writes_insert_no_history() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: first.revision,
                    config: first.config.clone()
                })
                .unwrap(),
            first
        );
        assert_eq!(store.reset_appearance(first.revision).unwrap(), first);
        assert_eq!(store.history().unwrap().len(), 1);
        store.connection().unwrap().execute_batch("CREATE TRIGGER reject_history BEFORE INSERT ON versions BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        let mut config = first.config.clone();
        config.theme = "onyx".into();
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: first.revision,
                    config
                })
                .unwrap_err()
                .code,
            "storage"
        );
        assert_eq!(store.read().unwrap().unwrap(), first);
        assert_eq!(store.history().unwrap().len(), 1);
    }

    #[test]
    fn unsupported_history_entries_are_preserved_but_not_offered() {
        let path = std::env::temp_dir().join(format!(
            "luma-customization-history-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let current;
        let future = serde_json::to_string(&Document {
            version: 99,
            revision: 2,
            config: Config::default(),
        })
        .unwrap();
        {
            let store = Store::from_connection(Connection::open(&path).unwrap()).unwrap();
            current = store.initialize(Config::default()).unwrap();
            store
                .connection()
                .unwrap()
                .execute(
                    "INSERT INTO versions (revision, saved_at, document) VALUES (2, NULL, ?1)",
                    [&future],
                )
                .unwrap();
        }
        {
            let store = Store::from_connection(Connection::open(&path).unwrap()).unwrap();
            assert_eq!(store.read().unwrap().unwrap(), current);
            assert_eq!(store.history().unwrap().len(), 1);
            assert_eq!(store.history().unwrap()[0].revision, current.revision);
            let saved: String = store
                .connection()
                .unwrap()
                .query_row(
                    "SELECT document FROM versions WHERE revision = 2",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(saved, future);
            let mut config = current.config.clone();
            config.theme = "onyx".into();
            let changed = store
                .commit(Commit {
                    expected_revision: current.revision,
                    config,
                })
                .unwrap();
            assert_eq!(changed.revision, 3);
            let restored = store.restore(changed.revision).unwrap();
            assert_eq!(restored.config, current.config);
            assert_eq!(restored.revision, 4);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn layout_restore_no_op_cancels_the_pending_preview() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut config = first.config.clone();
        config.theme = "onyx".into();
        let current = store
            .commit(Commit {
                expected_revision: first.revision,
                config,
            })
            .unwrap();
        let preview = store
            .stage(Commit {
                expected_revision: current.revision,
                config: first.config,
            })
            .unwrap();
        assert_eq!(store.restore_layout(current.revision).unwrap(), current);
        assert!(store.preview().unwrap().is_none());
        assert_eq!(
            store.keep(&preview.id).unwrap_err().code,
            "preview_mismatch"
        );
    }

    #[test]
    fn invalid_history_does_not_consume_the_usable_history_budget() {
        let store = store();
        store.initialize(Config::default()).unwrap();
        for revision in 50..60 {
            let raw = serde_json::to_string(&Document {
                version: 99,
                revision,
                config: Config::default(),
            })
            .unwrap();
            store
                .connection()
                .unwrap()
                .execute(
                    "INSERT INTO versions (revision, document) VALUES (?1, ?2)",
                    params![revision as i64, raw],
                )
                .unwrap();
        }
        for _ in 0..12 {
            store.edit(|config| config.home.reverse()).unwrap();
        }
        let entries = store.history().unwrap();
        assert_eq!(entries.len(), 10);
        assert_eq!(entries[0].revision, store.read().unwrap().unwrap().revision);
        let count: i64 = store
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM versions WHERE revision BETWEEN 50 AND 59",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 10);
    }

    #[test]
    fn invalid_and_failed_writes_preserve_current_document() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut config = first.config.clone();
        config.home.pop();
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: 1,
                    config
                })
                .unwrap_err()
                .code,
            "validation"
        );
        store.connection().unwrap().execute_batch("CREATE TRIGGER reject_write BEFORE UPDATE ON customization BEGIN SELECT RAISE(ABORT, 'disk write rejected'); END;").unwrap();
        let mut config = first.config.clone();
        config.theme = "onyx".into();
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: 1,
                    config
                })
                .unwrap_err()
                .code,
            "storage"
        );
        assert_eq!(store.read().unwrap().unwrap(), first);
    }

    #[test]
    fn unsupported_documents_are_not_overwritten_by_migration() {
        let store = store();
        let raw = serde_json::to_string(&Document {
            version: 99,
            revision: 1,
            config: Config::default(),
        })
        .unwrap();
        store
            .connection()
            .unwrap()
            .execute(
                "INSERT INTO customization (id, document) VALUES (1, ?1)",
                [&raw],
            )
            .unwrap();
        assert_eq!(
            store.initialize(Config::default()).unwrap_err().code,
            "unsupported_version"
        );
        let saved: String = store
            .connection()
            .unwrap()
            .query_row("SELECT document FROM customization", [], |row| row.get(0))
            .unwrap();
        assert_eq!(saved, raw);
    }

    #[test]
    fn configuration_and_previous_revision_survive_reopening_storage() {
        let path = std::env::temp_dir().join(format!(
            "luma-customization-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first;
        {
            let store = Store::from_connection(Connection::open(&path).unwrap()).unwrap();
            first = store.initialize(Config::default()).unwrap();
            let mut config = first.config.clone();
            config.home.reverse();
            config.widgets.spotify.placement = "bottom-left".into();
            store
                .commit(Commit {
                    expected_revision: 1,
                    config,
                })
                .unwrap();
        }
        {
            let store = Store::from_connection(Connection::open(&path).unwrap()).unwrap();
            let saved = store.read().unwrap().unwrap();
            assert_eq!(saved.revision, 2);
            assert_eq!(saved.config.widgets.spotify.placement, "bottom-left");
            assert_eq!(store.restore(2).unwrap().config, first.config);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unavailable_storage_reports_errors_without_using_temporary_defaults() {
        let store = Store(
            Err("Cannot open customization database".into()),
            Mutex::new(None),
        );
        assert_eq!(store.read().unwrap_err().code, "storage");
        assert_eq!(
            store.initialize(Config::default()).unwrap_err().code,
            "storage"
        );
        assert_eq!(
            store
                .commit(Commit {
                    expected_revision: 1,
                    config: Config::default()
                })
                .unwrap_err()
                .code,
            "storage"
        );
    }

    #[test]
    fn preview_is_temporary_and_keep_saves_the_exact_candidate_with_undo() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home_columns = 2;
        candidate.home[1].span = 1;
        let preview = store
            .stage(Commit {
                expected_revision: 1,
                config: candidate.clone(),
            })
            .unwrap();
        assert_eq!(store.read().unwrap().unwrap(), original);
        assert_eq!(store.preview().unwrap().unwrap().config, candidate);
        let kept = store.keep(&preview.id).unwrap();
        assert_eq!(kept.revision, 2);
        assert_eq!(kept.config, candidate);
        assert_eq!(store.history().unwrap().len(), 2);
        assert!(store.preview().unwrap().is_none());
        assert_eq!(store.restore(2).unwrap().config, original.config);
    }

    #[test]
    fn cancel_and_closing_the_launcher_preserve_committed_preferences() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home.reverse();
        let first = store
            .stage(Commit {
                expected_revision: 1,
                config: candidate.clone(),
            })
            .unwrap();
        assert_eq!(
            store
                .stage(Commit {
                    expected_revision: 1,
                    config: candidate.clone()
                })
                .unwrap_err()
                .code,
            "preview_busy"
        );
        store.cancel(&first.id).unwrap();
        let second = store
            .stage(Commit {
                expected_revision: 1,
                config: candidate,
            })
            .unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(store.keep(&first.id).unwrap_err().code, "preview_mismatch");
        assert_eq!(
            store.cancel(&first.id).unwrap_err().code,
            "preview_mismatch"
        );
        assert_eq!(store.preview().unwrap().unwrap().id, second.id);
        store.clear_preview();
        assert!(store.preview().unwrap().is_none());
        assert_eq!(store.read().unwrap().unwrap(), original);
    }

    #[test]
    fn another_writer_invalidates_preview_and_stale_keep_cannot_overwrite_it() {
        let store = store();
        store.initialize(Config::default()).unwrap();
        let mut candidate = Config::default();
        candidate.home_columns = 2;
        let preview = store
            .stage(Commit {
                expected_revision: 1,
                config: candidate,
            })
            .unwrap();
        let changed = store.edit(|config| config.theme = "onyx".into()).unwrap();
        assert!(store.preview().unwrap().is_none());
        assert_eq!(
            store.keep(&preview.id).unwrap_err().code,
            "preview_mismatch"
        );
        assert_eq!(store.read().unwrap().unwrap(), changed);
        assert_eq!(
            store
                .stage(Commit {
                    expected_revision: 1,
                    config: Config::default()
                })
                .unwrap_err()
                .code,
            "conflict"
        );
    }

    #[test]
    fn a_failed_keep_retains_the_reviewed_candidate_for_retry() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home_columns = 2;
        let preview = store
            .stage(Commit {
                expected_revision: 1,
                config: candidate.clone(),
            })
            .unwrap();
        store.connection().unwrap().execute_batch("CREATE TRIGGER reject_keep BEFORE UPDATE ON customization BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        assert_eq!(store.keep(&preview.id).unwrap_err().code, "storage");
        assert_eq!(store.read().unwrap().unwrap(), original);
        assert_eq!(store.preview().unwrap().unwrap().id, preview.id);
        store
            .connection()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_keep;")
            .unwrap();
        assert_eq!(store.keep(&preview.id).unwrap().config, candidate);
    }

    #[test]
    fn invalid_candidates_never_enter_preview() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home[1].span = 3;
        assert_eq!(
            store
                .stage(Commit {
                    expected_revision: 1,
                    config: candidate
                })
                .unwrap_err()
                .code,
            "validation"
        );
        assert!(store.preview().unwrap().is_none());
        assert_eq!(store.read().unwrap().unwrap(), original);
    }

    fn legacy(revision: u64, config: Config) -> String {
        let mut raw = serde_json::to_value(Document {
            version: 1,
            revision,
            config,
        })
        .unwrap();
        let raw_config = raw["config"].as_object_mut().unwrap();
        for field in [
            "homeColumns",
            "textSize",
            "reducedMotion",
            "interfaceSounds",
            "soundLevel",
        ] {
            raw_config.remove(field);
        }
        for row in raw_config["home"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("component");
            row.as_object_mut().unwrap().remove("span");
        }
        serde_json::to_string(&raw).unwrap()
    }

    #[test]
    fn stored_migration_preserves_revision_order_visibility_and_undo() {
        let store = store();
        let previous = legacy(4, Config::default());
        let mut config = Config::default();
        config.home.reverse();
        config.home[0].visible = false;
        let raw = legacy(5, config.clone());
        {
            let mut connection = store.connection().unwrap();
            connection
                .execute(
                    "INSERT INTO customization (id, document, previous) VALUES (1, ?1, ?2)",
                    params![raw, previous],
                )
                .unwrap();
            Store::migrate(&mut connection).unwrap();
            let persisted: String = connection
                .query_row("SELECT document FROM customization", [], |row| row.get(0))
                .unwrap();
            assert_eq!(
                serde_json::from_str::<Document>(&persisted)
                    .unwrap()
                    .version,
                3
            );
            Store::migrate(&mut connection).unwrap();
        }
        let migrated = store.read().unwrap().unwrap();
        assert_eq!(migrated.revision, 5);
        assert_eq!(migrated.config, config);
        let history = store.history().unwrap();
        assert_eq!(
            history
                .iter()
                .map(|entry| entry.revision)
                .collect::<Vec<_>>(),
            vec![5, 4]
        );
        assert_eq!(history[0].config, config);
        assert_eq!(history[1].config, Config::default());
        assert!(history.iter().all(|entry| entry.saved_at.is_none()));
        let restored = store.restore(5).unwrap();
        assert_eq!(restored.version, 3);
        assert_eq!(restored.revision, 6);
        assert_eq!(restored.config, Config::default());
    }

    #[test]
    fn stored_v2_migration_preserves_instance_identity_and_previous_configuration() {
        let store = store();
        let mut config = Config::default();
        config.home[2].id = "personal-dock".into();
        config.home.reverse();
        config.home[0].visible = false;
        let encode = |revision, config| {
            let mut value = serde_json::to_value(Document {
                version: 2,
                revision,
                config,
            })
            .unwrap();
            let value_config = value["config"].as_object_mut().unwrap();
            for field in [
                "homeColumns",
                "textSize",
                "reducedMotion",
                "interfaceSounds",
                "soundLevel",
            ] {
                value_config.remove(field);
            }
            for row in value_config["home"].as_array_mut().unwrap() {
                row.as_object_mut().unwrap().remove("span");
            }
            value.to_string()
        };
        {
            let mut connection = store.connection().unwrap();
            connection
                .execute(
                    "INSERT INTO customization (id, document, previous) VALUES (1, ?1, ?2)",
                    params![encode(8, config.clone()), encode(7, Config::default())],
                )
                .unwrap();
            Store::migrate(&mut connection).unwrap();
        }
        let migrated = store.read().unwrap().unwrap();
        assert_eq!(migrated.version, 3);
        assert_eq!(migrated.revision, 8);
        assert_eq!(migrated.config, config);
        assert_eq!(store.restore(8).unwrap().config, Config::default());
    }

    #[test]
    fn migration_rejects_an_incompatible_previous_document_without_partial_writes() {
        let store = store();
        let raw = legacy(5, Config::default());
        let future = serde_json::to_string(&Document {
            version: 99,
            revision: 4,
            config: Config::default(),
        })
        .unwrap();
        let mut connection = store.connection().unwrap();
        connection
            .execute(
                "INSERT INTO customization (id, document, previous) VALUES (1, ?1, ?2)",
                params![raw, future],
            )
            .unwrap();
        assert_eq!(
            Store::migrate(&mut connection).unwrap_err().code,
            "unsupported_version"
        );
        let saved: String = connection
            .query_row("SELECT document FROM customization", [], |row| row.get(0))
            .unwrap();
        assert_eq!(saved, raw);
    }
}
