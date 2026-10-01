use crate::error::AppError;
use crate::provider::UsageResult;
use cc_command_api::command_api;

#[command_api(rest = "GET /control/v1/balance", query = "base_url,api_key")]
pub async fn get_balance(base_url: String, api_key: String) -> Result<UsageResult, AppError> {
    crate::services::balance::get_balance(&base_url, &api_key)
        .await
        .map_err(AppError::from)
}
