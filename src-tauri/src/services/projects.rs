// ProjectService: CRUD-light operations on the `projects` table.
//
// Project rows are the user-supplied filesystem roots that ScanService
// walks. We validate that the path exists and is a directory, then
// canonicalize it so duplicate adds are detected even when the user
// types "/foo/." vs "/foo".

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};
use crate::domain::Project;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project path does not exist: {0}")]
    NotFound(String),
    #[error("project path is not a directory: {0}")]
    NotDirectory(String),
    #[error("project already exists: {0}")]
    Duplicate(String),
    #[error("project not found: {0}")]
    UnknownId(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("db error: {0}")]
    Db(#[from] DbError),
}

pub type ProjectResult<T> = Result<T, ProjectError>;

#[derive(Clone)]
pub struct ProjectService {
    db: Arc<Database>,
}

impl ProjectService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn add(&self, raw_path: &str, name: Option<&str>) -> ProjectResult<Project> {
        let path = Path::new(raw_path);
        if !path.exists() {
            return Err(ProjectError::NotFound(raw_path.to_string()));
        }
        if !path.is_dir() {
            return Err(ProjectError::NotDirectory(raw_path.to_string()));
        }
        let canonical: PathBuf = path.canonicalize()?;
        let canonical_str = canonical.to_string_lossy().to_string();

        let existing = self
            .db
            .with_conn(|c| -> rusqlite::Result<Option<String>> {
                c.query_row(
                    "SELECT id FROM projects WHERE path = ?1",
                    params![canonical_str],
                    |r| r.get::<_, String>(0),
                )
                .optional()
            })
            .map_err(DbError::from)?;
        if existing.is_some() {
            return Err(ProjectError::Duplicate(canonical_str));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let derived_name = name
            .map(|s| s.to_string())
            .unwrap_or_else(|| derive_name(&canonical));

        self.db
            .with_conn(|c| -> rusqlite::Result<()> {
                c.execute(
                    "INSERT INTO projects (id, name, path, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, derived_name, canonical_str, now, now],
                )?;
                Ok(())
            })
            .map_err(DbError::from)?;

        Ok(Project {
            id,
            name: derived_name,
            path: canonical_str,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list(&self) -> ProjectResult<Vec<Project>> {
        let rows = self
            .db
            .with_conn(|c| -> rusqlite::Result<Vec<Project>> {
                let mut stmt = c.prepare(
                    "SELECT id, name, path, created_at, updated_at FROM projects ORDER BY created_at DESC",
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(Project {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        path: r.get(2)?,
                        created_at: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                })?;
                let mut out = Vec::new();
                for p in iter {
                    out.push(p?);
                }
                Ok(out)
            })
            .map_err(DbError::from)?;
        Ok(rows)
    }

    pub fn get(&self, id: &str) -> ProjectResult<Project> {
        let project = self
            .db
            .with_conn(|c| -> rusqlite::Result<Option<Project>> {
                c.query_row(
                    "SELECT id, name, path, created_at, updated_at FROM projects WHERE id = ?1",
                    params![id],
                    |r| {
                        Ok(Project {
                            id: r.get(0)?,
                            name: r.get(1)?,
                            path: r.get(2)?,
                            created_at: r.get(3)?,
                            updated_at: r.get(4)?,
                        })
                    },
                )
                .optional()
            })
            .map_err(DbError::from)?;
        project.ok_or_else(|| ProjectError::UnknownId(id.to_string()))
    }

    pub fn remove(&self, id: &str) -> ProjectResult<()> {
        let n = self
            .db
            .with_conn(|c| -> rusqlite::Result<usize> {
                c.execute("DELETE FROM projects WHERE id = ?1", params![id])
            })
            .map_err(DbError::from)?;
        if n == 0 {
            return Err(ProjectError::UnknownId(id.to_string()));
        }
        Ok(())
    }
}

fn derive_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn service() -> (Arc<Database>, ProjectService) {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let svc = ProjectService::new(db.clone());
        (db, svc)
    }

    #[test]
    fn add_and_list() {
        let (_, svc) = service();
        let dir = tempdir().unwrap();
        let p = svc.add(dir.path().to_str().unwrap(), None).unwrap();
        let list = svc.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, p.id);
    }

    #[test]
    fn rejects_missing_path() {
        let (_, svc) = service();
        let err = svc.add("/this/path/should/not/exist/zz", None).unwrap_err();
        assert!(matches!(err, ProjectError::NotFound(_)));
    }

    #[test]
    fn rejects_duplicate_after_canonicalization() {
        let (_, svc) = service();
        let dir = tempdir().unwrap();
        svc.add(dir.path().to_str().unwrap(), None).unwrap();
        // Re-add through a path with `/.` suffix; canonicalize collapses it.
        let with_dot = format!("{}/.", dir.path().to_str().unwrap());
        let err = svc.add(&with_dot, None).unwrap_err();
        assert!(matches!(err, ProjectError::Duplicate(_)));
    }

    #[test]
    fn get_returns_project() {
        let (_, svc) = service();
        let dir = tempdir().unwrap();
        let p = svc
            .add(dir.path().to_str().unwrap(), Some("Custom"))
            .unwrap();
        let got = svc.get(&p.id).unwrap();
        assert_eq!(got.name, "Custom");
    }

    #[test]
    fn remove_deletes_row() {
        let (_, svc) = service();
        let dir = tempdir().unwrap();
        let p = svc.add(dir.path().to_str().unwrap(), None).unwrap();
        svc.remove(&p.id).unwrap();
        assert!(svc.get(&p.id).is_err());
    }
}
