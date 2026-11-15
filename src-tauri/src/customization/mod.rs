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

pub struct Store(Result<Mutex<Connection>, String>);

impl Store {
    pub fn open(app: &tauri::AppHandle) -> Self {
        Self::load(app).unwrap_or_else(|error| Self(Err(error)))
    }

    fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let directory = app.path().app_config_dir().map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        Self::from_connection(Connection::open(directory.join("customization.sqlite3")).map_err(|error| error.to_string())?)
            .map_err(|error| error.message)
    }

    fn from_connection(connection: Connection) -> Result<Self, Error> {
        connection.execute_batch("CREATE TABLE IF NOT EXISTS customization (id INTEGER PRIMARY KEY CHECK(id = 1), document TEXT NOT NULL, previous TEXT);")?;
        Ok(Self(Ok(Mutex::new(connection))))
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, Error> {
        self.0.as_ref().map_err(|error| Error::new("storage", error))?
            .lock().map_err(|error| Error::new("storage", error))
    }

    fn decode(value: &str) -> Result<Document, Error> {
        let document: Document = serde_json::from_str(value).map_err(|error| Error::new("invalid_document", error))?;
        if document.version != 1 { return Err(Error::new("unsupported_version", "This customization document requires a different version of Luma.")); }
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
        let document = Document { version: 1, revision: 1, config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("INSERT INTO customization (id, document) VALUES (1, ?1)", [raw])?;
        transaction.commit()?;
        Ok(document)
    }

    pub fn commit(&self, request: Commit) -> Result<Document, Error> {
        self.change(request.expected_revision, |_| Ok(request.config))
    }

    pub fn edit(&self, edit: impl FnOnce(&mut Config)) -> Result<Document, Error> {
        self.change_current(None, |mut current| { edit(&mut current); Ok(current) })
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
        let document = Document { version: 1, revision: current.revision + 1, config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("UPDATE customization SET previous = document, document = ?1 WHERE id = 1", [raw])?;
        transaction.commit()?;
        Ok(document)
    }

    pub fn restore(&self, expected_revision: u64) -> Result<Document, Error> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current = Self::current(&transaction)?.ok_or_else(|| Error::new("not_initialized", "Open Luma once before restoring customization."))?;
        if current.revision != expected_revision { return Err(Error::new("conflict", "Customization changed elsewhere.")); }
        let raw: Option<String> = transaction.query_row("SELECT previous FROM customization WHERE id = 1", [], |row| row.get(0))?;
        let previous = Self::decode(&raw.ok_or_else(|| Error::new("no_previous", "No previous customization is available."))?)?;
        if current.revision >= 9_007_199_254_740_991 { return Err(Error::new("revision_limit", "Customization revision limit reached.")); }
        let document = Document { version: 1, revision: current.revision + 1, config: previous.config };
        let raw = serde_json::to_string(&document).map_err(|error| Error::new("validation", error))?;
        transaction.execute("UPDATE customization SET previous = document, document = ?1 WHERE id = 1", params![raw])?;
        transaction.commit()?;
        Ok(document)
    }
}

pub fn publish(app: &tauri::AppHandle, document: &Document) {
    let _ = crate::remote::server::sync_customization(app, document);
    let _ = app.emit_to("main", "customization-changed", document);
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
        let store = Store(Err("Cannot open customization database".into()));
        assert_eq!(store.read().unwrap_err().code, "storage");
        assert_eq!(store.initialize(Config::default()).unwrap_err().code, "storage");
        assert_eq!(store.commit(Commit { expected_revision: 1, config: Config::default() }).unwrap_err().code, "storage");
    }
}
