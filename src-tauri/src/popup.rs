use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use yub_core::MessageKind;

use crate::popup_window::PopupWindow;
use crate::state::AppState;

pub const SHOW_EVENT: &str = "yub://show";
pub const SNAPSHOT_CHANGED_EVENT: &str = "yub://snapshot-changed";

const HIT_TEST_VISIBLE: Duration = Duration::from_millis(50);
const HIT_TEST_HIDDEN: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize)]
pub struct ShowPayload {
    pub kind: MessageKind,
    pub quiet: bool,
}

/// Owns the popup window and decides when it is shown.
pub struct Popup {
    window: Box<dyn PopupWindow>,
    manual_count: AtomicUsize,
}

impl Popup {
    pub fn new(window: Box<dyn PopupWindow>) -> Self {
        Self { window, manual_count: AtomicUsize::new(0) }
    }

    pub fn foreground_is_fullscreen(&self) -> bool {
        self.window.foreground_is_fullscreen()
    }
}

pub fn show(app: &AppHandle, kind: MessageKind) {
    let quiet = app.state::<AppState>().settings().quiet_mode;
    app.state::<Popup>().window.show_at_corner();
    let _ = app.emit_to("popup", SHOW_EVENT, ShowPayload { kind, quiet });
}

/// "Show one now" from the menu: rotate through the kinds.
pub fn show_next_manual(app: &AppHandle) {
    const ORDER: [MessageKind; 4] = [MessageKind::Proud, MessageKind::Goal, MessageKind::Quote, MessageKind::Gratitude];
    let n = app.state::<Popup>().manual_count.fetch_add(1, Ordering::Relaxed);
    show(app, ORDER[n % ORDER.len()]);
}

pub fn finish(app: &AppHandle) {
    *app.state::<AppState>().hit_rect.lock().unwrap_or_else(|e| e.into_inner()) = None;
    app.state::<Popup>().window.hide();
}

/// Tauri can only make a whole window click-through, so while the popup is
/// visible we poll the cursor and only accept clicks over the sprite/bubble.
pub fn spawn_hit_testing(app: AppHandle) {
    thread::spawn(move || loop {
        let popup = app.state::<Popup>();
        if !popup.window.is_visible() {
            thread::sleep(HIT_TEST_HIDDEN);
            continue;
        }
        let rect = *app.state::<AppState>().hit_rect.lock().unwrap_or_else(|e| e.into_inner());
        let over = match (rect, popup.window.cursor_in_window()) {
            (Some(r), Some((x, y))) => r.contains(x, y),
            _ => false,
        };
        popup.window.set_click_through(!over);
        thread::sleep(HIT_TEST_VISIBLE);
    });
}
