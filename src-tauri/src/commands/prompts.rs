use std::collections::HashMap;

use tauri::State;

use crate::domain::PromptTemplate;
use crate::error::CommandResult;
use crate::services::PromptRenderResult;
use crate::state::AppState;

#[tauri::command]
pub fn prompts_list(
    search: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> CommandResult<Vec<PromptTemplate>> {
    state
        .library
        .prompts_list(search.as_deref(), tags.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
pub fn prompts_create(
    slug: String,
    title: String,
    category: String,
    tags: Vec<String>,
    favorite: bool,
    body: String,
    state: State<'_, AppState>,
) -> CommandResult<PromptTemplate> {
    state
        .library
        .prompts_create(&slug, &title, &category, tags, favorite, &body)
        .map_err(Into::into)
}

#[tauri::command]
pub fn prompts_get(slug: String, state: State<'_, AppState>) -> CommandResult<PromptTemplate> {
    state.library.prompts_get(&slug).map_err(Into::into)
}

#[tauri::command]
pub fn prompts_update(
    slug: String,
    title: String,
    category: String,
    tags: Vec<String>,
    favorite: bool,
    body: String,
    state: State<'_, AppState>,
) -> CommandResult<PromptTemplate> {
    state
        .library
        .prompts_update(&slug, &title, &category, tags, favorite, &body)
        .map_err(Into::into)
}

#[tauri::command]
pub fn prompts_delete(slug: String, state: State<'_, AppState>) -> CommandResult<()> {
    state.library.prompts_delete(&slug).map_err(Into::into)
}

#[tauri::command]
pub fn prompts_render(
    slug: String,
    values: HashMap<String, String>,
    state: State<'_, AppState>,
) -> CommandResult<PromptRenderResult> {
    state
        .library
        .prompts_render(&slug, &values)
        .map_err(Into::into)
}
