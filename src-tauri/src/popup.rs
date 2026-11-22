use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tospu_core::{MessageKind, MonitorInfo, ShowPayload};

use crate::popup_window::PopupWindow;
use crate::state::AppState;

pub const SHOW_EVENT: &str = "tospu://show";

const HIT_TEST_VISIBLE: Duration = Duration::from_millis(50);
const HIT_TEST_HIDDEN: Duration = Duration::from_millis(250);
const WATCHDOG_GRACE_SECS: u64 = 20;
const WATCHDOG_HOVER_EXTEND: Duration = Duration::from_secs(10);

/// Owns the popup window and decides when it is shown.
pub struct Popup {
    window: Box<dyn PopupWindow>,
    manual_count: AtomicUsize,
    showing: AtomicBool,
    show_seq: AtomicU64,
}

impl Popup {
    pub fn new(window: Box<dyn PopupWindow>) -> Self {
        Self {
            window,
            manual_count: AtomicUsize::new(0),
            showing: AtomicBool::new(false),
            show_seq: AtomicU64::new(0),
        }
    }

    pub fn foreground_is_fullscreen(&self) -> bool {
        self.window.foreground_is_fullscreen()
    }

    pub fn monitors(&self) -> Vec<MonitorInfo> {
        self.window.monitors()
    }

    /// Atomically claim the popup slot. Returns a sequence number if the popup
    /// was idle, or `None` if another popup is already showing.
    pub(crate) fn try_begin_show(&self) -> Option<u64> {
        self.showing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        Some(self.show_seq.fetch_add(1, Ordering::AcqRel) + 1)
    }

    pub(crate) fn end_show(&self) {
        self.showing.store(false, Ordering::Release);
    }

    fn is_current_show(&self, seq: u64) -> bool {
        self.showing.load(Ordering::Acquire) && self.show_seq.load(Ordering::Acquire) == seq
    }
}

pub fn show(app: &AppHandle, kind: MessageKind) {
    let popup = app.state::<Popup>();
    let Some(seq) = popup.try_begin_show() else {
        return;
    };

    let state = app.state::<AppState>();
    let settings = state.settings();
    let display_seconds = settings.display_seconds;
    let payload = ShowPayload {
        message: state.pick(kind),
        quiet: settings.quiet_mode,
        display_seconds,
        pack: state.pack_view(),
    };
    popup.window.show_at_corner(&settings.display_target);
    let _ = app.emit_to("popup", SHOW_EVENT, payload);

    spawn_watchdog(app.clone(), seq, display_seconds);
}

/// Hides the popup if the UI never sends `popup_finished` (for example after a
/// decode error), while extending the deadline if the user is actively hovering.
fn spawn_watchdog(app: AppHandle, seq: u64, display_seconds: u32) {
    let initial_wait = Duration::from_secs(u64::from(display_seconds.max(1)) + WATCHDOG_GRACE_SECS);
    thread::spawn(move || {
        thread::sleep(initial_wait);
        loop {
            let popup = app.state::<Popup>();
            if !popup.is_current_show(seq) {
                return;
            }
            let hovered = {
                let rect = *app
                    .state::<AppState>()
                    .hit_rect
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                match (rect, popup.window.cursor_in_window()) {
                    (Some(r), Some((x, y))) => r.contains(x, y),
                    _ => false,
                }
            };
            if hovered {
                thread::sleep(WATCHDOG_HOVER_EXTEND);
                continue;
            }
            eprintln!("[tospu] popup watchdog timed out; hiding stuck popup");
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                if handle.state::<Popup>().is_current_show(seq) {
                    finish(&handle);
                }
            });
            return;
        }
    });
}

/// "Show one now" from the menu: rotate through the kinds.
pub fn show_next_manual(app: &AppHandle) {
    const ORDER: [MessageKind; 4] = [
        MessageKind::Proud,
        MessageKind::Goal,
        MessageKind::Quote,
        MessageKind::Gratitude,
    ];
    let n = app
        .state::<Popup>()
        .manual_count
        .fetch_add(1, Ordering::Relaxed);
    show(app, ORDER[n % ORDER.len()]);
}

pub fn finish(app: &AppHandle) {
    *app.state::<AppState>()
        .hit_rect
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    let popup = app.state::<Popup>();
    popup.window.hide();
    popup.end_show();
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
        let rect = *app
            .state::<AppState>()
            .hit_rect
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let over = match (rect, popup.window.cursor_in_window()) {
            (Some(r), Some((x, y))) => r.contains(x, y),
            _ => false,
        };
        popup.window.set_click_through(!over);
        thread::sleep(HIT_TEST_VISIBLE);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tospu_core::DisplayTarget;

    struct FakeWindow {
        shows: AtomicUsize,
        hides: AtomicUsize,
        visible: AtomicBool,
    }

    impl FakeWindow {
        fn new() -> Self {
            Self {
                shows: AtomicUsize::new(0),
                hides: AtomicUsize::new(0),
                visible: AtomicBool::new(false),
            }
        }
    }

    impl PopupWindow for FakeWindow {
        fn show_at_corner(&self, _target: &DisplayTarget) {
            self.shows.fetch_add(1, Ordering::Relaxed);
            self.visible.store(true, Ordering::Relaxed);
        }
        fn hide(&self) {
            self.hides.fetch_add(1, Ordering::Relaxed);
            self.visible.store(false, Ordering::Relaxed);
        }
        fn is_visible(&self) -> bool {
            self.visible.load(Ordering::Relaxed)
        }
        fn set_click_through(&self, _on: bool) {}
        fn cursor_in_window(&self) -> Option<(f64, f64)> {
            None
        }
    }

    #[test]
    fn second_show_is_ignored_while_popup_is_showing() {
        let popup = Popup::new(Box::new(FakeWindow::new()));
        assert!(popup.try_begin_show().is_some());
        assert!(popup.try_begin_show().is_none());
        popup.end_show();
        assert!(popup.try_begin_show().is_some());
    }
}
