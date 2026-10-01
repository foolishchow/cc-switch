use crate::error::AppError;
use crate::services::omo::{OmoLocalFileData, SLIM, STANDARD};
use crate::services::OmoService;
use crate::store::AppState;
use cc_command_api::command_api;

#[command_api]
pub async fn read_omo_local_file() -> Result<OmoLocalFileData, AppError> {
    OmoService::read_local_file(&STANDARD)
}

#[command_api]
pub async fn get_current_omo_provider_id(state: &AppState) -> Result<String, AppError> {
    let provider = state
        .db
        .get_current_omo_provider("opencode", "omo")
        .map_err(|e| e.to_string())?;
    Ok(provider.map(|p| p.id).unwrap_or_default())
}

#[command_api]
pub async fn disable_current_omo(state: &AppState) -> Result<(), AppError> {
    let providers = state
        .db
        .get_all_providers("opencode")
        .map_err(|e| e.to_string())?;
    for (id, p) in &providers {
        if p.category.as_deref() == Some("omo") {
            state
                .db
                .clear_omo_provider_current("opencode", id, "omo")
                .map_err(|e| e.to_string())?;
        }
    }
    OmoService::delete_config_file(&STANDARD)
}

// ── OMO Slim commands ───────────────────────────────────────

#[command_api]
pub async fn read_omo_slim_local_file() -> Result<OmoLocalFileData, AppError> {
    OmoService::read_local_file(&SLIM)
}

#[command_api]
pub async fn get_current_omo_slim_provider_id(state: &AppState) -> Result<String, AppError> {
    let provider = state
        .db
        .get_current_omo_provider("opencode", "omo-slim")
        .map_err(|e| e.to_string())?;
    Ok(provider.map(|p| p.id).unwrap_or_default())
}

#[command_api]
pub async fn disable_current_omo_slim(state: &AppState) -> Result<(), AppError> {
    let providers = state
        .db
        .get_all_providers("opencode")
        .map_err(|e| e.to_string())?;
    for (id, p) in &providers {
        if p.category.as_deref() == Some("omo-slim") {
            state
                .db
                .clear_omo_provider_current("opencode", id, "omo-slim")
                .map_err(|e| e.to_string())?;
        }
    }
    OmoService::delete_config_file(&SLIM)
}
