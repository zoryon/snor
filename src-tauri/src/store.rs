use rusqlite::{params, Connection, OpenFlags};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

pub type AppResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub model: String,
    pub mode: String,
    pub project_id: Option<String>,
    pub cwd: Option<String>,
    pub session_id: Option<String>,
    pub pinned: bool,
    pub archived: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// One writer and one reader connection. In WAL mode readers do not block the writer,
/// so a slow history search never stalls the streaming checkpoints of the active turn.
pub struct Store {
    write: Mutex<Connection>,
    read: Mutex<Connection>,
}

fn db_err(e: impl ToString) -> String {
    e.to_string()
}

fn parse<T: DeserializeOwned>(body: rusqlite::Result<String>) -> AppResult<T> {
    serde_json::from_str(&body.map_err(db_err)?).map_err(db_err)
}

impl Store {
    pub fn open(path: &Path) -> AppResult<Self> {
        let write = Connection::open(path).map_err(db_err)?;
        // synchronous=NORMAL is durable against application crashes in WAL mode and avoids
        // an fsync on every checkpoint of a streaming reply.
        write.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;
            PRAGMA busy_timeout=3000; PRAGMA temp_store=MEMORY;
            CREATE TABLE IF NOT EXISTS chats (id TEXT PRIMARY KEY, body TEXT NOT NULL, updated_at INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS chats_updated ON chats(updated_at);
            CREATE TABLE IF NOT EXISTS messages (seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL,
                chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE, body TEXT NOT NULL, text TEXT NOT NULL DEFAULT '');
            CREATE INDEX IF NOT EXISTS messages_chat ON messages(chat_id, seq);
            CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id=1), body TEXT NOT NULL);
            PRAGMA user_version=1;").map_err(db_err)?;
        let read = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(db_err)?;
        read.execute_batch("PRAGMA busy_timeout=3000; PRAGMA temp_store=MEMORY;")
            .map_err(db_err)?;
        Ok(Self {
            write: Mutex::new(write),
            read: Mutex::new(read),
        })
    }

    fn writer(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.write.lock().map_err(db_err)
    }

    fn reader(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.read.lock().map_err(db_err)
    }

    pub fn chats(&self, search: &str) -> AppResult<Vec<Chat>> {
        let conn = self.reader()?;
        if search.is_empty() {
            let mut stmt = conn
                .prepare_cached("SELECT body FROM chats ORDER BY updated_at DESC LIMIT 200")
                .map_err(db_err)?;
            let rows = stmt.query_map([], |r| r.get(0)).map_err(db_err)?;
            return rows.map(parse).collect();
        }
        // EXISTS stops at the first matching message of each chat and the updated_at index
        // lets SQLite stop after 200 matching chats, instead of joining every message.
        let mut stmt = conn
            .prepare_cached(
                "SELECT c.body FROM chats c
                WHERE json_extract(c.body,'$.title') LIKE ?1 ESCAPE '!'
                    OR EXISTS (SELECT 1 FROM messages m WHERE m.chat_id=c.id AND m.text LIKE ?1 ESCAPE '!')
                ORDER BY c.updated_at DESC LIMIT 200",
            )
            .map_err(db_err)?;
        let pattern = format!(
            "%{}%",
            search
                .replace('!', "!!")
                .replace('%', "!%")
                .replace('_', "!_")
        );
        let rows = stmt.query_map([pattern], |r| r.get(0)).map_err(db_err)?;
        rows.map(parse).collect()
    }

    pub fn chat(&self, id: &str) -> AppResult<Chat> {
        parse(
            self.reader()?
                .prepare_cached("SELECT body FROM chats WHERE id=?")
                .map_err(db_err)?
                .query_row([id], |r| r.get(0)),
        )
    }

    pub fn put_chat(&self, chat: &Chat) -> AppResult<()> {
        let body = serde_json::to_string(chat).map_err(db_err)?;
        self.writer()?
            .prepare_cached("INSERT INTO chats(id,body,updated_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET body=excluded.body, updated_at=excluded.updated_at")
            .map_err(db_err)?
            .execute(params![chat.id, body, chat.updated_at])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn put_message(&self, chat_id: &str, message: &Value) -> AppResult<()> {
        let id = message["id"].as_str().ok_or("Message ID mancante")?;
        let body = serde_json::to_string(message).map_err(db_err)?;
        if body.len() > 12 * 1024 * 1024 {
            return Err("Messaggio troppo grande".into());
        }
        self.writer()?
            .prepare_cached("INSERT INTO messages(id,chat_id,body,text) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET body=excluded.body,text=excluded.text WHERE messages.chat_id=excluded.chat_id")
            .map_err(db_err)?
            .execute(params![id, chat_id, body, message["text"].as_str().unwrap_or("")])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn messages(&self, chat_id: &str, before: Option<i64>) -> AppResult<Value> {
        let conn = self.reader()?;
        let mut stmt = conn
            .prepare_cached("SELECT seq,body FROM messages WHERE chat_id=?1 AND seq<?2 ORDER BY seq DESC LIMIT 61")
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![chat_id, before.unwrap_or(i64::MAX)], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db_err)?;
        let mut data: Vec<(i64, String)> = rows.collect::<Result<_, _>>().map_err(db_err)?;
        let more = data.len() > 60;
        data.truncate(60);
        data.reverse();
        let cursor = data.first().map(|v| v.0);
        let messages: Vec<Value> = data
            .into_iter()
            .map(|(_, b)| serde_json::from_str(&b).map_err(db_err))
            .collect::<Result<_, _>>()?;
        Ok(serde_json::json!({"messages":messages,"cursor":cursor,"hasMore":more}))
    }

    pub fn delete_chat(&self, id: &str) -> AppResult<()> {
        self.writer()?
            .execute("DELETE FROM chats WHERE id=?", [id])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn projects(&self) -> AppResult<Vec<Value>> {
        let conn = self.reader()?;
        let mut stmt = conn
            .prepare_cached("SELECT body FROM projects ORDER BY json_extract(body,'$.name')")
            .map_err(db_err)?;
        let rows = stmt.query_map([], |r| r.get(0)).map_err(db_err)?;
        rows.map(parse).collect()
    }

    pub fn project(&self, id: &str) -> Option<Value> {
        parse(
            self.reader()
                .ok()?
                .prepare_cached("SELECT body FROM projects WHERE id=?")
                .ok()?
                .query_row([id], |r| r.get(0)),
        )
        .ok()
    }

    pub fn put_project(&self, project: &Value) -> AppResult<()> {
        let id = project["id"].as_str().ok_or("Project ID mancante")?;
        self.writer()?.execute("INSERT INTO projects(id,body) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",
            params![id, project.to_string()]).map_err(db_err)?;
        Ok(())
    }

    pub fn settings(&self) -> Value {
        self.reader()
            .ok()
            .and_then(|c| {
                c.query_row("SELECT body FROM settings WHERE id=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .ok()
            })
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    }

    pub fn put_settings(&self, settings: &Value) -> AppResult<()> {
        self.writer()?.execute("INSERT INTO settings(id,body) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET body=excluded.body", [settings.to_string()]).map_err(db_err)?;
        Ok(())
    }

    /// Updates only the session ID in place, so a concurrent rename or pin from the UI
    /// is not overwritten by a stale copy of the chat.
    pub fn set_session(&self, chat_id: &str, session_id: &str) -> AppResult<()> {
        self.writer()?
            .execute(
                "UPDATE chats SET body=json_set(body,'$.sessionId',?2) WHERE id=?1",
                params![chat_id, session_id],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn export(&self, chat_id: &str) -> AppResult<Value> {
        let chat = self.chat(chat_id)?;
        let conn = self.reader()?;
        let mut stmt = conn
            .prepare("SELECT body FROM messages WHERE chat_id=? ORDER BY seq")
            .map_err(db_err)?;
        let rows = stmt.query_map([chat_id], |r| r.get(0)).map_err(db_err)?;
        let messages: Vec<Value> = rows.map(parse).collect::<Result<_, _>>()?;
        Ok(serde_json::json!({"version":1,"chat":chat,"messages":messages}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reader is a second connection, so tests need a real file rather than `:memory:`.
    struct TempDb(std::path::PathBuf);
    impl TempDb {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("snor-test-{}.sqlite3", uuid::Uuid::new_v4())))
        }
    }
    impl Drop for TempDb {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
            }
        }
    }

    fn chat() -> Chat {
        serde_json::from_value(serde_json::json!({"id":"a","title":"Test","model":"sonnet","mode":"chat","projectId":null,"cwd":null,"sessionId":null,"pinned":false,"archived":false,"createdAt":1,"updatedAt":1})).unwrap()
    }

    #[test]
    fn pagination_search_and_delete_preserve_integrity() {
        let file = TempDb::new();
        let db = Store::open(&file.0).unwrap();
        db.put_chat(&chat()).unwrap();
        for i in 0..65 {
            db.put_message(
                "a",
                &serde_json::json!({"id":format!("m{i}"),"text":format!("unique needle {i}")}),
            )
            .unwrap();
        }
        let page = db.messages("a", None).unwrap();
        assert_eq!(page["messages"].as_array().unwrap().len(), 60);
        assert_eq!(page["hasMore"], true);
        assert_eq!(
            db.messages("a", page["cursor"].as_i64()).unwrap()["messages"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(db.chats("needle").unwrap().len(), 1);
        assert_eq!(db.chats("Test").unwrap().len(), 1);
        assert_eq!(db.chats("%").unwrap().len(), 0);
        db.delete_chat("a").unwrap();
        assert_eq!(
            db.messages("a", None).unwrap()["messages"],
            serde_json::json!([])
        );
    }

    #[test]
    fn session_update_keeps_concurrent_chat_changes() {
        let file = TempDb::new();
        let db = Store::open(&file.0).unwrap();
        db.put_chat(&chat()).unwrap();
        let mut renamed = db.chat("a").unwrap();
        renamed.title = "Rinominata".into();
        db.put_chat(&renamed).unwrap();
        db.set_session("a", "s1").unwrap();
        let stored = db.chat("a").unwrap();
        assert_eq!(stored.title, "Rinominata");
        assert_eq!(stored.session_id.as_deref(), Some("s1"));
    }
}
