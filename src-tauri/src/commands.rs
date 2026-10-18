// src-tauri/src/commands.rs
//
// Tauri commands exposed to the frontend.
// The frontend calls `invoke("get_match", { force })` instead of fetch("/api/match").

use crate::model::{ApiError, ApiResponse};
use crate::service::match_service;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_match(force: bool, state: State<'_, AppState>) -> Result<ApiResponse, ApiError> {
    match_service::get_match(force, &state).await
}
