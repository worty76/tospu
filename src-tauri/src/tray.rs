use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::popup::{self, SNAPSHOT_CHANGED_EVENT};
use crate::state::AppState;

const TRAY_ID: &str = "tospu";
const PACK_PREFIX: &str = "pack:";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Tospu")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;
    Ok(())
}

/// Rebuild the menu after settings or packs change (check marks, pack list).
pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = app.emit(SNAPSHOT_CHANGED_EVENT, ());
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let settings = state.settings();

    let packs = Submenu::with_id(app, "packs", "Character", true)?;
    for p in state.packs() {
        let item = CheckMenuItem::with_id(
            app,
            format!("{PACK_PREFIX}{}", p.id),
            &p.id,
            true,
            p.id == settings.active_pack,
            None::<&str>,
        )?;
        packs.append(&item)?;
    }

    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show-now", "Show one now", true, None::<&str>)?,
            &CheckMenuItem::with_id(
                app,
                "quiet",
                "Quiet mode",
                true,
                settings.quiet_mode,
                Some("CmdOrCtrl+Shift+Y"),
            )?,
            &packs,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "open-content",
                "Edit goals & quotes…",
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "open-settings", "Edit schedule…", true, None::<&str>)?,
            &MenuItem::with_id(
                app,
                "open-sprites",
                "Open sprites folder",
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "reload", "Reload", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit Tospu", true, None::<&str>)?,
        ],
    )
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let state = app.state::<AppState>();
    let id = event.id().as_ref();
    match id {
        "show-now" => popup::show_next_manual(app),
        "quiet" => toggle_quiet_mode(app),
        "open-content" => open(app, &state.paths.content_file),
        "open-settings" => open(app, &state.paths.settings_file),
        "open-sprites" => open(app, &state.paths.user_sprites),
        "reload" => {
            state.reload();
            refresh(app);
        }
        "quit" => app.exit(0),
        _ => {
            if let Some(pack) = id.strip_prefix(PACK_PREFIX) {
                state.update_settings(|s| s.active_pack = pack.to_string());
                refresh(app);
            }
        }
    }
}

pub fn toggle_quiet_mode(app: &AppHandle) {
    app.state::<AppState>()
        .update_settings(|s| s.quiet_mode = !s.quiet_mode);
    refresh(app);
}

fn open(app: &AppHandle, path: &std::path::Path) {
    let _ = app.opener().open_path(path.to_string_lossy(), None::<&str>);
}
