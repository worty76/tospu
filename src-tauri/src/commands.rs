//! The IPC surface exposed to the popup and settings windows.

use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;
use tospu_core::{
    new_id, MessageDraft, MessageItem, MessageKind, OnboardingInput, Rect, Settings, SettingsView,
};

use crate::popup;
use crate::settings_window;
use crate::state::{apply_onboarding, AppState};
use crate::tray;

const PACK_GUIDE_URL: &str =
    "https://github.com/worty76/tospu/blob/main/docs/creating-a-sprite-pack.md";

fn is_autolaunch_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

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

#[tauri::command]
pub fn get_settings_view(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SettingsView, String> {
    Ok(state.settings_view(is_autolaunch_enabled(&app)))
}

#[tauri::command]
pub fn add_message(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: MessageDraft,
) -> Result<MessageItem, String> {
    let item = state.change_content(|file| file.add(draft, new_id()).cloned())?;
    settings_window::notify_changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn update_message(
    app: AppHandle,
    state: State<'_, AppState>,
    item: MessageItem,
) -> Result<MessageItem, String> {
    let updated = state.change_content(|file| file.update(item).cloned())?;
    settings_window::notify_changed(&app);
    Ok(updated)
}

#[tauri::command]
pub fn delete_message(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<MessageItem, String> {
    let removed = state.change_content(|file| file.remove(&id))?;
    settings_window::notify_changed(&app);
    Ok(removed)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, Vec<String>> {
    let saved = state.update_settings(|s| *s = settings)?;
    tray::refresh(&app);
    Ok(saved)
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable()
            .map_err(|e| format!("could not enable launch at login: {e}"))?;
    } else {
        mgr.disable()
            .map_err(|e| format!("could not disable launch at login: {e}"))?;
    }
    let current = mgr.is_enabled().unwrap_or(enabled);
    settings_window::notify_changed(&app);
    Ok(current)
}

#[tauri::command]
pub fn show_test_popup(app: AppHandle) -> Result<(), String> {
    popup::show_next_manual(&app);
    Ok(())
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let dir = state
        .paths
        .settings_file
        .parent()
        .unwrap_or(&state.paths.user_sprites);
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_sprites_folder(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    app.opener()
        .open_path(state.paths.user_sprites.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_pack_guide(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(PACK_GUIDE_URL, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reload_files(app: AppHandle, state: State<'_, AppState>) -> Result<SettingsView, String> {
    state.reload();
    tray::refresh(&app);
    Ok(state.settings_view(is_autolaunch_enabled(&app)))
}

#[tauri::command]
pub fn finish_onboarding(
    app: AppHandle,
    state: State<'_, AppState>,
    input: OnboardingInput,
) -> Result<(), String> {
    apply_onboarding(&state, &input)?;
    let mgr = app.autolaunch();
    if input.launch_at_login {
        let _ = mgr.enable();
    } else {
        let _ = mgr.disable();
    }
    tray::refresh(&app);
    settings_window::close(&app);
    popup::show(&app, MessageKind::Goal);
    Ok(())
}
