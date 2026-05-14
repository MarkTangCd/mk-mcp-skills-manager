use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::security::PathGuard;

#[derive(Debug, Error)]
pub enum AppDataError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Filesystem layout under the app-data root.
#[derive(Debug, Clone)]
pub struct AppDataLayout {
    pub root: PathBuf,
    pub library_skills: PathBuf,
    pub library_sub_agents: PathBuf,
    pub library_prompts: PathBuf,
    pub library_mcp_templates: PathBuf,
    pub backups: PathBuf,
    pub logs: PathBuf,
    pub cache_scans: PathBuf,
    pub database_path: PathBuf,
}

impl AppDataLayout {
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            library_skills: root.join("library").join("skills"),
            library_sub_agents: root.join("library").join("sub-agents"),
            library_prompts: root.join("library").join("prompts"),
            library_mcp_templates: root.join("library").join("mcp-templates"),
            backups: root.join("backups"),
            logs: root.join("logs"),
            cache_scans: root.join("cache").join("scans"),
            database_path: root.join("agenthub.sqlite3"),
            root,
        }
    }

    pub fn all_dirs(&self) -> [&Path; 8] {
        [
            self.root.as_path(),
            self.library_skills.as_path(),
            self.library_sub_agents.as_path(),
            self.library_prompts.as_path(),
            self.library_mcp_templates.as_path(),
            self.backups.as_path(),
            self.logs.as_path(),
            self.cache_scans.as_path(),
        ]
    }
}

/// Owns and provisions the on-disk layout under the OS app-data directory.
#[derive(Debug, Clone)]
pub struct AppDataService {
    layout: AppDataLayout,
    guard: PathGuard,
}

impl AppDataService {
    /// Create the layout on disk if missing, then build a PathGuard scoped to
    /// the app-data root.
    pub fn initialize(root: impl AsRef<Path>) -> Result<Self, AppDataError> {
        let layout = AppDataLayout::from_root(root);
        for dir in layout.all_dirs() {
            std::fs::create_dir_all(dir)?;
        }
        let guard = PathGuard::new(vec![layout.root.clone()]);
        Ok(Self { layout, guard })
    }

    pub fn layout(&self) -> &AppDataLayout {
        &self.layout
    }

    pub fn guard(&self) -> &PathGuard {
        &self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_creates_expected_directories() {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppDataService::initialize(dir.path()).unwrap();
        for path in svc.layout().all_dirs() {
            assert!(path.is_dir(), "expected directory at {path:?}");
        }
        // Database file path is *not* created here; only the parent directory.
        assert!(svc.layout().database_path.parent().unwrap().is_dir());
    }

    #[test]
    fn guard_rejects_paths_outside_root() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let svc = AppDataService::initialize(dir.path()).unwrap();
        let escape = other.path().join("hack.txt");
        assert!(svc.guard().ensure_writable(&escape).is_err());
    }

    #[test]
    fn guard_accepts_paths_inside_root() {
        let dir = tempfile::tempdir().unwrap();
        let svc = AppDataService::initialize(dir.path()).unwrap();
        let target = svc.layout().library_skills.join("hello.md");
        assert!(svc.guard().ensure_writable(&target).is_ok());
    }
}
