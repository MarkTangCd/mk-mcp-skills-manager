// BackupService: creates snapshots of target files before any write is
// applied.  Each backup is a directory under the app-data `backups` folder
// containing a `manifest.json` and copies of the original files.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};
use crate::domain::Backup;
use crate::services::AppDataService;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("backup not found: {0}")]
    NotFound(String),
    #[error("target file not found: {0}")]
    TargetNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("db error: {0}")]
    Db(#[from] DbError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type BackupResult<T> = Result<T, BackupError>;

/// Manifest describing a single backup snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub change_set_id: String,
    pub created_at: String,
    pub files: Vec<BackupFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFileEntry {
    pub original_path: String,
    pub backup_path: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Clone)]
pub struct BackupService {
    db: Arc<Database>,
    backups_dir: PathBuf,
}

impl BackupService {
    pub fn new(db: Arc<Database>, app_data: &AppDataService) -> Self {
        Self {
            db,
            backups_dir: app_data.layout().backups.clone(),
        }
    }

    /// Create a backup for the given change set and target files.
    ///
    /// For files that do not exist on disk, an entry is still recorded with
    /// size 0 and an empty hash so the restore process can distinguish
    /// "file did not exist before" from "missing backup".
    pub fn create(&self, change_set_id: &str, target_files: &[PathBuf]) -> BackupResult<Backup> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let backup_dir = self.backups_dir.join(&id);
        fs::create_dir_all(&backup_dir)?;

        let mut entries = Vec::with_capacity(target_files.len());
        for original in target_files {
            let entry = if original.exists() {
                let hash = file_sha256(original)?;
                let size = fs::metadata(original)?.len();
                let file_name = original.file_name().unwrap_or_default().to_string_lossy();
                let backup_path = backup_dir.join(&*file_name);
                fs::copy(original, &backup_path)?;
                BackupFileEntry {
                    original_path: original.to_string_lossy().to_string(),
                    backup_path: backup_path.to_string_lossy().to_string(),
                    hash,
                    size,
                }
            } else {
                // Record that the file did not exist at backup time.
                BackupFileEntry {
                    original_path: original.to_string_lossy().to_string(),
                    backup_path: String::new(),
                    hash: String::new(),
                    size: 0,
                }
            };
            entries.push(entry);
        }

        let manifest = BackupManifest {
            change_set_id: change_set_id.to_string(),
            created_at: now.clone(),
            files: entries,
        };
        let manifest_path = backup_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        let manifest_path_str = manifest_path.to_string_lossy().to_string();
        self.db
            .with_conn(|c| -> rusqlite::Result<()> {
                c.execute(
                    "INSERT INTO backups (id, change_set_id, manifest_path, created_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(id) DO UPDATE SET
                        manifest_path = excluded.manifest_path,
                        created_at = excluded.created_at",
                    rusqlite::params![&id, change_set_id, &manifest_path_str, &now],
                )?;
                Ok(())
            })
            .map_err(DbError::from)?;

        Ok(Backup {
            id,
            change_set_id: change_set_id.to_string(),
            manifest_path: manifest_path_str,
            created_at: now,
        })
    }

    /// Load a backup manifest from disk.
    pub fn load_manifest(&self, backup_id: &str) -> BackupResult<BackupManifest> {
        let manifest_path = self.backups_dir.join(backup_id).join("manifest.json");
        let contents = fs::read_to_string(&manifest_path)?;
        let manifest = serde_json::from_str(&contents)?;
        Ok(manifest)
    }

    /// List persisted backups ordered by newest first.
    pub fn list(&self) -> BackupResult<Vec<Backup>> {
        let rows = self
            .db
            .with_conn(|c| -> rusqlite::Result<Vec<Backup>> {
                let mut stmt = c.prepare(
                    "SELECT id, change_set_id, manifest_path, created_at
                     FROM backups ORDER BY created_at DESC",
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(Backup {
                        id: r.get(0)?,
                        change_set_id: r.get(1)?,
                        manifest_path: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                })?;
                let mut out = Vec::new();
                for b in iter {
                    out.push(b?);
                }
                Ok(out)
            })
            .map_err(DbError::from)?;
        Ok(rows)
    }

    /// Verify that every file entry in the manifest still matches its recorded hash.
    pub fn verify(&self, backup_id: &str) -> BackupResult<Vec<(String, bool)>> {
        let manifest = self.load_manifest(backup_id)?;
        let mut results = Vec::new();
        for entry in &manifest.files {
            if entry.backup_path.is_empty() {
                // File did not exist at backup time; nothing to verify.
                results.push((entry.original_path.clone(), true));
                continue;
            }
            let path = Path::new(&entry.backup_path);
            let ok = if path.exists() {
                let current_hash = file_sha256(path)?;
                current_hash == entry.hash
            } else {
                false
            };
            results.push((entry.original_path.clone(), ok));
        }
        Ok(results)
    }

    /// Restore a single file from a backup.
    ///
    /// If the backup entry indicates the file did not exist at backup time,
    /// the original path is removed when present.
    pub fn restore_file(&self, backup_id: &str, file_index: usize) -> BackupResult<()> {
        let manifest = self.load_manifest(backup_id)?;
        let entry = manifest
            .files
            .get(file_index)
            .ok_or_else(|| BackupError::NotFound(format!("file index {file_index}")))?;

        let original = Path::new(&entry.original_path);
        if entry.backup_path.is_empty() {
            // File did not exist at backup time; remove if present.
            if original.exists() {
                fs::remove_file(original)?;
            }
            return Ok(());
        }

        let backup = Path::new(&entry.backup_path);
        if let Some(parent) = original.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(backup, original)?;
        Ok(())
    }

    /// Restore all files belonging to a backup and optionally verify hashes.
    ///
    /// Returns the list of restored original paths.
    pub fn restore_change_set(&self, backup_id: &str) -> BackupResult<Vec<String>> {
        let manifest = self.load_manifest(backup_id)?;
        let mut restored = Vec::new();
        for (idx, _entry) in manifest.files.iter().enumerate() {
            self.restore_file(backup_id, idx)?;
            restored.push(_entry.original_path.clone());
        }
        Ok(restored)
    }
}

pub(crate) fn file_sha256(path: &Path) -> BackupResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn svc() -> (tempfile::TempDir, Arc<Database>, BackupService) {
        let dir = tempdir().unwrap();
        let db = Arc::new(Database::open_in_memory().unwrap());
        let app_data = AppDataService::initialize(dir.path()).unwrap();
        let svc = BackupService::new(db.clone(), &app_data);
        (dir, db, svc)
    }

    fn seed_change_set(db: &Database, id: &str) {
        db.with_conn(|c| {
            c.execute(
                "INSERT INTO change_sets (id, status, operations_json, patches_json, diff_summary_json, created_at, updated_at)
                 VALUES (?1, 'draft', '[]', '[]', '{\"filesChanged\":0,\"additions\":0,\"deletions\":0}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![id],
            ).unwrap();
        });
    }

    #[test]
    fn backup_creates_manifest_and_copies_files() {
        let (dir, db, svc) = svc();
        seed_change_set(&db, "cs1");
        let file_a = dir.path().join("a.txt");
        fs::write(&file_a, "hello").unwrap();
        let file_b = dir.path().join("b.txt");
        fs::write(&file_b, "world").unwrap();

        let backup = svc
            .create("cs1", &[file_a.clone(), file_b.clone()])
            .unwrap();

        let manifest = svc.load_manifest(&backup.id).unwrap();
        assert_eq!(manifest.change_set_id, "cs1");
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[0].size, 5);
        assert!(!manifest.files[0].hash.is_empty());

        // Verify backup files exist on disk.
        let backup_a = Path::new(&manifest.files[0].backup_path);
        assert!(backup_a.exists());
        assert_eq!(fs::read_to_string(backup_a).unwrap(), "hello");
    }

    #[test]
    fn missing_target_file_gets_empty_entry() {
        let (_dir, db, svc) = svc();
        seed_change_set(&db, "cs2");
        let missing = PathBuf::from("/this/path/does/not/exist/xyz.txt");
        let backup = svc.create("cs2", &[missing.clone()]).unwrap();
        let manifest = svc.load_manifest(&backup.id).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].size, 0);
        assert!(manifest.files[0].hash.is_empty());
        assert!(manifest.files[0].backup_path.is_empty());
    }

    #[test]
    fn verify_matches_recorded_hash() {
        let (dir, db, svc) = svc();
        seed_change_set(&db, "cs3");
        let file_a = dir.path().join("a.txt");
        fs::write(&file_a, "hello").unwrap();

        let backup = svc.create("cs3", &[file_a]).unwrap();
        let results = svc.verify(&backup.id).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1);
    }

    #[test]
    fn list_returns_created_backups() {
        let (dir, db, svc) = svc();
        seed_change_set(&db, "cs4");
        let file_a = dir.path().join("a.txt");
        fs::write(&file_a, "hello").unwrap();
        svc.create("cs4", &[file_a]).unwrap();
        let list = svc.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].change_set_id, "cs4");
    }

    #[test]
    fn restore_reverts_file_to_backup_state() {
        let (dir, db, svc) = svc();
        seed_change_set(&db, "cs5");
        let file_a = dir.path().join("a.txt");
        fs::write(&file_a, "hello").unwrap();

        let backup = svc.create("cs5", &[file_a.clone()]).unwrap();
        // Modify original after backup.
        fs::write(&file_a, "modified").unwrap();
        assert_eq!(fs::read_to_string(&file_a).unwrap(), "modified");

        svc.restore_change_set(&backup.id).unwrap();
        assert_eq!(fs::read_to_string(&file_a).unwrap(), "hello");
    }

    #[test]
    fn restore_removes_file_that_did_not_exist_at_backup() {
        let (dir, db, svc) = svc();
        seed_change_set(&db, "cs6");
        let missing = dir.path().join("new.txt");
        // Ensure the file does NOT exist at backup time.
        if missing.exists() {
            fs::remove_file(&missing).unwrap();
        }

        let backup = svc.create("cs6", &[missing.clone()]).unwrap();
        // After backup, someone creates the file.
        fs::write(&missing, "I should be removed").unwrap();
        assert!(missing.exists());

        // Restore should remove it because it did not exist at backup time.
        svc.restore_change_set(&backup.id).unwrap();
        assert!(!missing.exists());
    }

    #[test]
    fn restore_file_hash_matches_backup() {
        let (dir, db, svc) = svc();
        seed_change_set(&db, "cs7");
        let file_a = dir.path().join("a.txt");
        fs::write(&file_a, "hello").unwrap();

        let backup = svc.create("cs7", &[file_a.clone()]).unwrap();
        fs::write(&file_a, "modified").unwrap();

        svc.restore_change_set(&backup.id).unwrap();
        let restored_hash = file_sha256(&file_a).unwrap();
        let manifest = svc.load_manifest(&backup.id).unwrap();
        assert_eq!(restored_hash, manifest.files[0].hash);
    }
}
