//! The IPC surface exposed to the popup and settings windows.

use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;
use tospu_core::update::{
    parse_github_release, ALLOWED_RELEASE_PREFIX, RELEASES_API_URL, RELEASES_PAGE_URL,
};
use tospu_core::{
    new_id, MessageDraft, MessageItem, MessageKind, OnboardingInput, Rect, Settings, SettingsView,
    UpdateInfo,
};

use crate::popup::{self, Popup};
use crate::settings_window;
use crate::state::{apply_onboarding, AppState};
use crate::tray;

const PACK_GUIDE_URL: &str =
    "https://github.com/worty76/tospu/blob/main/docs/creating-a-sprite-pack.md";

fn is_autolaunch_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn build_settings_view(app: &AppHandle, state: &AppState) -> SettingsView {
    let mut view = state.settings_view(is_autolaunch_enabled(app));
    if let Some(popup) = app.try_state::<Popup>() {
        view.monitors = popup.monitors();
    }
    view
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
    Ok(build_settings_view(&app, &state))
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
    Ok(build_settings_view(&app, &state))
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

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let user_agent = format!("Tospu/{current}");
        let res = ureq::get(RELEASES_API_URL)
            .header("User-Agent", &user_agent)
            .header("Accept", "application/vnd.github+json")
            .call();

        match res {
            Ok(mut response) => {
                let body = response
                    .body_mut()
                    .read_to_string()
                    .map_err(|e| format!("could not read release response: {e}"))?;
                parse_github_release(&body, &current)
            }
            Err(ureq::Error::StatusCode(404)) => Ok(UpdateInfo {
                current_version: current.clone(),
                latest_version: current,
                update_available: false,
                release_url: RELEASES_PAGE_URL.to_string(),
                notes: None,
            }),
            Err(e) => Err(format!("could not check for updates: {e}")),
        }
    })
    .await
    .map_err(|e| format!("update check failed: {e}"))?
}

#[tauri::command]
pub fn open_release_page(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with(ALLOWED_RELEASE_PREFIX) {
        return Err("invalid release URL".into());
    }
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}
