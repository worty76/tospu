//! Single-instance settings window lifecycle and macOS activation policy.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const DATA_CHANGED_EVENT: &str = "tospu://data-changed";

pub fn notify_changed(app: &AppHandle) {
    let _ = app.emit_to("settings", DATA_CHANGED_EVENT, ());
}

pub fn open(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }

    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    let builder = WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html?view=settings".into()),
    )
    .title("Tospu Settings")
    .inner_size(760.0, 560.0)
    .min_inner_size(640.0, 480.0)
    .center()
    .focused(true);

    match builder.build() {
        Ok(win) => {
            #[cfg(target_os = "macos")]
            {
                let handle = app.clone();
                win.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Destroyed) {
                        let _ = handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                });
            }
            #[cfg(not(target_os = "macos"))]
            let _ = win;
        }
        Err(e) => {
            eprintln!("[tospu] failed to open settings window: {e}");
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
}

pub fn close(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.close();
    }
}
