use serde_json::Value as JsonValue;
use tauri::State;

use crate::domain::AgentKind;
use crate::error::{CommandError, CommandResult};
use crate::services::{LibraryEntry, LibraryEntryDetail, LibraryKind, LibraryMetadata};
use crate::state::AppState;

#[tauri::command]
pub fn library_list(kind: String, state: State<'_, AppState>) -> CommandResult<Vec<LibraryEntry>> {
    let library_kind = parse_kind(&kind)?;
    state.library.list(library_kind).map_err(Into::into)
}

#[tauri::command]
pub fn library_create(
    kind: String,
    slug: String,
    metadata: JsonValue,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let library_kind = parse_kind(&kind)?;
    let metadata = build_metadata(&slug, metadata)?;
    state.library.create(library_kind, &slug, metadata).map_err(Into::into)
}

#[tauri::command]
pub fn library_get(
    kind: String,
    slug: String,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntryDetail> {
    let library_kind = parse_kind(&kind)?;
    state.library.get(library_kind, &slug).map_err(Into::into)
}

#[tauri::command]
pub fn library_update(
    kind: String,
    slug: String,
    metadata: JsonValue,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let library_kind = parse_kind(&kind)?;

    // Read existing metadata so we can preserve created_at and merge fields.
    let existing = state.library.get(library_kind, &slug).map_err(Into::<CommandError>::into)?;
    let metadata = merge_metadata(existing.metadata, metadata)?;

    state.library.update(library_kind, &slug, metadata).map_err(Into::into)
}

#[tauri::command]
pub fn library_delete(kind: String, slug: String, state: State<'_, AppState>) -> CommandResult<()> {
    let library_kind = parse_kind(&kind)?;
    state.library.delete(library_kind, &slug).map_err(Into::into)
}

// ------------------------------------------------------------------
// Skill-specific commands
// ------------------------------------------------------------------

#[tauri::command]
pub fn skills_list(
    search: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> CommandResult<Vec<LibraryEntry>> {
    state.library.skills_list(search.as_deref(), tags.as_deref()).map_err(Into::into)
}

#[tauri::command]
pub fn skills_create(
    slug: String,
    title: String,
    description: Option<String>,
    tags: Vec<String>,
    entry_file: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let entry = state
        .library
        .skills_create(&slug, &title, description.as_deref(), tags, entry_file.as_deref())
        .map_err(Into::<CommandError>::into)?;

    let source_path = entry_file.as_ref().map(|name| {
        state
            .library
            .entry_dir(LibraryKind::Skills, &slug)
            .join(name)
            .to_string_lossy()
            .to_string()
    });
    state
        .resources
        .upsert_library_skill(&slug, &title, description.as_deref(), source_path.as_deref())
        .map_err(Into::<CommandError>::into)?;

    Ok(entry)
}

#[tauri::command]
pub fn skills_import(
    source_path: String,
    slug: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let entry = state
        .library
        .skills_import(&source_path, slug.as_deref())
        .map_err(Into::<CommandError>::into)?;

    let source_path = entry.metadata.entry_file.as_ref().map(|name| {
        state
            .library
            .entry_dir(LibraryKind::Skills, &entry.slug)
            .join(name)
            .to_string_lossy()
            .to_string()
    });
    state
        .resources
        .upsert_library_skill(
            &entry.slug,
            &entry.metadata.title,
            entry.metadata.description.as_deref(),
            source_path.as_deref(),
        )
        .map_err(Into::<CommandError>::into)?;

    Ok(entry)
}

#[tauri::command]
pub fn skills_get(slug: String, state: State<'_, AppState>) -> CommandResult<LibraryEntryDetail> {
    state.library.skills_get(&slug).map_err(Into::into)
}

#[tauri::command]
pub fn skills_update(
    slug: String,
    metadata: JsonValue,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let existing = state
        .library
        .skills_get(&slug)
        .map_err(Into::<CommandError>::into)?;
    let metadata = merge_skill_metadata(existing.metadata, metadata)?;

    let entry = state
        .library
        .skills_update(&slug, metadata.clone())
        .map_err(Into::<CommandError>::into)?;

    let source_path = metadata.entry_file.as_ref().map(|name| {
        state
            .library
            .entry_dir(LibraryKind::Skills, &slug)
            .join(name)
            .to_string_lossy()
            .to_string()
    });
    state
        .resources
        .upsert_library_skill(&slug, &metadata.title, metadata.description.as_deref(), source_path.as_deref())
        .map_err(Into::<CommandError>::into)?;

    Ok(entry)
}

#[tauri::command]
pub fn skills_delete(slug: String, state: State<'_, AppState>) -> CommandResult<()> {
    state
        .library
        .skills_delete(&slug)
        .map_err(Into::<CommandError>::into)?;
    state
        .resources
        .delete_library_skill(&slug)
        .map_err(Into::<CommandError>::into)?;
    Ok(())
}

// ------------------------------------------------------------------
// Sub-agent commands
// ------------------------------------------------------------------

#[tauri::command]
pub fn sub_agents_list(
    search: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> CommandResult<Vec<LibraryEntry>> {
    state.library.sub_agents_list(search.as_deref(), tags.as_deref()).map_err(Into::into)
}

#[tauri::command]
pub fn sub_agents_create(
    slug: String,
    metadata: JsonValue,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let metadata = build_sub_agent_metadata(&slug, metadata)?;
    let entry = state
        .library
        .sub_agents_create(&slug, metadata)
        .map_err(Into::<CommandError>::into)?;
    Ok(entry)
}

#[tauri::command]
pub fn sub_agents_get(slug: String, state: State<'_, AppState>) -> CommandResult<LibraryEntryDetail> {
    state.library.sub_agents_get(&slug).map_err(Into::into)
}

#[tauri::command]
pub fn sub_agents_update(
    slug: String,
    metadata: JsonValue,
    state: State<'_, AppState>,
) -> CommandResult<LibraryEntry> {
    let existing = state
        .library
        .sub_agents_get(&slug)
        .map_err(Into::<CommandError>::into)?;
    let metadata = merge_sub_agent_metadata(existing.metadata, metadata)?;

    let entry = state
        .library
        .sub_agents_update(&slug, metadata)
        .map_err(Into::<CommandError>::into)?;
    Ok(entry)
}

#[tauri::command]
pub fn sub_agents_delete(slug: String, state: State<'_, AppState>) -> CommandResult<()> {
    state.library.sub_agents_delete(&slug).map_err(Into::into)
}

#[tauri::command]
pub fn sub_agent_templates(state: State<'_, AppState>) -> CommandResult<Vec<LibraryEntry>> {
    Ok(state.library.sub_agent_templates())
}

// ------------------------------------------------------------------
// Sub-agent enable / disable commands (return ChangePlan for preview flow)
// ------------------------------------------------------------------

fn sub_agents_build_plan(
    mut intent: crate::domain::ChangeIntent,
    state: State<'_, AppState>,
) -> CommandResult<crate::domain::ChangePlan> {
    // Enrich payload with sub-agent metadata from library.
    if let Some(slug) = intent.payload.get("slug").and_then(|v| v.as_str()) {
        if let Ok(detail) = state.library.sub_agents_get(slug) {
            if let Some(obj) = intent.payload.as_object_mut() {
                obj.entry("role".to_string()).or_insert_with(|| {
                    serde_json::json!(detail.metadata.role)
                });
                obj.entry("description".to_string()).or_insert_with(|| {
                    serde_json::json!(detail.metadata.description)
                });
                obj.entry("tools".to_string()).or_insert_with(|| {
                    serde_json::json!(detail.metadata.bound_mcp_ids)
                });
                obj.entry("skills".to_string()).or_insert_with(|| {
                    serde_json::json!(detail.metadata.bound_skill_ids)
                });
            }
        }
    }

    let svc = crate::services::ChangeService::new(state.db.clone());
    let mut ctx = if let Some(ref project_id) = intent.project_id {
        match state.projects.get(project_id) {
            Ok(project) => crate::adapters::ScanContext::for_project(project.path.into()),
            Err(_) => crate::adapters::ScanContext::empty(),
        }
    } else {
        crate::adapters::ScanContext::empty()
    };
    ctx = ctx.with_app_data(state.app_data.layout().root.clone());
    let plan = svc.create_plan_from_intent(&intent, &state.registry, &ctx)?;

    // Record or remove binding based on intent type.
    if let Some(slug) = intent.payload.get("slug").and_then(|v| v.as_str()) {
        if let Some(agent_kind) = intent.agent_kind {
            let scope_type = intent.scope_type.unwrap_or(crate::domain::ScopeType::Global);
            match intent.change_type.as_str() {
                "enableSubAgent" => {
                    state.resources.record_sub_agent_binding(
                        slug,
                        agent_kind,
                        scope_type,
                        intent.project_id.as_deref(),
                    ).map_err(Into::<CommandError>::into)?;
                }
                "disableSubAgent" | "deleteSubAgent" => {
                    state.resources.remove_sub_agent_binding(
                        slug,
                        agent_kind,
                        intent.project_id.as_deref(),
                    ).map_err(Into::<CommandError>::into)?;
                }
                _ => {}
            }
        }
    }

    Ok(plan)
}

#[tauri::command]
pub fn sub_agents_enable(
    intent: crate::domain::ChangeIntent,
    state: State<'_, AppState>,
) -> CommandResult<crate::domain::ChangePlan> {
    sub_agents_build_plan(intent, state)
}

#[tauri::command]
pub fn sub_agents_disable(
    intent: crate::domain::ChangeIntent,
    state: State<'_, AppState>,
) -> CommandResult<crate::domain::ChangePlan> {
    sub_agents_build_plan(intent, state)
}

// ------------------------------------------------------------------
// Skill enable / disable commands (return ChangePlan for preview flow)
// ------------------------------------------------------------------

#[tauri::command]
pub fn skills_enable(
    mut intent: crate::domain::ChangeIntent,
    state: State<'_, AppState>,
) -> CommandResult<crate::domain::ChangePlan> {
    // Enrich payload with the absolute library skill path.
    if let Some(slug) = intent.payload.get("slug").and_then(|v| v.as_str()) {
        let skill_dir = state
            .library
            .entry_dir(crate::services::LibraryKind::Skills, slug)
            .to_string_lossy()
            .to_string();
        if let Some(obj) = intent.payload.as_object_mut() {
            obj.insert("path".to_string(), serde_json::json!(skill_dir));
        }
    }

    let svc = crate::services::ChangeService::new(state.db.clone());
    let mut ctx = if let Some(ref project_id) = intent.project_id {
        match state.projects.get(project_id) {
            Ok(project) => crate::adapters::ScanContext::for_project(project.path.into()),
            Err(_) => crate::adapters::ScanContext::empty(),
        }
    } else {
        crate::adapters::ScanContext::empty()
    };
    ctx = ctx.with_app_data(state.app_data.layout().root.clone());
    let plan = svc.create_plan_from_intent(&intent, &state.registry, &ctx)?;
    Ok(plan)
}

#[tauri::command]
pub fn skills_disable(
    intent: crate::domain::ChangeIntent,
    state: State<'_, AppState>,
) -> CommandResult<crate::domain::ChangePlan> {
    skills_enable(intent, state)
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn parse_kind(kind: &str) -> CommandResult<LibraryKind> {
    LibraryKind::from_str(kind)
        .ok_or_else(|| CommandError::new("invalid_kind", format!("invalid library kind: {}", kind)))
}

/// Build a full LibraryMetadata from the slug and the JSON payload sent by
/// the frontend.  The backend owns `slug`, `created_at` and `updated_at`.
fn build_metadata(slug: &str, value: JsonValue) -> CommandResult<LibraryMetadata> {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let entry_file = value
        .get("entryFile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let now = chrono::Utc::now().to_rfc3339();
    Ok(LibraryMetadata {
        slug: slug.to_string(),
        title,
        description,
        tags,
        entry_file,
        role: None,
        agent_kinds: vec![],
        bound_mcp_ids: vec![],
        bound_skill_ids: vec![],
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Merge an existing metadata record with a partial update from the frontend.
/// Preserves `created_at`; refreshes `updated_at`.
fn merge_metadata(
    existing: LibraryMetadata,
    value: JsonValue,
) -> CommandResult<LibraryMetadata> {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or(existing.title);
    let description = if value.get("description").is_some() {
        value
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        existing.description
    };
    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or(existing.tags);
    let entry_file = if value.get("entryFile").is_some() {
        value
            .get("entryFile")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        existing.entry_file
    };

    Ok(LibraryMetadata {
        slug: existing.slug,
        title,
        description,
        tags,
        entry_file,
        role: existing.role,
        agent_kinds: existing.agent_kinds,
        bound_mcp_ids: existing.bound_mcp_ids,
        bound_skill_ids: existing.bound_skill_ids,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Merge skill metadata, same as merge_metadata but used for skill-specific
/// updates where the existing record is already a skill.
fn merge_skill_metadata(
    existing: LibraryMetadata,
    value: JsonValue,
) -> CommandResult<LibraryMetadata> {
    merge_metadata(existing, value)
}

// ------------------------------------------------------------------
// Sub-agent metadata helpers
// ------------------------------------------------------------------

fn build_sub_agent_metadata(slug: &str, value: JsonValue) -> CommandResult<LibraryMetadata> {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let role = value
        .get("role")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let agent_kinds = value
        .get("agentKinds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(|s| parse_agent_kind(s)))
                .filter(|k| !matches!(k, AgentKind::Pi))
                .collect()
        })
        .unwrap_or_default();
    let bound_mcp_ids = value
        .get("boundMcpIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let bound_skill_ids = value
        .get("boundSkillIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let now = chrono::Utc::now().to_rfc3339();
    Ok(LibraryMetadata {
        slug: slug.to_string(),
        title,
        description,
        tags,
        entry_file: None,
        role,
        agent_kinds,
        bound_mcp_ids,
        bound_skill_ids,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn merge_sub_agent_metadata(
    existing: LibraryMetadata,
    value: JsonValue,
) -> CommandResult<LibraryMetadata> {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or(existing.title);
    let description = if value.get("description").is_some() {
        value
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        existing.description
    };
    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or(existing.tags);
    let role = if value.get("role").is_some() {
        value
            .get("role")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        existing.role
    };
    let agent_kinds = value
        .get("agentKinds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(|s| parse_agent_kind(s)))
                .filter(|k| !matches!(k, AgentKind::Pi))
                .collect()
        })
        .unwrap_or(existing.agent_kinds);
    let bound_mcp_ids = value
        .get("boundMcpIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or(existing.bound_mcp_ids);
    let bound_skill_ids = value
        .get("boundSkillIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or(existing.bound_skill_ids);

    Ok(LibraryMetadata {
        slug: existing.slug,
        title,
        description,
        tags,
        entry_file: existing.entry_file,
        role,
        agent_kinds,
        bound_mcp_ids,
        bound_skill_ids,
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn parse_agent_kind(s: &str) -> Option<AgentKind> {
    match s {
        "claude-code" => Some(AgentKind::ClaudeCode),
        "codex" => Some(AgentKind::Codex),
        "opencode" => Some(AgentKind::Opencode),
        "pi" => Some(AgentKind::Pi),
        _ => None,
    }
}
