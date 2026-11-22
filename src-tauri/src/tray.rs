use std::fs;
use std::path::Path;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::popup;
use crate::settings_window;
use crate::state::AppState;

const TRAY_ID: &str = "tospu";
const PACK_PREFIX: &str = "pack:";
const TRAY_ICON_SIZE: u32 = 44;
const TRAY_ICON_PAD: u32 = 2;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let icon = active_tray_icon(app)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Tospu")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;
    Ok(())
}

/// Rebuild the menu and icon after settings or packs change and notify the settings window.
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
        if let Ok(icon) = active_tray_icon(app) {
            let _ = tray.set_icon(Some(icon));
        }
    }
    settings_window::notify_changed(app);
}

fn active_tray_icon(app: &AppHandle) -> tauri::Result<Image<'static>> {
    if let Some(dir) = app.state::<AppState>().active_pack_dir() {
        if let Some(icon) = load_pack_tray_icon(&dir) {
            return Ok(icon);
        }
    }
    Ok(Image::from_bytes(include_bytes!("../icons/tray.png"))?.to_owned())
}

fn load_pack_tray_icon(pack_dir: &Path) -> Option<Image<'static>> {
    let bytes = fs::read(pack_dir.join("preview.png")).ok()?;
    let decoded = Image::from_bytes(&bytes).ok()?;
    fit_tray_icon(&decoded, TRAY_ICON_SIZE)
}

/// Trims transparent margins and scales the sprite into a square `size x size`
/// RGBA tray icon using bilinear sampling.
fn fit_tray_icon(src: &Image<'_>, size: u32) -> Option<Image<'static>> {
    let sw = src.width();
    let sh = src.height();
    let rgba = src.rgba();
    if sw == 0 || sh == 0 || rgba.len() < (sw * sh * 4) as usize {
        return None;
    }

    let mut min_x = sw;
    let mut min_y = sh;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for y in 0..sh {
        for x in 0..sw {
            let a = rgba[((y * sw + x) * 4 + 3) as usize];
            if a > 16 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return None;
    }

    let cw = (max_x - min_x + 1) as f64;
    let ch = (max_y - min_y + 1) as f64;
    let inner = (size.saturating_sub(TRAY_ICON_PAD * 2)).max(1) as f64;
    let scale = (inner / cw).min(inner / ch);
    let dw = (cw * scale).round().max(1.0);
    let dh = (ch * scale).round().max(1.0);
    let ox = (size as f64 - dw) / 2.0;
    let oy = (size as f64 - dh) / 2.0;

    let mut out = vec![0u8; (size * size * 4) as usize];
    for dy in 0..size {
        let fy = (dy as f64 + 0.5 - oy) / scale + min_y as f64 - 0.5;
        if fy < min_y as f64 - 0.5 || fy > max_y as f64 + 0.5 {
            continue;
        }
        for dx in 0..size {
            let fx = (dx as f64 + 0.5 - ox) / scale + min_x as f64 - 0.5;
            if fx < min_x as f64 - 0.5 || fx > max_x as f64 + 0.5 {
                continue;
            }
            let px = sample_bilinear(rgba, sw, sh, fx, fy);
            let idx = ((dy * size + dx) * 4) as usize;
            out[idx..idx + 4].copy_from_slice(&px);
        }
    }
    Some(Image::new_owned(out, size, size))
}

fn sample_bilinear(rgba: &[u8], w: u32, h: u32, x: f64, y: f64) -> [u8; 4] {
    let x0 = (x.floor() as i64).clamp(0, w as i64 - 1) as u32;
    let y0 = (y.floor() as i64).clamp(0, h as i64 - 1) as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let tx = (x - x.floor()).clamp(0.0, 1.0);
    let ty = (y - y.floor()).clamp(0.0, 1.0);

    let at = |px: u32, py: u32| -> [f64; 4] {
        let i = ((py * w + px) * 4) as usize;
        let a = rgba[i + 3] as f64;
        // Premultiply RGB by alpha so transparent neighbors don't darken edges.
        [
            rgba[i] as f64 * a,
            rgba[i + 1] as f64 * a,
            rgba[i + 2] as f64 * a,
            a,
        ]
    };

    let c00 = at(x0, y0);
    let c10 = at(x1, y0);
    let c01 = at(x0, y1);
    let c11 = at(x1, y1);

    let w00 = (1.0 - tx) * (1.0 - ty);
    let w10 = tx * (1.0 - ty);
    let w01 = (1.0 - tx) * ty;
    let w11 = tx * ty;

    let mut acc = [0.0f64; 4];
    for ch in 0..4 {
        acc[ch] = c00[ch] * w00 + c10[ch] * w10 + c01[ch] * w01 + c11[ch] * w11;
    }
    let a = acc[3].round().clamp(0.0, 255.0);
    if a <= 0.0 {
        return [0, 0, 0, 0];
    }
    [
        (acc[0] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[1] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[2] / acc[3]).round().clamp(0.0, 255.0) as u8,
        a as u8,
    ]
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let settings = state.settings();

    let packs = Submenu::with_id(app, "packs", "Character", true)?;
    for p in state.packs() {
        let item = CheckMenuItem::with_id(
            app,
            format!("{PACK_PREFIX}{}", p.id),
            &p.manifest.name,
            true,
            p.id == settings.active_pack,
            None::<&str>,
        )?;
        packs.append(&item)?;
    }

    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show-now", "Show one now", true, None::<&str>)?,
            &CheckMenuItem::with_id(
                app,
                "quiet",
                "Quiet mode",
                true,
                settings.quiet_mode,
                Some("CmdOrCtrl+Shift+Y"),
            )?,
            &packs,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit Tospu", true, None::<&str>)?,
        ],
    )
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let state = app.state::<AppState>();
    let id = event.id().as_ref();
    match id {
        "show-now" => popup::show_next_manual(app),
        "quiet" => toggle_quiet_mode(app),
        "settings" => settings_window::open(app),
        "quit" => app.exit(0),
        _ => {
            if let Some(pack) = id.strip_prefix(PACK_PREFIX) {
                let _ = state.update_settings(|s| s.active_pack = pack.to_string());
                refresh(app);
            }
        }
    }
}

pub fn toggle_quiet_mode(app: &AppHandle) {
    let _ = app
        .state::<AppState>()
        .update_settings(|s| s.quiet_mode = !s.quiet_mode);
    refresh(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_mochi_preview_into_44x44_tray_icon() {
        let raw = Image::from_bytes(include_bytes!("../../sprites/mochi/preview.png")).unwrap();
        let icon = fit_tray_icon(&raw, TRAY_ICON_SIZE).expect("non-empty icon");
        assert_eq!(icon.width(), 44);
        assert_eq!(icon.height(), 44);
        assert!(icon.rgba().as_chunks::<4>().0.iter().any(|px| px[3] > 200));
    }
}
