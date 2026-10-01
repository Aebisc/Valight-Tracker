// src-tauri/src/commands.rs
//
// Tauri commands exposed to the frontend.
// The frontend calls `invoke("get_match", { force })` instead of fetch("/api/match").

use tauri::State;
use crate::model::ApiResponse;
use crate::state::AppState;
use crate::service::match_service;

#[tauri::command]
pub async fn get_match(force: bool, state: State<'_, AppState>) -> Result<ApiResponse, String> {
    Ok(match_service::get_match(force, &state).await)
}
