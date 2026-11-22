//! The only place that knows how the popup window behaves on screen.
//!
//! Everything OS-specific goes behind [`PopupWindow`]. Today all platforms
//! use [`TauriPopupWindow`], built on Tauri's cross-platform window API.
//! When a platform needs native calls (for example detecting a full-screen
//! app on macOS, or Wayland quirks on Linux), add `macos.rs` / `linux.rs`
//! implementing the trait and pick it in [`create`].

mod tauri_window;

use tauri::WebviewWindow;
use tospu_core::{DisplayTarget, MonitorInfo};

pub use tauri_window::TauriPopupWindow;

pub trait PopupWindow: Send + Sync {
    /// Move to the bottom-right of the chosen display, then show
    /// without taking keyboard focus.
    fn show_at_corner(&self, target: &DisplayTarget);
    fn hide(&self);
    fn is_visible(&self) -> bool;
    /// `true`: clicks pass through to the apps underneath.
    fn set_click_through(&self, on: bool);
    /// Cursor position in logical px relative to the window's top-left.
    fn cursor_in_window(&self) -> Option<(f64, f64)>;
    /// Connected displays available for positioning the popup.
    fn monitors(&self) -> Vec<MonitorInfo> {
        Vec::new()
    }
    /// Whether a full-screen app (meeting, presentation) is in front.
    /// Implementations that cannot tell return `false`.
    fn foreground_is_fullscreen(&self) -> bool {
        false
    }
}

pub fn create(window: WebviewWindow) -> Box<dyn PopupWindow> {
    Box::new(TauriPopupWindow::new(window))
}
