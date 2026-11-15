use super::models::{app_id, app_provider, ActivityItem, ActivityType, ProgressSource, ResumeKind};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::{path::Path, sync::Mutex, time::Duration};

pub struct Storage {
    connection: Mutex<Connection>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        Self::initialize(connection)
    }

    #[cfg(test)]
    pub fn memory() -> Result<Self, String> {
        Self::initialize(Connection::open_in_memory().map_err(|error| error.to_string())?)
    }

    fn initialize(connection: Connection) -> Result<Self, String> {
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;
                 CREATE TABLE IF NOT EXISTS activity_items (
                     id TEXT PRIMARY KEY,
                     provider TEXT NOT NULL,
                     provider_item_id TEXT,
                     profile_id TEXT,
                     item_type TEXT NOT NULL,
                     title TEXT NOT NULL,
                     subtitle TEXT,
                     artwork TEXT,
                     last_activity_at TEXT NOT NULL,
                     progress_seconds REAL,
                     duration_seconds REAL,
                     progress_source TEXT NOT NULL,
                     completed INTEGER,
                     can_resume INTEGER NOT NULL DEFAULT 0,
                     launch_target TEXT,
                     resume_kind TEXT NOT NULL DEFAULT 'none',
                     launch_count INTEGER NOT NULL DEFAULT 0,
                     metadata_updated_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS activity_items_recent
                     ON activity_items(last_activity_at DESC);
                 CREATE TABLE IF NOT EXISTS activity_preferences (
                     key TEXT PRIMARY KEY,
                     value TEXT NOT NULL
                 );",
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn upsert(&self, item: &ActivityItem) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO activity_items (
                     id, provider, provider_item_id, profile_id, item_type, title, subtitle,
                     artwork, last_activity_at, progress_seconds, duration_seconds,
                     progress_source, completed, can_resume, launch_target, resume_kind,
                     launch_count, metadata_updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(id) DO UPDATE SET
                     title = excluded.title,
                     subtitle = COALESCE(excluded.subtitle, activity_items.subtitle),
                     artwork = COALESCE(excluded.artwork, activity_items.artwork),
                     last_activity_at = CASE
                         WHEN excluded.last_activity_at > activity_items.last_activity_at
                         THEN excluded.last_activity_at ELSE activity_items.last_activity_at END,
                     progress_seconds = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN excluded.progress_seconds ELSE activity_items.progress_seconds END,
                     duration_seconds = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN excluded.duration_seconds ELSE activity_items.duration_seconds END,
                     progress_source = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN excluded.progress_source ELSE activity_items.progress_source END,
                     completed = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN excluded.completed ELSE activity_items.completed END,
                     can_resume = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN excluded.can_resume ELSE activity_items.can_resume END,
                     launch_target = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN COALESCE(excluded.launch_target, activity_items.launch_target)
                         ELSE activity_items.launch_target END,
                     resume_kind = CASE
                         WHEN excluded.last_activity_at >= activity_items.last_activity_at
                         THEN excluded.resume_kind ELSE activity_items.resume_kind END,
                     metadata_updated_at = excluded.metadata_updated_at",
                params![
                    item.id,
                    item.provider,
                    item.provider_item_id,
                    item.profile_id,
                    item.item_type.as_str(),
                    item.title,
                    item.subtitle,
                    item.artwork,
                    item.last_activity_at,
                    item.progress_seconds,
                    item.duration_seconds,
                    item.progress_source.as_str(),
                    item.completed.map(|value| if value { 1i64 } else { 0i64 }),
                    if item.can_resume { 1i64 } else { 0i64 },
                    item.launch_target,
                    item.resume_kind.as_str(),
                    i64::from(item.launch_count.unwrap_or(0)),
                    item.last_activity_at,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn record_launch(&self, app: &str, now: &str) -> Result<ActivityItem, String> {
        let id = app_id(app);
        let provider = app_provider(app);
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO activity_items (
                     id, provider, provider_item_id, profile_id, item_type, title, subtitle,
                     artwork, last_activity_at, progress_seconds, duration_seconds,
                     progress_source, completed, can_resume, launch_target, resume_kind,
                     launch_count, metadata_updated_at
                 ) VALUES (?1, ?2, ?3, NULL, 'application', ?4, NULL, NULL, ?5, NULL, NULL,
                     'unknown', NULL, 0, ?4, 'service', 1, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                     last_activity_at = excluded.last_activity_at,
                     launch_count = activity_items.launch_count + 1,
                     launch_target = excluded.launch_target,
                     resume_kind = excluded.resume_kind,
                     metadata_updated_at = excluded.metadata_updated_at",
                params![id, provider, app, app, now],
            )
            .map_err(|error| error.to_string())?;
        drop(connection);
        self.get(&id)?
            .ok_or_else(|| "The launch was not stored.".to_string())
    }

    pub fn get(&self, id: &str) -> Result<Option<ActivityItem>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT id, provider, provider_item_id, profile_id, item_type, title, subtitle,
                        artwork, last_activity_at, progress_seconds, duration_seconds,
                        progress_source, completed, can_resume, launch_target, resume_kind,
                        launch_count, metadata_updated_at
                 FROM activity_items WHERE id = ?1",
                params![id],
                read_item,
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn latest_for_provider(&self, provider: &str) -> Result<Option<ActivityItem>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT id, provider, provider_item_id, profile_id, item_type, title, subtitle,
                        artwork, last_activity_at, progress_seconds, duration_seconds,
                        progress_source, completed, can_resume, launch_target, resume_kind,
                        launch_count, metadata_updated_at
                 FROM activity_items WHERE provider = ?1
                 ORDER BY last_activity_at DESC LIMIT 1",
                params![provider],
                read_item,
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn list(&self, limit: usize) -> Result<Vec<ActivityItem>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id, provider, provider_item_id, profile_id, item_type, title, subtitle,
                        artwork, last_activity_at, progress_seconds, duration_seconds,
                        progress_source, completed, can_resume, launch_target, resume_kind,
                        launch_count, metadata_updated_at
                 FROM activity_items ORDER BY last_activity_at DESC LIMIT ?1",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![limit as i64], read_item)
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute("DELETE FROM activity_items WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn clear(&self, provider: Option<&str>) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        match provider {
            Some(provider) => connection.execute(
                "DELETE FROM activity_items WHERE provider = ?1",
                params![provider],
            ),
            None => connection.execute("DELETE FROM activity_items", []),
        }
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn count_playback(&self, provider: &str) -> Result<u32, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM activity_items
                 WHERE provider = ?1 AND item_type != 'application'",
                params![provider],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value.max(0) as u32)
            .map_err(|error| error.to_string())
    }

    pub fn preference(&self, key: &str) -> Result<Option<String>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .query_row(
                "SELECT value FROM activity_preferences WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn set_preference(&self, key: &str, value: &str) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO activity_preferences (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn remove_preference(&self, key: &str) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;
        connection
            .execute(
                "DELETE FROM activity_preferences WHERE key = ?1",
                params![key],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn bool_preference(&self, key: &str, default: bool) -> Result<bool, String> {
        Ok(self
            .preference(key)?
            .map(|value| value == "true")
            .unwrap_or(default))
    }

    pub fn set_bool_preference(&self, key: &str, value: bool) -> Result<(), String> {
        self.set_preference(key, if value { "true" } else { "false" })
    }
}

fn read_item(row: &Row<'_>) -> rusqlite::Result<ActivityItem> {
    let item_type = ActivityType::parse(&row.get::<_, String>(4)?).unwrap_or(ActivityType::Video);
    let progress_source =
        ProgressSource::parse(&row.get::<_, String>(11)?).unwrap_or(ProgressSource::Unknown);
    let resume_kind =
        ResumeKind::parse(&row.get::<_, String>(15)?).unwrap_or(ResumeKind::Unavailable);
    let completed = row.get::<_, Option<i64>>(12)?.map(|value| value != 0);
    let launch_count = row.get::<_, i64>(16)?;
    Ok(ActivityItem {
        id: row.get(0)?,
        provider: row.get(1)?,
        provider_item_id: row.get(2)?,
        profile_id: row.get(3)?,
        item_type,
        title: row.get(5)?,
        subtitle: row.get(6)?,
        artwork: row.get(7)?,
        last_activity_at: row.get(8)?,
        progress_seconds: row.get(9)?,
        duration_seconds: row.get(10)?,
        progress_source,
        completed,
        can_resume: row.get::<_, i64>(13)? != 0,
        launch_target: row.get(14)?,
        resume_kind,
        launch_count: (item_type == ActivityType::Application)
            .then_some(launch_count.max(0) as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, at: &str, position: f64) -> ActivityItem {
        ActivityItem {
            id: id.into(),
            provider: "plex".into(),
            provider_item_id: Some(id.into()),
            profile_id: None,
            item_type: ActivityType::Movie,
            title: "Example".into(),
            subtitle: None,
            artwork: None,
            last_activity_at: at.into(),
            progress_seconds: Some(position),
            duration_seconds: Some(100.0),
            progress_source: ProgressSource::Provider,
            completed: Some(false),
            can_resume: false,
            launch_target: Some("https://app.plex.tv/".into()),
            resume_kind: ResumeKind::Content,
            launch_count: None,
        }
    }

    #[test]
    fn application_launches_increment_and_keep_one_row() {
        let storage = Storage::memory().unwrap();
        let first = storage
            .record_launch("Spotify", "2026-01-01T00:00:00.000Z")
            .unwrap();
        let second = storage
            .record_launch("Spotify", "2026-01-02T00:00:00.000Z")
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.launch_count, Some(2));
        assert_eq!(storage.list(10).unwrap().len(), 1);
        assert_eq!(second.provider, "spotify");
        assert_eq!(second.item_type, ActivityType::Application);
    }

    #[test]
    fn stale_provider_refresh_does_not_move_progress_backwards() {
        let storage = Storage::memory().unwrap();
        storage
            .upsert(&item("plex:1", "2026-01-02T00:00:00.000Z", 80.0))
            .unwrap();
        storage
            .upsert(&item("plex:1", "2026-01-01T00:00:00.000Z", 5.0))
            .unwrap();
        let stored = storage.get("plex:1").unwrap().unwrap();
        assert_eq!(stored.progress_seconds, Some(80.0));
        assert_eq!(stored.last_activity_at, "2026-01-02T00:00:00.000Z");
    }

    #[test]
    fn preferences_are_local_and_removable() {
        let storage = Storage::memory().unwrap();
        assert!(storage.bool_preference("tracking", true).unwrap());
        storage.set_bool_preference("tracking", false).unwrap();
        assert!(!storage.bool_preference("tracking", true).unwrap());
        storage.remove_preference("tracking").unwrap();
        assert!(storage.bool_preference("tracking", true).unwrap());
    }
}
