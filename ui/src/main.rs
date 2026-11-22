mod ipc;
mod popup;

use leptos::prelude::*;

use crate::popup::PopupApp;

fn current_view() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
        .and_then(|p| p.get("view"))
        .unwrap_or_else(|| "popup".to_string())
}

fn main() {
    match current_view().as_str() {
        "settings" => leptos::mount::mount_to_body(|| view! { <div class="settings-root"></div> }),
        _ => leptos::mount::mount_to_body(PopupApp),
    }
}
