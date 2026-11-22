mod ipc;
mod popup;
mod settings;

use crate::popup::PopupApp;
use crate::settings::SettingsApp;

fn current_view() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
        .and_then(|p| p.get("view"))
        .unwrap_or_else(|| "popup".to_string())
}

fn main() {
    match current_view().as_str() {
        "settings" => leptos::mount::mount_to_body(SettingsApp),
        _ => leptos::mount::mount_to_body(PopupApp),
    }
}
