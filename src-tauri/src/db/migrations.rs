use rusqlite::Connection;

use super::{DbError, DbResult};

struct Migration {
    version: u32,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial",
    sql: include_str!("../../migrations/0001_initial.sql"),
}];

/// Apply pending migrations using SQLite's `user_version` pragma as the marker.
pub fn run(conn: &Connection) -> DbResult<()> {
    let current: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        conn.execute_batch(migration.sql).map_err(|e| {
            DbError::Migration(format!(
                "failed migration {}_{}: {e}",
                migration.version, migration.name
            ))
        })?;
        // PRAGMA does not accept bound parameters, so format the version inline.
        conn.execute_batch(&format!("PRAGMA user_version = {}", migration.version))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn user_version_reflects_latest_migration() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let v: u32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.last().unwrap().version);
    }
}
