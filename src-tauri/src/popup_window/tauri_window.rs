use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Monitor, PhysicalPosition, WebviewWindow};
use tospu_core::{DisplayTarget, MonitorInfo};

use super::PopupWindow;

/// Gap between the popup and the screen edge, in logical px.
const MARGIN: f64 = 16.0;

pub struct TauriPopupWindow {
    window: WebviewWindow,
    click_through: AtomicBool,
}

fn monitor_label(monitor: &Monitor, index: usize) -> String {
    monitor
        .name()
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Display {}", index + 1))
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

    fn pick_monitor(&self, target: &DisplayTarget) -> tauri::Result<Option<Monitor>> {
        match target {
            DisplayTarget::Cursor => {
                let by_cursor = match self.window.cursor_position() {
                    Ok(p) => self.window.monitor_from_point(p.x, p.y)?,
                    Err(_) => None,
                };
                Ok(by_cursor.or(self.window.primary_monitor()?))
            }
            DisplayTarget::Primary => self.window.primary_monitor(),
            DisplayTarget::Named { name } => {
                let matched = self
                    .window
                    .available_monitors()?
                    .into_iter()
                    .enumerate()
                    .find(|(i, m)| monitor_label(m, *i) == *name)
                    .map(|(_, m)| m);
                Ok(matched.or(self.window.primary_monitor()?))
            }
        }
    }

    fn move_to_corner(&self, target: &DisplayTarget) -> tauri::Result<()> {
        let Some(monitor) = self.pick_monitor(target)? else {
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
    fn show_at_corner(&self, target: &DisplayTarget) {
        let _ = self.move_to_corner(target);
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

    fn monitors(&self) -> Vec<MonitorInfo> {
        let primary = self.window.primary_monitor().ok().flatten();
        let Ok(list) = self.window.available_monitors() else {
            return Vec::new();
        };
        list.into_iter()
            .enumerate()
            .map(|(i, m)| {
                let scale = m.scale_factor().max(1.0);
                let size = m.size();
                let is_primary = primary
                    .as_ref()
                    .is_some_and(|p| p.position() == m.position() && p.size() == m.size());
                MonitorInfo {
                    name: monitor_label(&m, i),
                    width: (size.width as f64 / scale).round() as u32,
                    height: (size.height as f64 / scale).round() as u32,
                    is_primary,
                }
            })
            .collect()
    }
}
