use std::sync::Arc;
use tauri::State;

use crate::{api, db, parser, settings, AppState};

#[tauri::command]
pub fn get_sessions(state: State<'_, Arc<AppState>>) -> Result<Vec<db::SessionRow>, String> {
    api::list_sessions(&state)
}

#[tauri::command]
pub fn get_session_packets(
    state: State<'_, Arc<AppState>>,
    session_id: i64,
) -> Result<Vec<parser::TelemetryPacket>, String> {
    api::session_packets(&state, session_id)
}

#[tauri::command]
pub fn get_session_laps(
    state: State<'_, Arc<AppState>>,
    session_id: i64,
) -> Result<Vec<db::LapRow>, String> {
    api::session_laps(&state, session_id)
}

#[tauri::command]
pub fn delete_session(state: State<'_, Arc<AppState>>, session_id: i64) -> Result<(), String> {
    api::delete_session(&state, session_id)
}

#[tauri::command]
pub fn clear_all_sessions(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    api::clear_all_sessions(&state)
}

#[tauri::command]
pub fn rename_session(
    state: State<'_, Arc<AppState>>,
    session_id: i64,
    name: Option<String>,
) -> Result<(), String> {
    api::rename_session(&state, session_id, name)
}

#[tauri::command]
pub fn set_session_bookmark(
    state: State<'_, Arc<AppState>>,
    session_id: i64,
    bookmarked: bool,
) -> Result<(), String> {
    api::set_session_bookmark(&state, session_id, bookmarked)
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> settings::Settings {
    api::get_settings(&state)
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, Arc<AppState>>,
    new_settings: settings::Settings,
) -> Result<(), String> {
    api::save_settings(&state, new_settings)
}
