#![allow(non_snake_case)]

use indexmap::IndexMap;
use std::str::FromStr;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::prompt::Prompt;
use crate::services::pi_prompt_files::{
    PiPromptFileKind, PiPromptFileService, PiPromptFileSnapshot, PiPromptTemplate,
    PiPromptTemplateService,
};
use crate::services::prompt::PromptService;
use crate::store::AppState;
use cc_command_api::command_api;

#[command_api]
pub async fn get_prompts(
    app: String,
    state: &AppState,
) -> Result<IndexMap<String, Prompt>, AppError> {
    let app_type = AppType::from_str(&app)?;
    PromptService::get_prompts(state, app_type)
}

#[command_api]
pub async fn upsert_prompt(
    app: String,
    id: String,
    prompt: Prompt,
    state: &AppState,
) -> Result<(), AppError> {
    let app_type = AppType::from_str(&app)?;
    PromptService::upsert_prompt(state, app_type, &id, prompt)?;
    Ok(())
}

#[command_api]
pub async fn delete_prompt(app: String, id: String, state: &AppState) -> Result<(), AppError> {
    let app_type = AppType::from_str(&app)?;
    PromptService::delete_prompt(state, app_type, &id)?;
    Ok(())
}

#[command_api]
pub async fn enable_prompt(app: String, id: String, state: &AppState) -> Result<(), AppError> {
    let app_type = AppType::from_str(&app)?;
    PromptService::enable_prompt(state, app_type, &id)?;
    Ok(())
}

#[command_api]
pub async fn import_prompt_from_file(app: String, state: &AppState) -> Result<String, AppError> {
    let app_type = AppType::from_str(&app)?;
    PromptService::import_from_file(state, app_type)
}

#[command_api]
pub async fn get_current_prompt_file_content(app: String) -> Result<Option<String>, AppError> {
    let app_type = AppType::from_str(&app)?;
    PromptService::get_current_file_content(app_type)
}

#[command_api]
pub async fn get_pi_prompt_file(kind: PiPromptFileKind) -> Result<PiPromptFileSnapshot, AppError> {
    PiPromptFileService::read(kind)
}

#[command_api]
pub async fn replace_pi_prompt_file(
    kind: PiPromptFileKind,
    expectedRevision: String,
    content: String,
) -> Result<PiPromptFileSnapshot, AppError> {
    PiPromptFileService::replace(kind, &expectedRevision, &content)
}

#[command_api]
pub async fn delete_pi_prompt_file(
    kind: PiPromptFileKind,
    expectedRevision: String,
) -> Result<bool, AppError> {
    PiPromptFileService::delete(kind, &expectedRevision)
}

#[command_api]
pub async fn list_pi_prompt_templates() -> Result<Vec<PiPromptTemplate>, AppError> {
    PiPromptTemplateService::list()
}

#[command_api]
pub async fn upsert_pi_prompt_template(
    slug: String,
    originalSlug: Option<String>,
    expectedRevision: String,
    content: String,
) -> Result<PiPromptTemplate, AppError> {
    PiPromptTemplateService::upsert(&slug, originalSlug.as_deref(), &expectedRevision, &content)
}

#[command_api]
pub async fn delete_pi_prompt_template(
    slug: String,
    expectedRevision: String,
) -> Result<bool, AppError> {
    PiPromptTemplateService::delete(&slug, &expectedRevision)
}
