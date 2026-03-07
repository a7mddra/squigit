use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn spawn_capture(app: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    crate::services::capture::spawn_capture(&app);
    Ok(())
}

#[tauri::command]
pub fn spawn_capture_to_input(app: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    crate::services::capture::spawn_capture_to_input(&app);
    Ok(())
}
