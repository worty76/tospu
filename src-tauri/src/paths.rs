use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

const DEFAULT_CONTENT: &str = include_str!("../../defaults/content.json");
const LEGACY_IDENTIFIER: &str = "dev.yub.companion";

/// Every file location Tospu uses, resolved once at startup.
#[derive(Debug, Clone)]
pub struct Paths {
    pub settings_file: PathBuf,
    pub content_file: PathBuf,
    /// Packs the user adds; highest priority.
    pub user_sprites: PathBuf,
    /// Packs shipped inside the app bundle.
    pub bundled_sprites: Option<PathBuf>,
}

impl Paths {
    pub fn resolve(app: &AppHandle) -> tauri::Result<Self> {
        let data = app.path().app_data_dir()?;
        if let Some(parent) = data.parent() {
            let old_dir = parent.join(LEGACY_IDENTIFIER);
            if let Err(e) = tospu_core::legacy::migrate_legacy_data(&old_dir, &data) {
                eprintln!("[tospu] legacy data migration failed: {e}");
            }
        }
        Ok(Self {
            settings_file: data.join("settings.json"),
            content_file: data.join("content.json"),
            user_sprites: data.join("sprites"),
            bundled_sprites: app.path().resource_dir().ok().map(|d| d.join("sprites")),
        })
    }

    /// Create the data folders and a starter content.json on first run.
    pub fn ensure(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.user_sprites)?;
        if !self.content_file.exists() {
            fs::write(&self.content_file, DEFAULT_CONTENT)?;
        }
        Ok(())
    }

    /// Lowest priority first, as `discover_packs` expects.
    pub fn sprite_roots(&self) -> Vec<PathBuf> {
        self.bundled_sprites
            .iter()
            .cloned()
            .chain([self.user_sprites.clone()])
            .collect()
    }
}
