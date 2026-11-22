use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(target_os = "macos"))]
use tauri::PhysicalPosition;
use tauri::Position;
#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, LogicalSize};
use tauri::{Monitor, WebviewWindow};
use tospu_core::{DisplayTarget, MonitorInfo};

use super::PopupWindow;

/// Gap between the popup and the screen edge, in logical px.
const MARGIN: f64 = 16.0;
/// Logical size of the popup window; keep in sync with `tauri.conf.json`.
#[cfg(not(target_os = "macos"))]
const POPUP_SIZE: (f64, f64) = (420.0, 340.0);

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

    /// Cursor position in the coordinate space expected by `monitor_from_point`
    /// and logical screen points on macOS.
    ///
    /// On macOS, `tao::cursor_position()` multiplies logical screen points by
    /// `primary_monitor().scale_factor()`, whereas `monitor_from_point()`
    /// compares against `CGDisplayBounds` in logical points.
    fn cursor_point_for_monitor(&self) -> Option<(f64, f64)> {
        let p = self.window.cursor_position().ok()?;
        #[cfg(target_os = "macos")]
        {
            let scale = self
                .window
                .primary_monitor()
                .ok()
                .flatten()
                .map(|m| m.scale_factor())
                .unwrap_or(1.0)
                .max(1.0);
            Some((p.x / scale, p.y / scale))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Some((p.x, p.y))
        }
    }

    fn pick_monitor(&self, target: &DisplayTarget) -> tauri::Result<Option<Monitor>> {
        match target {
            DisplayTarget::Cursor => {
                let by_cursor = match self.cursor_point_for_monitor() {
                    Some((x, y)) => self.window.monitor_from_point(x, y)?,
                    None => None,
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

    /// Moves the window to the corner and returns the position it set
    /// (`None` on macOS or when there is no monitor).
    fn move_to_corner(&self, target: &DisplayTarget) -> tauri::Result<Option<Position>> {
        let Some(monitor) = self.pick_monitor(target)? else {
            return Ok(None);
        };
        #[cfg(not(target_os = "macos"))]
        {
            // Work in the target monitor's physical pixels: a logical position
            // would be converted with the window's current scale, which is
            // wrong when monitors use different scaling (common on Windows).
            let m_scale = monitor.scale_factor();
            let area = monitor.work_area();
            let w_scale = self.window.scale_factor()?;
            let size = self.window.outer_size()?;
            // Before its first show GTK reports a placeholder size; fall back
            // to the size set in tauri.conf.json.
            let (logical_w, logical_h) = if size.width < 100 || size.height < 100 {
                POPUP_SIZE
            } else {
                (size.width as f64 / w_scale, size.height as f64 / w_scale)
            };
            let win_w = logical_w * m_scale;
            let win_h = logical_h * m_scale;
            let margin = MARGIN * m_scale;
            let x = area.position.x as f64 + area.size.width as f64 - win_w - margin;
            let y = area.position.y as f64 + area.size.height as f64 - win_h - margin;
            let pos = Position::Physical(PhysicalPosition::new(x.round() as i32, y.round() as i32));
            self.window.set_position(pos)?;
            Ok(Some(pos))
        }
        #[cfg(target_os = "macos")]
        {
            self.move_to_corner_logical(&monitor).map(|()| None)
        }
    }

    #[cfg(target_os = "macos")]
    fn move_to_corner_logical(&self, monitor: &Monitor) -> tauri::Result<()> {
        let m_scale = monitor.scale_factor().max(1.0);
        let area = monitor.work_area();
        let area_pos: LogicalPosition<f64> = area.position.to_logical(m_scale);
        let area_size: LogicalSize<f64> = area.size.to_logical(m_scale);

        let w_scale = self.window.scale_factor()?.max(1.0);
        let win_size: LogicalSize<f64> = self.window.outer_size()?.to_logical(w_scale);

        let x = area_pos.x + area_size.width - win_size.width - MARGIN;
        let y = area_pos.y + area_size.height - win_size.height - MARGIN;
        self.window.set_position(LogicalPosition::new(x, y))
    }
}

impl PopupWindow for TauriPopupWindow {
    fn show_at_corner(&self, target: &DisplayTarget) {
        #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
        let placed = self.move_to_corner(target);
        // `show` doesn't focus a window created with `focus: false`.
        let _ = self.window.show();
        // X11 window managers place a never-shown window themselves and drop
        // the earlier move, so apply the same position again once it is
        // mapped. Don't recompute: GTK reports a placeholder size right now.
        #[cfg(target_os = "linux")]
        if let Ok(Some(pos)) = &placed {
            let _ = self.window.set_position(*pos);
        }
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
        let origin = self.window.inner_position().ok()?;
        let win_scale = self.window.scale_factor().ok()?.max(1.0);
        #[cfg(target_os = "macos")]
        {
            let (cx, cy) = self.cursor_point_for_monitor()?;
            let origin_logical: LogicalPosition<f64> = origin.to_logical(win_scale);
            Some((cx - origin_logical.x, cy - origin_logical.y))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let cursor = self.window.cursor_position().ok()?;
            Some((
                (cursor.x - origin.x as f64) / win_scale,
                (cursor.y - origin.y as f64) / win_scale,
            ))
        }
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
