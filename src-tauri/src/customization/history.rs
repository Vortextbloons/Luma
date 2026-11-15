use super::{
    document::{Config, Document},
    Error,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

const LIMIT: i64 = 10;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub revision: u64,
    pub saved_at: Option<String>,
    pub config: Config,
}

pub(super) fn create(connection: &Connection) -> Result<(), Error> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS versions (revision INTEGER PRIMARY KEY, saved_at TEXT, document TEXT NOT NULL);")?;
    Ok(())
}

pub(super) fn migrate(transaction: &Connection, current: Option<&Document>, previous: Option<&Document>) -> Result<(), Error> {
    let mut statement = transaction.prepare("SELECT revision, document FROM versions ORDER BY revision DESC")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
    let mut entries = Vec::new();
    for row in rows {
        let (revision, raw) = row?;
        let Ok(document) = super::Store::decode(&raw) else { continue };
        if i64::try_from(document.revision).ok() != Some(revision) { continue; }
        entries.push((revision, raw, document));
    }
    drop(statement);
    for (revision, raw, document) in entries {
        let canonical = serde_json::to_string(&document).map_err(|error| Error::new("invalid_document", error))?;
        if canonical != raw {
            transaction.execute("UPDATE versions SET document = ?1 WHERE revision = ?2", params![canonical, revision])?;
        }
    }
    for document in current.into_iter().chain(previous) {
        let raw = serde_json::to_string(document).map_err(|error| Error::new("invalid_document", error))?;
        transaction.execute("INSERT OR IGNORE INTO versions (revision, saved_at, document) VALUES (?1, NULL, ?2)", params![document.revision, raw])?;
    }
    prune(transaction)
}

pub(super) fn record(transaction: &Connection, document: &Document) -> Result<(), Error> {
    let raw = serde_json::to_string(document).map_err(|error| Error::new("invalid_document", error))?;
    let saved_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    transaction.execute("INSERT INTO versions (revision, saved_at, document) VALUES (?1, ?2, ?3)", params![document.revision, saved_at, raw])?;
    prune(transaction)
}

fn prune(transaction: &Connection) -> Result<(), Error> {
    transaction.execute("DELETE FROM versions WHERE revision NOT IN (SELECT revision FROM versions ORDER BY revision DESC LIMIT ?1)", [LIMIT])?;
    Ok(())
}

pub(super) fn list(connection: &Connection) -> Result<Vec<HistoryEntry>, Error> {
    let mut statement = connection.prepare("SELECT revision, saved_at, document FROM versions ORDER BY revision DESC")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?)))?;
    let mut entries = Vec::new();
    for row in rows {
        let (revision, saved_at, raw) = row?;
        let Ok(document) = super::Store::decode(&raw) else { continue };
        if i64::try_from(document.revision).ok() != Some(revision) { continue; }
        entries.push(HistoryEntry { revision: document.revision, saved_at, config: document.config });
    }
    Ok(entries)
}

pub(super) fn find(connection: &Connection, revision: u64) -> Result<Option<Document>, Error> {
    let raw: Option<String> = connection.query_row("SELECT document FROM versions WHERE revision = ?1", [revision], |row| row.get(0)).optional()?;
    raw.as_deref().map(super::Store::decode).transpose()
}

pub(super) fn previous(connection: &Connection, revision: u64) -> Result<Option<Document>, Error> {
    let raw: Option<String> = connection.query_row("SELECT document FROM versions WHERE revision < ?1 ORDER BY revision DESC LIMIT 1", [revision], |row| row.get(0)).optional()?;
    raw.as_deref().map(super::Store::decode).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(revision: u64) -> HistoryEntry {
        let mut config = Config::default();
        config.home.reverse();
        config.home[0].visible = false;
        HistoryEntry { revision, saved_at: None, config }
    }

    #[test]
    fn entries_serialize_as_camel_case_metadata_and_configuration() {
        let raw = serde_json::to_string(&entry(4)).unwrap();
        assert!(raw.starts_with("{\"revision\":4,\"savedAt\":null,\"config\":{"));
        assert!(raw.contains("\"homeColumns\":1"));
        assert!(!raw.contains("\"saved_at\""));
    }

    #[test]
    fn table_rejects_duplicate_revisions_and_trims_beyond_ten() {
        let connection = Connection::open_in_memory().unwrap();
        create(&connection).unwrap();
        for revision in 1..=12 {
            let document = Document { version: 3, revision, config: Config::default() };
            record(&connection, &document).unwrap();
        }
        let revisions: Vec<u64> = list(&connection).unwrap().into_iter().map(|entry| entry.revision).collect();
        assert_eq!(revisions, (3..=12).rev().collect::<Vec<_>>());
        assert_eq!(find(&connection, 12).unwrap().unwrap().revision, 12);
        assert!(find(&connection, 2).unwrap().is_none());
        assert_eq!(previous(&connection, 12).unwrap().unwrap().revision, 11);
    }

    #[test]
    fn migration_canonicalizes_supported_rows_and_preserves_unsupported_ones() {
        let connection = Connection::open_in_memory().unwrap();
        create(&connection).unwrap();
        let mut legacy = serde_json::to_value(Document { version: 3, revision: 2, config: Config::default() }).unwrap();
        legacy["config"].as_object_mut().unwrap().remove("textSize");
        let unsupported = serde_json::to_string(&Document { version: 99, revision: 3, config: Config::default() }).unwrap();
        connection.execute("INSERT INTO versions (revision, saved_at, document) VALUES (2, NULL, ?1), (3, NULL, ?2)", params![legacy.to_string(), unsupported]).unwrap();
        migrate(&connection, None, None).unwrap();
        let revisions: Vec<u64> = list(&connection).unwrap().into_iter().map(|entry| entry.revision).collect();
        assert_eq!(revisions, vec![2]);
        let saved: String = connection.query_row("SELECT document FROM versions WHERE revision = 2", [], |row| row.get(0)).unwrap();
        assert!(saved.contains("\"textSize\":\"standard\""));
        let saved: String = connection.query_row("SELECT document FROM versions WHERE revision = 3", [], |row| row.get(0)).unwrap();
        assert_eq!(saved, unsupported);
    }
}
