// SQLite database wrapper for AgentHub Local.
//
// SQLite stores indexed views and history. The filesystem remains the
// authoritative source for agent configuration files.

pub mod migrations;

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use rusqlite::Connection;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(String),
}

pub type DbResult<T> = Result<T, DbError>;

/// Owns a single SQLite connection guarded by a mutex. The MVP workload is
/// low-concurrency so a single connection keeps the model simple while
/// preserving thread-safety across Tauri command invocations.
pub struct Database {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    /// Open or create the database at `path`, applying pending migrations.
    pub fn open(path: impl AsRef<Path>) -> DbResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DbError::Migration(e.to_string()))?;
        }
        let conn = Connection::open(&path)?;
        Self::configure(&conn)?;
        migrations::run(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    /// Open an in-memory database, primarily for tests.
    pub fn open_in_memory() -> DbResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        migrations::run(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    fn configure(conn: &Connection) -> DbResult<()> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;\n             PRAGMA foreign_keys = ON;\n             PRAGMA synchronous = NORMAL;",
        )?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn with_conn<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&Connection) -> T,
    {
        let guard = self.conn.lock();
        f(&guard)
    }

    pub fn with_conn_mut<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut Connection) -> T,
    {
        let mut guard = self.conn.lock();
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_and_applies_migrations() {
        let db = Database::open_in_memory().unwrap();
        let tables: Vec<String> = db.with_conn(|c| {
            let mut stmt = c
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .unwrap();
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            rows
        });
        for expected in [
            "agents",
            "backups",
            "change_sets",
            "config_scopes",
            "doctor_issues",
            "projects",
            "prompt_templates",
            "resource_bindings",
            "resources",
            "scan_snapshots",
            "settings",
        ] {
            assert!(
                tables.iter().any(|t| t == expected),
                "missing table {expected}; got {tables:?}"
            );
        }
    }

    #[test]
    fn reopen_preserves_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agenthub.sqlite3");
        {
            let db = Database::open(&path).unwrap();
            db.with_conn(|c| {
                c.execute(
                    "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                    ["seed", "ok", "2026-01-01T00:00:00Z"],
                )
                .unwrap();
            });
        }
        let db2 = Database::open(&path).unwrap();
        let value: String = db2.with_conn(|c| {
            c.query_row("SELECT value FROM settings WHERE key = ?1", ["seed"], |r| {
                r.get(0)
            })
            .unwrap()
        });
        assert_eq!(value, "ok");
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = Database::open_in_memory().unwrap();
        // Re-running migrations on the same connection must succeed.
        db.with_conn(|c| migrations::run(c).unwrap());
    }
}
