#![allow(non_snake_case)]

use crate::error::AppError;
use crate::provider::UsageScript;
use crate::services::pi_state::{PiCurrentState, PiStateService};
use crate::services::ProviderService;
use crate::session_manager::providers::pi::PiSessionDiscovery;
use crate::store::AppState;
use cc_command_api::command_api;
use tauri::State;

#[tauri::command]
pub(crate) fn get_pi_current_state(state: State<'_, AppState>) -> Result<PiCurrentState, String> {
    PiStateService::current(state.inner()).map_err(|error| error.to_string())
}

#[command_api]
pub(crate) fn update_pi_provider_usage_script(
    state: &AppState,
    id: String,
    usageScript: UsageScript,
) -> Result<bool, AppError> {
    Ok(
        ProviderService::update_pi_usage_script(state, &id, usageScript)
            .map_err(|error| error.to_string())?,
    )
}

#[command_api]
pub(crate) fn get_pi_session_discovery() -> PiSessionDiscovery {
    crate::session_manager::providers::pi::session_discovery()
}
