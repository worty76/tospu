use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{PhysicalPosition, WebviewWindow};

use super::PopupWindow;

/// Gap between the popup and the screen edge, in logical px.
const MARGIN: f64 = 16.0;

pub struct TauriPopupWindow {
    window: WebviewWindow,
    click_through: AtomicBool,
}

impl TauriPopupWindow {
    pub fn new(window: WebviewWindow) -> Self {
        let _ = window.set_visible_on_all_workspaces(true);
        let _ = window.set_always_on_top(true);
        // Click-through is applied after the first `show`: on Linux, changing
        // input regions of a never-shown (unrealized) window panics in tao.
        Self {
            window,
            click_through: AtomicBool::new(false),
        }
    }

    fn move_to_corner(&self) -> tauri::Result<()> {
        let monitor = match self.window.cursor_position() {
            Ok(p) => self.window.monitor_from_point(p.x, p.y)?,
            Err(_) => None,
        };
        let Some(monitor) = monitor.or(self.window.primary_monitor()?) else {
            return Ok(());
        };
        let area = monitor.work_area();
        let size = self.window.outer_size()?;
        let margin = (MARGIN * monitor.scale_factor()) as i32;
        let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
        let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
        self.window.set_position(PhysicalPosition::new(x, y))
    }
}

impl PopupWindow for TauriPopupWindow {
    fn show_at_corner(&self) {
        let _ = self.move_to_corner();
        // `show` doesn't focus a window created with `focus: false`.
        let _ = self.window.show();
        let _ = self.window.set_always_on_top(true);
        // Force it on (not just on change): the window may be new.
        self.click_through.store(true, Ordering::Relaxed);
        let _ = self.window.set_ignore_cursor_events(true);
    }

    fn hide(&self) {
        let _ = self.window.hide();
    }

    fn is_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(false)
    }

    fn set_click_through(&self, on: bool) {
        if !self.is_visible() {
            return; // applied again on the next show
        }
        if self.click_through.swap(on, Ordering::Relaxed) != on {
            let _ = self.window.set_ignore_cursor_events(on);
        }
    }

    fn cursor_in_window(&self) -> Option<(f64, f64)> {
        let cursor = self.window.cursor_position().ok()?;
        let origin = self.window.inner_position().ok()?;
        let scale = self.window.scale_factor().ok()?;
        Some((
            (cursor.x - origin.x as f64) / scale,
            (cursor.y - origin.y as f64) / scale,
        ))
    }
}
