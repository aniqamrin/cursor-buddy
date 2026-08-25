use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const CURRENT_VERSION: i64 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: i64,
    pub title: String,
    pub created_at: String,
    pub message_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageRow {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("open db: {e}"))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;

        // A missing row means fresh install; any OTHER error (e.g. a
        // transient lock left by a previous instance) must abort loudly —
        // treating it as "fresh" would attempt to recreate existing tables.
        // `value` is stored as TEXT, so read it as String and parse.
        let version_text: String = match conn.query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| r.get(0),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(()), // fresh db
            Err(e) => return Err(format!("read schema version: {e}")),
        };
        let version: i64 = version_text.trim().parse().unwrap_or(0);

        if version < 1 {
            conn.execute_batch(
                "BEGIN;
                 CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS conversations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL DEFAULT 'New chat',
                    created_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    conversation_id INTEGER NOT NULL REFERENCES conversations(id),
                    role TEXT NOT NULL CHECK(role IN ('user','assistant','system')),
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
                 INSERT INTO schema_meta(key, value) VALUES ('version', '1')
                    ON CONFLICT(key) DO UPDATE SET value = excluded.value;
                 COMMIT;",
            )
            .map_err(|e| format!("migrate v1: {e}"))?;
        }

        let _ = CURRENT_VERSION;
        Ok(())
    }

    /// Lock and access the connection. Poisoned mutex (a panic mid-statement)
    /// recovers to the inner connection: SQLite is crash-safe by design.
    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    // ---- settings ----

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.lock()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        self.lock()
            .execute(
                "INSERT INTO settings(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|e| format!("save setting: {e}"))?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<(), String> {
        self.lock()
            .execute("DELETE FROM settings WHERE key = ?1", params![key])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ---- conversations ----

    pub fn create_conversation(&self, title: &str) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO conversations(title, created_at) VALUES (?1, ?2)",
            params![title, now()],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn set_conversation_title(&self, id: i64, title: &str) -> Result<(), String> {
        self.lock()
            .execute(
                "UPDATE conversations SET title = ?2 WHERE id = ?1",
                params![id, title],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_message(
        &self,
        conversation_id: i64,
        role: &str,
        content: &str,
    ) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO messages(conversation_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![conversation_id, role, content, now()],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// Most recent messages of a conversation in chronological order.
    pub fn recent_messages(
        &self,
        conversation_id: i64,
        limit: usize,
    ) -> Result<Vec<(String, String)>, String> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
                "SELECT role, content FROM (
                    SELECT * FROM messages WHERE conversation_id = ?1
                    ORDER BY id DESC LIMIT ?2
                 ) ORDER BY id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![conversation_id, limit as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn list_conversations(&self) -> Result<Vec<ConversationSummary>, String> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
                "SELECT c.id, c.title, c.created_at,
                        (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id)
                 FROM conversations c ORDER BY c.id DESC LIMIT 200",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ConversationSummary {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    created_at: r.get(2)?,
                    message_count: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn messages_for(&self, conversation_id: i64) -> Result<Vec<MessageRow>, String> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
                "SELECT id, conversation_id, role, content, created_at
                 FROM messages WHERE conversation_id = ?1 ORDER BY id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![conversation_id], |r| {
                Ok(MessageRow {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    role: r.get(2)?,
                    content: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn clear_history(&self) -> Result<u64, String> {
        let conn = self.lock();
        conn.execute("DELETE FROM messages", [])
            .map_err(|e| e.to_string())?;
        let msgs = conn.changes() as u64;
        conn.execute("DELETE FROM conversations", [])
            .map_err(|e| e.to_string())?;
        Ok(msgs)
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}
