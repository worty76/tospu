//! The IPC surface exposed to the popup window.

use tauri::{AppHandle, State};
use tospu_core::Rect;

use crate::popup;
use crate::state::AppState;

/// The front end reports where the sprite + bubble are, so only that area
/// catches clicks. `None` makes the whole window click-through.
#[tauri::command]
pub fn set_hit_rect(state: State<'_, AppState>, rect: Option<Rect>) {
    *state.hit_rect.lock().unwrap_or_else(|e| e.into_inner()) = rect;
}

#[tauri::command]
pub fn popup_finished(app: AppHandle) {
    popup::finish(&app);
}
