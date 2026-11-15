pub mod document;
pub mod server;

use document::{Config, Document};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, MutexGuard};
use tauri::{Emitter, Manager};

#[derive(Debug, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}

impl Error {
    fn new(code: &'static str, message: impl ToString) -> Self {
        Self { code, message: message.to_string() }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self { Self::new("storage", error) }
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
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewAction {
    pub preview_id: String,
}

pub struct Store(Result<Mutex<Connection>, String>, Mutex<Option<Preview>>);

impl Store {
    pub fn open(app: &tauri::AppHandle) -> Self {
        Self::load(app).unwrap_or_else(|error| Self(Err(error), Mutex::new(None)))
    }

    fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let directory = app.path().app_config_dir().map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        Self::from_connection(Connection::open(directory.join("customization.sqlite3")).map_err(|error| error.to_string())?)
            .map_err(|error| error.message)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, Error> {
        connection.execute_batch("CREATE TABLE IF NOT EXISTS customization (id INTEGER PRIMARY KEY CHECK(id = 1), document TEXT NOT NULL, previous TEXT);")?;
        Self::migrate(&mut connection)?;
        Ok(Self(Ok(Mutex::new(connection)), Mutex::new(None)))
    }

    fn migrate(connection: &mut Connection) -> Result<(), Error> {
        let transaction = connection.transaction()?;
        let saved: Option<(String, Option<String>)> = transaction.query_row("SELECT document, previous FROM customization WHERE id = 1", [], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        if let Some((raw, previous)) = saved {
            let document = Self::decode(&raw)?;
            let previous_document = previous.as_deref().map(Self::decode).transpose()?;
            let document = serde_json::to_string(&document).map_err(|error| Error::new("invalid_document", error))?;
            let previous_document = previous_document.as_ref().map(serde_json::to_string).transpose().map_err(|error| Error::new("invalid_document", error))?;
            if document != raw || previous_document != previous {
                transaction.execute("UPDATE customization SET document = ?1, previous = ?2 WHERE id = 1", params![document, previous_document])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, Error> {
        self.0.as_ref().map_err(|error| Error::new("storage", error))?
            .lock().map_err(|error| Error::new("storage", error))
    }

    fn decode(value: &str) -> Result<Document, Error> {
        let raw: serde_json::Value = serde_json::from_str(value).map_err(|error| Error::new("invalid_document", error))?;
        let document: Document = match raw.get("version").and_then(serde_json::Value::as_u64) {
            Some(1) => document::migrate_legacy(value).map_err(|error| Error::new("invalid_document", error))?,
            Some(2) => document::migrate_v2(value).map_err(|error| Error::new("invalid_document", error))?,
            Some(3) => serde_json::from_value(raw).map_err(|error| Error::new("invalid_document", error))?,
            Some(_) => return Err(Error::new("unsupported_version", "This customization document requires a different version of Luma.")),
            None => return Err(Error::new("invalid_document", "Missing customization document version.")),
        };
        if document.revision == 0 || document.revision > 9_007_199_254_740_991 {
            return Err(Error::new("invalid_document", "Invalid customization revision."));
        }
        document::validate(&document.config).map_err(|error| Error::new("validation", error))?;
        Ok(document)
    }

    fn current(connection: &Connection) -> Result<Option<Document>, Error> {
        let raw: Option<String> = connection.query_row("SELECT document FROM customization WHERE id = 1", [], |row| row.get(0)).optional()?;
        raw.as_deref().map(Self::decode).transpose()
    }

    pub fn read(&self) -> Result<Option<Document>, Error> {
        let connection = self.connection()?;
        Self::current(&connection)
    }

    pub fn initialize(&self, config: Config) -> Result<Document, Error> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        if let Some(current) = Self::current(&transaction)? { return Ok(current); }
        document::validate(&config).map_err(|error| Error::new("validation", error))?;
        let document = Document { version: 3, revision: 1, config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("INSERT INTO customization (id, document) VALUES (1, ?1)", [raw])?;
        transaction.commit()?;
        Ok(document)
    }

    pub fn commit(&self, request: Commit) -> Result<Document, Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        let document = self.change(request.expected_revision, |_| Ok(request.config))?;
        *preview = None;
        Ok(document)
    }

    pub fn edit(&self, edit: impl FnOnce(&mut Config)) -> Result<Document, Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        let document = self.change_current(None, |mut current| { edit(&mut current); Ok(current) })?;
        *preview = None;
        Ok(document)
    }

    fn change(&self, revision: u64, edit: impl FnOnce(Config) -> Result<Config, Error>) -> Result<Document, Error> {
        self.change_current(Some(revision), edit)
    }

    fn change_current(&self, revision: Option<u64>, edit: impl FnOnce(Config) -> Result<Config, Error>) -> Result<Document, Error> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| Error::new("not_initialized", "Open Luma once to migrate your existing preferences."))?;
        if revision.is_some_and(|revision| revision != current.revision) {
            return Err(Error::new("conflict", "Customization changed elsewhere. Read the current revision before applying your changes."));
        }
        let config = edit(current.config.clone())?;
        document::validate(&config).map_err(|error| Error::new("validation", error))?;
        if config == current.config { return Ok(current); }
        if current.revision >= 9_007_199_254_740_991 { return Err(Error::new("revision_limit", "Customization revision limit reached.")); }
        let document = Document { version: 3, revision: current.revision + 1, config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("UPDATE customization SET previous = document, document = ?1 WHERE id = 1", [raw])?;
        transaction.commit()?;
        Ok(document)
    }

    pub fn restore(&self, expected_revision: u64) -> Result<Document, Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| Error::new("not_initialized", "Open Luma once before restoring customization."))?;
        if current.revision != expected_revision { return Err(Error::new("conflict", "Customization changed elsewhere.")); }
        let raw: Option<String> = transaction.query_row("SELECT previous FROM customization WHERE id = 1", [], |row| row.get(0))?;
        let previous = Self::decode(&raw.ok_or_else(|| Error::new("no_previous", "No previous customization is available."))?)?;
        if current.revision >= 9_007_199_254_740_991 { return Err(Error::new("revision_limit", "Customization revision limit reached.")); }
        let document = Document { version: 3, revision: current.revision + 1, config: previous.config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("UPDATE customization SET previous = document, document = ?1 WHERE id = 1", params![raw])?;
        transaction.commit()?;
        *preview = None;
        Ok(document)
    }

    pub fn restore_layout(&self, expected_revision: u64) -> Result<Document, Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| Error::new("not_initialized", "Open Luma once before restoring a layout."))?;
        if current.revision != expected_revision { return Err(Error::new("conflict", "The saved layout changed. Reload it before restoring.")); }
        let raw: Option<String> = transaction.query_row("SELECT previous FROM customization WHERE id = 1", [], |row| row.get(0))?;
        let previous = Self::decode(&raw.ok_or_else(|| Error::new("no_previous", "No previous layout is available."))?)?;
        let mut config = current.config.clone();
        config.home = previous.config.home;
        config.home_columns = previous.config.home_columns;
        if config == current.config { return Ok(current); }
        if current.revision >= 9_007_199_254_740_991 { return Err(Error::new("revision_limit", "Customization revision limit reached.")); }
        let document = Document { version: 3, revision: current.revision + 1, config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("UPDATE customization SET previous = document, document = ?1 WHERE id = 1", params![raw])?;
        transaction.commit()?;
        *preview = None;
        Ok(document)
    }

    pub fn preview(&self) -> Result<Option<Preview>, Error> {
        Ok(self.1.lock().map_err(|error| Error::new("storage", error))?.clone())
    }

    pub fn stage(&self, request: Commit) -> Result<Preview, Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        if preview.is_some() { return Err(Error::new("preview_busy", "Finish or cancel the current preview first.")); }
        let current = self.read()?.ok_or_else(|| Error::new("not_initialized", "Open Luma once before previewing a layout."))?;
        if current.revision != request.expected_revision { return Err(Error::new("conflict", "The saved layout changed. Reload it before previewing your draft.")); }
        document::validate(&request.config).map_err(|error| Error::new("validation", error))?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| Error::new("storage", error))?;
        let candidate = Preview { id: bytes.iter().map(|byte| format!("{byte:02x}")).collect(), base_revision: current.revision, config: request.config };
        *preview = Some(candidate.clone());
        Ok(candidate)
    }

    pub fn keep(&self, id: &str) -> Result<Document, Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        let candidate = preview.as_ref().filter(|preview| preview.id == id).ok_or_else(|| Error::new("preview_mismatch", "This preview is no longer active. Preview your draft again."))?.clone();
        let document = self.change(candidate.base_revision, |_| Ok(candidate.config))?;
        *preview = None;
        Ok(document)
    }

    pub fn cancel(&self, id: &str) -> Result<(), Error> {
        let mut preview = self.1.lock().map_err(|error| Error::new("storage", error))?;
        if preview.as_ref().is_some_and(|preview| preview.id != id) {
            return Err(Error::new("preview_mismatch", "This preview is no longer active."));
        }
        *preview = None;
        Ok(())
    }

    pub fn clear_preview(&self) {
        if let Ok(mut preview) = self.1.lock() { *preview = None; }
    }
}

pub fn publish(app: &tauri::AppHandle, document: &Document) {
    let _ = crate::remote::server::sync_customization(app, document);
    let _ = app.emit_to("main", "customization-changed", document);
    publish_preview(app);
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
pub async fn customization_preview(app: tauri::AppHandle, request: Commit) -> Result<Preview, Error> {
    if !app.get_webview_window("main").is_some_and(|window| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)) {
        return Err(Error::new("launcher_unavailable", "Open Luma before previewing a layout."));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let preview = app.state::<Store>().stage(request)?;
        let visible = app.get_webview_window("main").is_some_and(|window| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true));
        if !visible || app.emit_to("main", "customization-preview-changed", Some(&preview)).is_err() {
            let _ = app.state::<Store>().cancel(&preview.id);
            return Err(Error::new("launcher_unavailable", "Luma closed before the preview could open."));
        }
        Ok(preview)
    }).await.map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_preview_keep(app: tauri::AppHandle, request: PreviewAction) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().keep(&request.preview_id)?;
        publish(&app, &document);
        Ok(document)
    }).await.map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_preview_cancel(app: tauri::AppHandle, request: PreviewAction) -> Result<(), Error> {
    app.state::<Store>().cancel(&request.preview_id)?;
    publish_preview(&app);
    Ok(())
}

#[tauri::command]
pub async fn customization_restore(app: tauri::AppHandle, expected_revision: u64) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().restore(expected_revision)?;
        publish(&app, &document);
        Ok(document)
    }).await.map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_restore_layout(app: tauri::AppHandle, expected_revision: u64) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().restore_layout(expected_revision)?;
        publish(&app, &document);
        Ok(document)
    }).await.map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_read(app: tauri::AppHandle) -> Result<Option<Document>, Error> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Store>().read()).await.map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_initialize(app: tauri::AppHandle, config: Config) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().initialize(config)?;
        publish(&app, &document);
        Ok(document)
    }).await.map_err(|error| Error::new("storage", error))?
}

#[tauri::command]
pub async fn customization_commit(app: tauri::AppHandle, request: Commit) -> Result<Document, Error> {
    tauri::async_runtime::spawn_blocking(move || {
        let document = app.state::<Store>().commit(request)?;
        publish(&app, &document);
        Ok(document)
    }).await.map_err(|error| Error::new("storage", error))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store { Store::from_connection(Connection::open_in_memory().unwrap()).unwrap() }

    #[test]
    fn initialization_is_explicit_and_preserves_first_migration() {
        let store = store();
        assert!(store.read().unwrap().is_none());
        assert_eq!(store.commit(Commit { expected_revision: 0, config: Config::default() }).unwrap_err().code, "not_initialized");
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
        let second = store.commit(Commit { expected_revision: 1, config }).unwrap();
        assert_eq!(second.revision, 2);
        assert_eq!(store.commit(Commit { expected_revision: 1, config: first.config.clone() }).unwrap_err().code, "conflict");
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
        let themed = store.commit(Commit { expected_revision: first.revision, config: themed }).unwrap();
        let mut rearranged = themed.config.clone();
        rearranged.home.reverse();
        let rearranged = store.commit(Commit { expected_revision: themed.revision, config: rearranged }).unwrap();
        let restored = store.restore_layout(rearranged.revision).unwrap();
        assert_eq!(restored.revision, rearranged.revision + 1);
        assert_eq!(restored.config.theme, "onyx");
        assert_eq!(restored.config.home, first.config.home);
        assert_eq!(restored.config.home_columns, first.config.home_columns);
        assert_eq!(restored.config.order, first.config.order);
    }

    #[test]
    fn invalid_and_failed_writes_preserve_current_document() {
        let store = store();
        let first = store.initialize(Config::default()).unwrap();
        let mut config = first.config.clone();
        config.home.pop();
        assert_eq!(store.commit(Commit { expected_revision: 1, config }).unwrap_err().code, "validation");
        store.connection().unwrap().execute_batch("CREATE TRIGGER reject_write BEFORE UPDATE ON customization BEGIN SELECT RAISE(ABORT, 'disk write rejected'); END;").unwrap();
        let mut config = first.config.clone();
        config.theme = "onyx".into();
        assert_eq!(store.commit(Commit { expected_revision: 1, config }).unwrap_err().code, "storage");
        assert_eq!(store.read().unwrap().unwrap(), first);
    }

    #[test]
    fn unsupported_documents_are_not_overwritten_by_migration() {
        let store = store();
        let raw = serde_json::to_string(&Document { version: 99, revision: 1, config: Config::default() }).unwrap();
        store.connection().unwrap().execute("INSERT INTO customization (id, document) VALUES (1, ?1)", [&raw]).unwrap();
        assert_eq!(store.initialize(Config::default()).unwrap_err().code, "unsupported_version");
        let saved: String = store.connection().unwrap().query_row("SELECT document FROM customization", [], |row| row.get(0)).unwrap();
        assert_eq!(saved, raw);
    }

    #[test]
    fn configuration_and_previous_revision_survive_reopening_storage() {
        let path = std::env::temp_dir().join(format!("luma-customization-{}-{}.sqlite3", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let first;
        {
            let store = Store::from_connection(Connection::open(&path).unwrap()).unwrap();
            first = store.initialize(Config::default()).unwrap();
            let mut config = first.config.clone();
            config.home.reverse();
            config.widgets.spotify.placement = "bottom-left".into();
            store.commit(Commit { expected_revision: 1, config }).unwrap();
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
        let store = Store(Err("Cannot open customization database".into()), Mutex::new(None));
        assert_eq!(store.read().unwrap_err().code, "storage");
        assert_eq!(store.initialize(Config::default()).unwrap_err().code, "storage");
        assert_eq!(store.commit(Commit { expected_revision: 1, config: Config::default() }).unwrap_err().code, "storage");
    }

    #[test]
    fn preview_is_temporary_and_keep_saves_the_exact_candidate_with_undo() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home_columns = 2;
        candidate.home[1].span = 1;
        let preview = store.stage(Commit { expected_revision: 1, config: candidate.clone() }).unwrap();
        assert_eq!(store.read().unwrap().unwrap(), original);
        assert_eq!(store.preview().unwrap().unwrap().config, candidate);
        let kept = store.keep(&preview.id).unwrap();
        assert_eq!(kept.revision, 2);
        assert_eq!(kept.config, candidate);
        assert!(store.preview().unwrap().is_none());
        assert_eq!(store.restore(2).unwrap().config, original.config);
    }

    #[test]
    fn cancel_and_closing_the_launcher_preserve_committed_preferences() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home.reverse();
        let first = store.stage(Commit { expected_revision: 1, config: candidate.clone() }).unwrap();
        assert_eq!(store.stage(Commit { expected_revision: 1, config: candidate.clone() }).unwrap_err().code, "preview_busy");
        store.cancel(&first.id).unwrap();
        let second = store.stage(Commit { expected_revision: 1, config: candidate }).unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(store.keep(&first.id).unwrap_err().code, "preview_mismatch");
        assert_eq!(store.cancel(&first.id).unwrap_err().code, "preview_mismatch");
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
        let preview = store.stage(Commit { expected_revision: 1, config: candidate }).unwrap();
        let changed = store.edit(|config| config.theme = "onyx".into()).unwrap();
        assert!(store.preview().unwrap().is_none());
        assert_eq!(store.keep(&preview.id).unwrap_err().code, "preview_mismatch");
        assert_eq!(store.read().unwrap().unwrap(), changed);
        assert_eq!(store.stage(Commit { expected_revision: 1, config: Config::default() }).unwrap_err().code, "conflict");
    }

    #[test]
    fn a_failed_keep_retains_the_reviewed_candidate_for_retry() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home_columns = 2;
        let preview = store.stage(Commit { expected_revision: 1, config: candidate.clone() }).unwrap();
        store.connection().unwrap().execute_batch("CREATE TRIGGER reject_keep BEFORE UPDATE ON customization BEGIN SELECT RAISE(ABORT, 'disk full'); END;").unwrap();
        assert_eq!(store.keep(&preview.id).unwrap_err().code, "storage");
        assert_eq!(store.read().unwrap().unwrap(), original);
        assert_eq!(store.preview().unwrap().unwrap().id, preview.id);
        store.connection().unwrap().execute_batch("DROP TRIGGER reject_keep;").unwrap();
        assert_eq!(store.keep(&preview.id).unwrap().config, candidate);
    }

    #[test]
    fn invalid_candidates_never_enter_preview() {
        let store = store();
        let original = store.initialize(Config::default()).unwrap();
        let mut candidate = original.config.clone();
        candidate.home[1].span = 3;
        assert_eq!(store.stage(Commit { expected_revision: 1, config: candidate }).unwrap_err().code, "validation");
        assert!(store.preview().unwrap().is_none());
        assert_eq!(store.read().unwrap().unwrap(), original);
    }

    fn legacy(revision: u64, config: Config) -> String {
        let mut raw = serde_json::to_value(Document { version: 1, revision, config }).unwrap();
        let raw_config = raw["config"].as_object_mut().unwrap();
        for field in ["homeColumns", "textSize", "reducedMotion", "interfaceSounds", "soundLevel"] { raw_config.remove(field); }
        for row in raw_config["home"].as_array_mut().unwrap() { row.as_object_mut().unwrap().remove("component"); row.as_object_mut().unwrap().remove("span"); }
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
            connection.execute("INSERT INTO customization (id, document, previous) VALUES (1, ?1, ?2)", params![raw, previous]).unwrap();
            Store::migrate(&mut connection).unwrap();
            let persisted: String = connection.query_row("SELECT document FROM customization", [], |row| row.get(0)).unwrap();
            assert_eq!(serde_json::from_str::<Document>(&persisted).unwrap().version, 3);
            Store::migrate(&mut connection).unwrap();
        }
        let migrated = store.read().unwrap().unwrap();
        assert_eq!(migrated.revision, 5);
        assert_eq!(migrated.config, config);
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
            let mut value = serde_json::to_value(Document { version: 2, revision, config }).unwrap();
            let value_config = value["config"].as_object_mut().unwrap();
            for field in ["homeColumns", "textSize", "reducedMotion", "interfaceSounds", "soundLevel"] { value_config.remove(field); }
            for row in value_config["home"].as_array_mut().unwrap() {
                row.as_object_mut().unwrap().remove("span");
            }
            value.to_string()
        };
        {
            let mut connection = store.connection().unwrap();
            connection.execute("INSERT INTO customization (id, document, previous) VALUES (1, ?1, ?2)", params![encode(8, config.clone()), encode(7, Config::default())]).unwrap();
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
        let future = serde_json::to_string(&Document { version: 99, revision: 4, config: Config::default() }).unwrap();
        let mut connection = store.connection().unwrap();
        connection.execute("INSERT INTO customization (id, document, previous) VALUES (1, ?1, ?2)", params![raw, future]).unwrap();
        assert_eq!(Store::migrate(&mut connection).unwrap_err().code, "unsupported_version");
        let saved: String = connection.query_row("SELECT document FROM customization", [], |row| row.get(0)).unwrap();
        assert_eq!(saved, raw);
    }
}
