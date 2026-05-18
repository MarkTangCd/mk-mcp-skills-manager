// LibraryService: manages AgentHub's internal library directories (skills,
// sub-agents, prompts, mcp-templates) under the app-data root.
//
// Each library entry lives in its own directory named by its slug and contains
// a `metadata.json` file together with optional content files.  All writes use
// temp-file + atomic rename and are validated by PathGuard.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::AgentKind;
use crate::security::{PathGuard, PathGuardError};
use crate::services::AppDataLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryKind {
  Skills,
  SubAgents,
  Prompts,
  McpTemplates,
}

impl LibraryKind {
  pub fn from_str(s: &str) -> Option<Self> {
    match s {
      "skills" => Some(LibraryKind::Skills),
      "sub-agents" => Some(LibraryKind::SubAgents),
      "prompts" => Some(LibraryKind::Prompts),
      "mcp-templates" => Some(LibraryKind::McpTemplates),
      _ => None,
    }
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      LibraryKind::Skills => "skills",
      LibraryKind::SubAgents => "sub-agents",
      LibraryKind::Prompts => "prompts",
      LibraryKind::McpTemplates => "mcp-templates",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMetadata {
  pub slug: String,
  pub title: String,
  pub description: Option<String>,
  pub tags: Vec<String>,
  #[serde(default)]
  pub entry_file: Option<String>,
  // Sub-agent specific fields
  #[serde(default)]
  pub role: Option<String>,
  #[serde(default)]
  pub agent_kinds: Vec<AgentKind>,
  #[serde(default)]
  pub bound_mcp_ids: Vec<String>,
  #[serde(default)]
  pub bound_skill_ids: Vec<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
  pub kind: LibraryKind,
  pub slug: String,
  pub metadata: LibraryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntryDetail {
  pub kind: LibraryKind,
  pub slug: String,
  pub metadata: LibraryMetadata,
  pub files: HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum LibraryError {
  #[error("invalid library kind: {0}")]
  InvalidKind(String),
  #[error("invalid slug: {0}")]
  InvalidSlug(String),
  #[error("slug already exists: {0}")]
  DuplicateSlug(String),
  #[error("entry not found: {0}/{1}")]
  NotFound(String, String),
  #[error("path not allowed: {0}")]
  PathNotAllowed(String),
  #[error("io error: {0}")]
  Io(#[from] std::io::Error),
  #[error("json error: {0}")]
  Json(#[from] serde_json::Error),
  #[error("path guard error: {0}")]
  PathGuard(#[from] PathGuardError),
}

pub type LibraryResult<T> = Result<T, LibraryError>;

#[derive(Debug, Clone)]
pub struct LibraryService {
  layout: AppDataLayout,
  guard: PathGuard,
}

impl LibraryService {
  pub fn new(layout: AppDataLayout, guard: PathGuard) -> Self {
    Self { layout, guard }
  }

  pub fn list(&self, kind: LibraryKind) -> LibraryResult<Vec<LibraryEntry>> {
    let dir = self.kind_dir(kind);
    let mut entries = Vec::new();

    if !dir.exists() {
      return Ok(entries);
    }

    for entry in std::fs::read_dir(&dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.is_dir() {
        let slug = path
          .file_name()
          .and_then(|n| n.to_str())
          .unwrap_or_default()
          .to_string();

        let metadata_path = path.join("metadata.json");
        if metadata_path.exists() {
          let metadata = self.read_metadata(&metadata_path)?;
          entries.push(LibraryEntry {
            kind,
            slug,
            metadata,
          });
        }
      }
    }

    // Sort by title for consistent ordering.
    entries.sort_by(|a, b| a.metadata.title.cmp(&b.metadata.title));
    Ok(entries)
  }

  pub fn create(
    &self,
    kind: LibraryKind,
    slug: &str,
    metadata: LibraryMetadata,
  ) -> LibraryResult<LibraryEntry> {
    if !is_valid_slug(slug) {
      return Err(LibraryError::InvalidSlug(slug.to_string()));
    }

    let entry_dir = self.entry_dir(kind, slug);

    if entry_dir.exists() {
      return Err(LibraryError::DuplicateSlug(slug.to_string()));
    }

    self.guard.ensure_writable(&entry_dir)?;
    std::fs::create_dir_all(&entry_dir)?;

    let metadata_path = entry_dir.join("metadata.json");
    self.write_json_atomic(&metadata_path, &metadata)?;

    Ok(LibraryEntry {
      kind,
      slug: slug.to_string(),
      metadata,
    })
  }

  pub fn get(&self, kind: LibraryKind, slug: &str) -> LibraryResult<LibraryEntryDetail> {
    let entry_dir = self.entry_dir(kind, slug);

    if !entry_dir.exists() {
      return Err(LibraryError::NotFound(
        kind.as_str().to_string(),
        slug.to_string(),
      ));
    }

    let metadata_path = entry_dir.join("metadata.json");
    let metadata = if metadata_path.exists() {
      self.read_metadata(&metadata_path)?
    } else {
      LibraryMetadata {
        slug: slug.to_string(),
        title: slug.to_string(),
        description: None,
        tags: vec![],
        entry_file: None,
        role: None,
        agent_kinds: vec![],
        bound_mcp_ids: vec![],
        bound_skill_ids: vec![],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
      }
    };

    let mut files = HashMap::new();
    for entry in std::fs::read_dir(&entry_dir)? {
      let entry = entry?;
      let file_name = entry.file_name();
      let file_name_str = file_name.to_string_lossy();
      if file_name_str != "metadata.json" && entry.path().is_file() {
        if let Ok(content) = std::fs::read_to_string(&entry.path()) {
          files.insert(file_name_str.to_string(), content);
        }
      }
    }

    Ok(LibraryEntryDetail {
      kind,
      slug: slug.to_string(),
      metadata,
      files,
    })
  }

  pub fn update(
    &self,
    kind: LibraryKind,
    slug: &str,
    metadata: LibraryMetadata,
  ) -> LibraryResult<LibraryEntry> {
    let entry_dir = self.entry_dir(kind, slug);

    if !entry_dir.exists() {
      return Err(LibraryError::NotFound(
        kind.as_str().to_string(),
        slug.to_string(),
      ));
    }

    let metadata_path = entry_dir.join("metadata.json");
    self.guard.ensure_writable(&metadata_path)?;

    let mut metadata = metadata;
    metadata.slug = slug.to_string();
    metadata.updated_at = Utc::now().to_rfc3339();

    self.write_json_atomic(&metadata_path, &metadata)?;

    Ok(LibraryEntry {
      kind,
      slug: slug.to_string(),
      metadata,
    })
  }

  pub fn delete(&self, kind: LibraryKind, slug: &str) -> LibraryResult<()> {
    let entry_dir = self.entry_dir(kind, slug);

    if !entry_dir.exists() {
      return Err(LibraryError::NotFound(
        kind.as_str().to_string(),
        slug.to_string(),
      ));
    }

    // Validate that we have write permission to the directory tree.
    self.guard
      .ensure_writable(&entry_dir.join(".guard-check"))?;

    std::fs::remove_dir_all(&entry_dir)?;
    Ok(())
  }

  // ------------------------------------------------------------------
  // Skill-specific helpers
  // ------------------------------------------------------------------

  pub fn skills_list(
    &self,
    search: Option<&str>,
    tags: Option<&[String]>,
  ) -> LibraryResult<Vec<LibraryEntry>> {
    let mut entries = self.list(LibraryKind::Skills)?;

    if let Some(query) = search {
      let q = query.to_lowercase();
      entries.retain(|entry| {
        entry.slug.to_lowercase().contains(&q)
          || entry.metadata.title.to_lowercase().contains(&q)
          || entry
            .metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&q)
          || entry
            .metadata
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&q))
      });
    }

    if let Some(required_tags) = tags {
      if !required_tags.is_empty() {
        entries.retain(|entry| {
          required_tags
            .iter()
            .all(|tag| entry.metadata.tags.contains(tag))
        });
      }
    }

    Ok(entries)
  }

  pub fn skills_create(
    &self,
    slug: &str,
    title: &str,
    description: Option<&str>,
    tags: Vec<String>,
    entry_file: Option<&str>,
  ) -> LibraryResult<LibraryEntry> {
    let now = Utc::now().to_rfc3339();
    let metadata = LibraryMetadata {
      slug: slug.to_string(),
      title: title.to_string(),
      description: description.map(|s| s.to_string()),
      tags,
      entry_file: entry_file.map(|s| s.to_string()),
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: now.clone(),
      updated_at: now,
    };

    let entry = self.create(LibraryKind::Skills, slug, metadata)?;

    if let Some(file_name) = entry_file {
      let file_path = self.entry_dir(LibraryKind::Skills, slug).join(file_name);
      self.guard.ensure_writable(&file_path)?;
      if !file_path.exists() {
        std::fs::write(&file_path, "")?;
      }
    }

    Ok(entry)
  }

  pub fn skills_import(
    &self,
    source_path: &str,
    slug: Option<&str>,
  ) -> LibraryResult<LibraryEntry> {
    let source = Path::new(source_path);
    if !source.exists() || !source.is_dir() {
      return Err(LibraryError::PathNotAllowed(source_path.to_string()));
    }

    let slug = match slug {
      Some(s) => s.to_string(),
      None => source
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
          LibraryError::InvalidSlug("derived slug is empty".to_string())
        })?,
    };

    if !is_valid_slug(&slug) {
      return Err(LibraryError::InvalidSlug(slug.clone()));
    }

    let entry_dir = self.entry_dir(LibraryKind::Skills, &slug);
    if entry_dir.exists() {
      return Err(LibraryError::DuplicateSlug(slug.clone()));
    }

    self.guard.ensure_writable(&entry_dir)?;
    copy_dir_recursive(source, &entry_dir)?;

    let metadata_path = entry_dir.join("metadata.json");
    let metadata = if metadata_path.exists() {
      self.read_metadata(&metadata_path)?
    } else {
      LibraryMetadata {
        slug: slug.clone(),
        title: slug.clone(),
        description: None,
        tags: vec![],
        entry_file: None,
        role: None,
        agent_kinds: vec![],
        bound_mcp_ids: vec![],
        bound_skill_ids: vec![],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
      }
    };

    self.write_json_atomic(&metadata_path, &metadata)?;

    Ok(LibraryEntry {
      kind: LibraryKind::Skills,
      slug,
      metadata,
    })
  }

  pub fn skills_get(&self, slug: &str) -> LibraryResult<LibraryEntryDetail> {
    self.get(LibraryKind::Skills, slug)
  }

  pub fn skills_update(
    &self,
    slug: &str,
    metadata: LibraryMetadata,
  ) -> LibraryResult<LibraryEntry> {
    self.update(LibraryKind::Skills, slug, metadata)
  }

  pub fn skills_delete(&self, slug: &str) -> LibraryResult<()> {
    self.delete(LibraryKind::Skills, slug)
  }

  // ------------------------------------------------------------------
  // Sub-agent helpers
  // ------------------------------------------------------------------

  pub fn sub_agents_list(
    &self,
    search: Option<&str>,
    tags: Option<&[String]>,
  ) -> LibraryResult<Vec<LibraryEntry>> {
    let mut entries = self.list(LibraryKind::SubAgents)?;

    if let Some(query) = search {
      let q = query.to_lowercase();
      entries.retain(|entry| {
        entry.slug.to_lowercase().contains(&q)
          || entry.metadata.title.to_lowercase().contains(&q)
          || entry
            .metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&q)
          || entry
            .metadata
            .role
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&q)
          || entry
            .metadata
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&q))
      });
    }

    if let Some(required_tags) = tags {
      if !required_tags.is_empty() {
        entries.retain(|entry| {
          required_tags
            .iter()
            .all(|tag| entry.metadata.tags.contains(tag))
        });
      }
    }

    Ok(entries)
  }

  pub fn sub_agents_create(
    &self,
    slug: &str,
    metadata: LibraryMetadata,
  ) -> LibraryResult<LibraryEntry> {
    self.create(LibraryKind::SubAgents, slug, metadata)
  }

  pub fn sub_agents_get(&self, slug: &str) -> LibraryResult<LibraryEntryDetail> {
    self.get(LibraryKind::SubAgents, slug)
  }

  pub fn sub_agents_update(
    &self,
    slug: &str,
    metadata: LibraryMetadata,
  ) -> LibraryResult<LibraryEntry> {
    self.update(LibraryKind::SubAgents, slug, metadata)
  }

  pub fn sub_agents_delete(&self, slug: &str) -> LibraryResult<()> {
    self.delete(LibraryKind::SubAgents, slug)
  }

  pub fn sub_agent_templates(&self) -> Vec<LibraryEntry> {
    let now = Utc::now().to_rfc3339();
    vec![
      LibraryEntry {
        kind: LibraryKind::SubAgents,
        slug: "code-reviewer".to_string(),
        metadata: LibraryMetadata {
          slug: "code-reviewer".to_string(),
          title: "Code Reviewer".to_string(),
          description: Some("A dedicated sub-agent for code review tasks".to_string()),
          tags: vec!["review".to_string(), "pr".to_string()],
          entry_file: None,
          role: Some("Review PRs for style, bugs, and security issues".to_string()),
          agent_kinds: vec![AgentKind::ClaudeCode, AgentKind::Codex],
          bound_mcp_ids: vec![],
          bound_skill_ids: vec![],
          created_at: now.clone(),
          updated_at: now.clone(),
        },
      },
      LibraryEntry {
        kind: LibraryKind::SubAgents,
        slug: "debugger".to_string(),
        metadata: LibraryMetadata {
          slug: "debugger".to_string(),
          title: "Debugger".to_string(),
          description: Some("A dedicated sub-agent for debugging tasks".to_string()),
          tags: vec!["debug".to_string(), "trace".to_string()],
          entry_file: None,
          role: Some("Debug runtime errors and trace execution".to_string()),
          agent_kinds: vec![AgentKind::ClaudeCode, AgentKind::Codex],
          bound_mcp_ids: vec![],
          bound_skill_ids: vec![],
          created_at: now.clone(),
          updated_at: now.clone(),
        },
      },
      LibraryEntry {
        kind: LibraryKind::SubAgents,
        slug: "test-writer".to_string(),
        metadata: LibraryMetadata {
          slug: "test-writer".to_string(),
          title: "Test Writer".to_string(),
          description: Some("A dedicated sub-agent for test generation".to_string()),
          tags: vec!["test".to_string(), "quality".to_string()],
          entry_file: None,
          role: Some("Generate unit and integration tests".to_string()),
          agent_kinds: vec![AgentKind::ClaudeCode, AgentKind::Codex],
          bound_mcp_ids: vec![],
          bound_skill_ids: vec![],
          created_at: now.clone(),
          updated_at: now.clone(),
        },
      },
      LibraryEntry {
        kind: LibraryKind::SubAgents,
        slug: "docs-writer".to_string(),
        metadata: LibraryMetadata {
          slug: "docs-writer".to_string(),
          title: "Docs Writer".to_string(),
          description: Some("A dedicated sub-agent for documentation tasks".to_string()),
          tags: vec!["docs".to_string(), "write".to_string()],
          entry_file: None,
          role: Some("Write and maintain documentation".to_string()),
          agent_kinds: vec![AgentKind::ClaudeCode, AgentKind::Codex],
          bound_mcp_ids: vec![],
          bound_skill_ids: vec![],
          created_at: now.clone(),
          updated_at: now.clone(),
        },
      },
    ]
  }

  // ------------------------------------------------------------------
  // Helpers
  // ------------------------------------------------------------------

  fn kind_dir(&self, kind: LibraryKind) -> PathBuf {
    match kind {
      LibraryKind::Skills => self.layout.library_skills.clone(),
      LibraryKind::SubAgents => self.layout.library_sub_agents.clone(),
      LibraryKind::Prompts => self.layout.library_prompts.clone(),
      LibraryKind::McpTemplates => self.layout.library_mcp_templates.clone(),
    }
  }

  pub fn entry_dir(&self, kind: LibraryKind, slug: &str) -> PathBuf {
    self.kind_dir(kind).join(slug)
  }

  fn read_metadata(&self, path: &Path) -> LibraryResult<LibraryMetadata> {
    let content = std::fs::read_to_string(path)?;
    let metadata: LibraryMetadata = serde_json::from_str(&content)?;
    Ok(metadata)
  }

  fn write_json_atomic(
    &self,
    path: &Path,
    value: &impl Serialize,
  ) -> LibraryResult<()> {
    self.guard.ensure_writable(path)?;

    let parent = path.parent().ok_or_else(|| {
      LibraryError::PathNotAllowed(path.to_string_lossy().to_string())
    })?;
    std::fs::create_dir_all(parent)?;

    let temp_path = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(&temp_path, json)?;
    std::fs::rename(&temp_path, path)?;

    Ok(())
  }
}

fn copy_dir_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> LibraryResult<()> {
  std::fs::create_dir_all(&dst)?;
  for entry in std::fs::read_dir(src)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    let src_path = entry.path();
    let dst_path = dst.as_ref().join(entry.file_name());
    if file_type.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      std::fs::copy(&src_path, &dst_path)?;
    }
  }
  Ok(())
}

/// Validates a library entry slug.
/// Only lowercase kebab-case is allowed: `^[a-z0-9]+(-[a-z0-9]+)*$`.
fn is_valid_slug(slug: &str) -> bool {
  if slug.is_empty() {
    return false;
  }

  let parts: Vec<&str> = slug.split('-').collect();

  for part in &parts {
    if part.is_empty() {
      return false;
    }
    for ch in part.chars() {
      if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() {
        return false;
      }
    }
  }

  true
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::AppDataService;

  fn svc() -> (tempfile::TempDir, LibraryService) {
    let dir = tempfile::tempdir().unwrap();
    let app_data = AppDataService::initialize(dir.path()).unwrap();
    let svc = LibraryService::new(app_data.layout().clone(), app_data.guard().clone());
    (dir, svc)
  }

  #[test]
  fn slug_validation() {
    assert!(is_valid_slug("hello"));
    assert!(is_valid_slug("hello-world"));
    assert!(is_valid_slug("hello-world-123"));
    assert!(!is_valid_slug("Hello"));
    assert!(!is_valid_slug("hello_world"));
    assert!(!is_valid_slug("hello world"));
    assert!(!is_valid_slug("-hello"));
    assert!(!is_valid_slug("hello-"));
    assert!(!is_valid_slug(""));
    assert!(!is_valid_slug("hello--world"));
  }

  #[test]
  fn create_and_get_entry() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "test-skill".to_string(),
      title: "Test Skill".to_string(),
      description: Some("A test skill".to_string()),
      tags: vec!["test".to_string()],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    let entry = svc
      .create(LibraryKind::Skills, "test-skill", metadata.clone())
      .unwrap();
    assert_eq!(entry.slug, "test-skill");
    assert_eq!(entry.metadata.title, "Test Skill");

    let detail = svc.get(LibraryKind::Skills, "test-skill").unwrap();
    assert_eq!(detail.metadata.title, "Test Skill");
    assert!(detail.files.is_empty());
  }

  #[test]
  fn list_entries() {
    let (_dir, svc) = svc();
    let metadata1 = LibraryMetadata {
      slug: "skill-a".to_string(),
      title: "Skill A".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };
    let metadata2 = LibraryMetadata {
      slug: "skill-b".to_string(),
      title: "Skill B".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "skill-a", metadata1).unwrap();
    svc.create(LibraryKind::Skills, "skill-b", metadata2).unwrap();

    let list = svc.list(LibraryKind::Skills).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].slug, "skill-a"); // sorted by title
    assert_eq!(list[1].slug, "skill-b");
  }

  #[test]
  fn duplicate_slug_rejected() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "dup".to_string(),
      title: "Dup".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "dup", metadata.clone())
      .unwrap();
    let err = svc
      .create(LibraryKind::Skills, "dup", metadata)
      .unwrap_err();
    assert!(matches!(err, LibraryError::DuplicateSlug(_)));
  }

  #[test]
  fn update_entry() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "update-test".to_string(),
      title: "Original".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "update-test", metadata)
      .unwrap();

    let updated = LibraryMetadata {
      slug: "update-test".to_string(),
      title: "Updated".to_string(),
      description: Some("New desc".to_string()),
      tags: vec!["updated".to_string()],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    let entry = svc
      .update(LibraryKind::Skills, "update-test", updated)
      .unwrap();
    assert_eq!(entry.metadata.title, "Updated");

    let detail = svc.get(LibraryKind::Skills, "update-test").unwrap();
    assert_eq!(
      detail.metadata.description,
      Some("New desc".to_string())
    );
  }

  #[test]
  fn delete_entry() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "del".to_string(),
      title: "Del".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "del", metadata).unwrap();
    assert!(svc.get(LibraryKind::Skills, "del").is_ok());

    svc.delete(LibraryKind::Skills, "del").unwrap();
    assert!(svc.get(LibraryKind::Skills, "del").is_err());
  }

  #[test]
  fn invalid_slug_rejected() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "bad".to_string(),
      title: "Bad".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    let err = svc
      .create(LibraryKind::Skills, "Bad_Slug", metadata)
      .unwrap_err();
    assert!(matches!(err, LibraryError::InvalidSlug(_)));
  }

  #[test]
  fn different_kinds_isolated() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "shared".to_string(),
      title: "Shared".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "shared", metadata.clone())
      .unwrap();
    svc.create(LibraryKind::Prompts, "shared", metadata)
      .unwrap();

    let skills = svc.list(LibraryKind::Skills).unwrap();
    let prompts = svc.list(LibraryKind::Prompts).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(prompts.len(), 1);
  }

  #[test]
  fn get_reads_content_files() {
    let (dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "with-files".to_string(),
      title: "With Files".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "with-files", metadata)
      .unwrap();

    // Write an extra text file directly.
    let entry_dir = dir.path().join("library/skills/with-files");
    std::fs::write(entry_dir.join("body.md"), "# Hello").unwrap();

    let detail = svc.get(LibraryKind::Skills, "with-files").unwrap();
    assert_eq!(detail.files.get("body.md").unwrap(), "# Hello");
  }

  #[test]
  fn write_uses_atomic_rename() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "atomic".to_string(),
      title: "Atomic".to_string(),
      description: None,
      tags: vec![],
      entry_file: None,
      role: None,
      agent_kinds: vec![],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.create(LibraryKind::Skills, "atomic", metadata)
      .unwrap();

    // There should be no .tmp file left behind.
    let entry_dir = svc.entry_dir(LibraryKind::Skills, "atomic");
    for entry in std::fs::read_dir(&entry_dir).unwrap() {
      let entry = entry.unwrap();
      assert!(
        !entry.path().extension().map_or(false, |e| e == "tmp"),
        "temp file should have been renamed"
      );
    }
  }

  #[test]
  fn skills_list_filters_by_search() {
    let (_dir, svc) = svc();
    svc.skills_create("alpha-skill", "Alpha", Some("first skill"), vec!["tag-a".to_string()], Some("a.md")).unwrap();
    svc.skills_create("beta-skill", "Beta", Some("second skill"), vec!["tag-b".to_string()], None).unwrap();

    let all = svc.skills_list(None, None).unwrap();
    assert_eq!(all.len(), 2);

    let by_slug = svc.skills_list(Some("alpha"), None).unwrap();
    assert_eq!(by_slug.len(), 1);
    assert_eq!(by_slug[0].slug, "alpha-skill");

    let by_desc = svc.skills_list(Some("second"), None).unwrap();
    assert_eq!(by_desc.len(), 1);
    assert_eq!(by_desc[0].slug, "beta-skill");

    let by_tag = svc.skills_list(None, Some(&["tag-a".to_string()])).unwrap();
    assert_eq!(by_tag.len(), 1);
    assert_eq!(by_tag[0].slug, "alpha-skill");
  }

  #[test]
  fn skills_create_with_entry_file() {
    let (dir, svc) = svc();
    let entry = svc.skills_create("with-entry", "With Entry", None, vec![], Some("skill.md")).unwrap();
    assert_eq!(entry.metadata.entry_file, Some("skill.md".to_string()));

    let entry_file_path = dir.path().join("library/skills/with-entry/skill.md");
    assert!(entry_file_path.exists(), "entry file placeholder should exist");
    assert_eq!(std::fs::read_to_string(&entry_file_path).unwrap(), "");
  }

  #[test]
  fn skills_import_copies_directory() {
    let (dir, svc) = svc();
    let source = dir.path().join("external/some-skill");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("readme.md"), "# Hello").unwrap();

    let entry = svc.skills_import(source.to_str().unwrap(), None).unwrap();
    assert_eq!(entry.slug, "some-skill");

    let imported_readme = dir.path().join("library/skills/some-skill/readme.md");
    assert!(imported_readme.exists());
    assert_eq!(std::fs::read_to_string(&imported_readme).unwrap(), "# Hello");
  }

  #[test]
  fn skills_import_rejects_invalid_slug() {
    let (dir, svc) = svc();
    let source = dir.path().join("external/BadSlug");
    std::fs::create_dir_all(&source).unwrap();

    let err = svc.skills_import(source.to_str().unwrap(), None).unwrap_err();
    assert!(matches!(err, LibraryError::InvalidSlug(_)));
  }

  #[test]
  fn sub_agents_list_filters_by_search() {
    let (_dir, svc) = svc();
    svc.sub_agents_create("alpha-agent", LibraryMetadata {
      slug: "alpha-agent".to_string(),
      title: "Alpha".to_string(),
      description: Some("first agent".to_string()),
      tags: vec!["tag-a".to_string()],
      entry_file: None,
      role: Some("Alpha role".to_string()),
      agent_kinds: vec![AgentKind::ClaudeCode],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    }).unwrap();
    svc.sub_agents_create("beta-agent", LibraryMetadata {
      slug: "beta-agent".to_string(),
      title: "Beta".to_string(),
      description: Some("second agent".to_string()),
      tags: vec!["tag-b".to_string()],
      entry_file: None,
      role: Some("Beta role".to_string()),
      agent_kinds: vec![AgentKind::Codex],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    }).unwrap();

    let all = svc.sub_agents_list(None, None).unwrap();
    assert_eq!(all.len(), 2);

    let by_slug = svc.sub_agents_list(Some("alpha"), None).unwrap();
    assert_eq!(by_slug.len(), 1);
    assert_eq!(by_slug[0].slug, "alpha-agent");

    let by_role = svc.sub_agents_list(Some("Beta role"), None).unwrap();
    assert_eq!(by_role.len(), 1);
    assert_eq!(by_role[0].slug, "beta-agent");

    let by_tag = svc.sub_agents_list(None, Some(&["tag-a".to_string()])).unwrap();
    assert_eq!(by_tag.len(), 1);
    assert_eq!(by_tag[0].slug, "alpha-agent");
  }

  #[test]
  fn sub_agent_templates_returned() {
    let (_dir, svc) = svc();
    let templates = svc.sub_agent_templates();
    assert_eq!(templates.len(), 4);
    let slugs: Vec<String> = templates.iter().map(|t| t.slug.clone()).collect();
    assert!(slugs.contains(&"code-reviewer".to_string()));
    assert!(slugs.contains(&"debugger".to_string()));
    assert!(slugs.contains(&"test-writer".to_string()));
    assert!(slugs.contains(&"docs-writer".to_string()));
  }

  #[test]
  fn sub_agents_crud() {
    let (_dir, svc) = svc();
    let metadata = LibraryMetadata {
      slug: "my-agent".to_string(),
      title: "My Agent".to_string(),
      description: Some("A test agent".to_string()),
      tags: vec!["test".to_string()],
      entry_file: None,
      role: Some("Test role".to_string()),
      agent_kinds: vec![AgentKind::ClaudeCode],
      bound_mcp_ids: vec!["mcp:1".to_string()],
      bound_skill_ids: vec!["skill:1".to_string()],
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    };

    svc.sub_agents_create("my-agent", metadata.clone()).unwrap();

    let detail = svc.sub_agents_get("my-agent").unwrap();
    assert_eq!(detail.metadata.title, "My Agent");
    assert_eq!(detail.metadata.role, Some("Test role".to_string()));
    assert_eq!(detail.metadata.agent_kinds, vec![AgentKind::ClaudeCode]);
    assert_eq!(detail.metadata.bound_mcp_ids, vec!["mcp:1".to_string()]);
    assert_eq!(detail.metadata.bound_skill_ids, vec!["skill:1".to_string()]);

    let updated = LibraryMetadata {
      slug: "my-agent".to_string(),
      title: "My Agent Updated".to_string(),
      description: Some("Updated desc".to_string()),
      tags: vec!["test".to_string(), "updated".to_string()],
      entry_file: None,
      role: Some("Updated role".to_string()),
      agent_kinds: vec![AgentKind::ClaudeCode, AgentKind::Codex],
      bound_mcp_ids: vec![],
      bound_skill_ids: vec![],
      created_at: metadata.created_at.clone(),
      updated_at: Utc::now().to_rfc3339(),
    };
    svc.sub_agents_update("my-agent", updated).unwrap();

    let detail2 = svc.sub_agents_get("my-agent").unwrap();
    assert_eq!(detail2.metadata.title, "My Agent Updated");
    assert_eq!(detail2.metadata.agent_kinds.len(), 2);

    svc.sub_agents_delete("my-agent").unwrap();
    assert!(svc.sub_agents_get("my-agent").is_err());
  }
}
