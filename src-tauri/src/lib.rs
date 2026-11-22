mod assets;
mod commands;
mod paths;
mod platform;
mod popup;
mod popup_window;
mod scheduler_loop;
mod settings_window;
mod state;
mod tray;

use tauri::{App, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::paths::Paths;
use crate::popup::Popup;
use crate::state::AppState;

const QUIET_SHORTCUT: &str = "CmdOrCtrl+Shift+Y";

pub fn run() {
    platform::prepare_environment();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        tray::toggle_quiet_mode(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::set_hit_rect,
            commands::popup_finished,
            commands::get_settings_view,
            commands::add_message,
            commands::update_message,
            commands::delete_message,
            commands::save_settings,
            commands::set_launch_at_login,
            commands::show_test_popup,
            commands::open_data_folder,
            commands::open_sprites_folder,
            commands::open_pack_guide,
            commands::reload_files,
            commands::finish_onboarding,
            commands::check_for_updates,
            commands::open_release_page,
        ])
        .setup(|app| {
            setup(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tospu");
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // Menu-bar-only app: no Dock icon, never steals the foreground.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let paths = Paths::resolve(app.handle())?;
    paths.ensure()?;
    let state = AppState::load(paths);
    let show_welcome = !state.settings().onboarded && state.settings_writable();
    app.manage(state);

    let window = app
        .get_webview_window("popup")
        .ok_or("popup window missing from tauri.conf.json")?;
    app.manage(Popup::new(popup_window::create(window)));

    // Registered here, not in the plugin builder, so a shortcut that can't be
    // taken (another app owns it, or a Wayland session) doesn't stop startup.
    if let Err(e) = app.global_shortcut().register(QUIET_SHORTCUT) {
        eprintln!("[tospu] couldn't register {QUIET_SHORTCUT}: {e}");
    }

    tray::create(app.handle())?;
    popup::spawn_hit_testing(app.handle().clone());
    scheduler_loop::spawn(app.handle().clone());

    if show_welcome {
        settings_window::open(app.handle());
    }

    Ok(())
}
