use std::fs;
use std::sync::{Condvar, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use tospu_core::{discover_packs, PackEntry, PackScan, Settings};

use crate::paths::Paths;

/// Area of the popup (logical px, relative to the window) that should take
/// clicks. Everything else passes clicks through to the apps behind.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.width && y <= self.y + self.height
    }
}

/// Everything the front end needs to render a popup, read fresh each time.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub packs: Vec<PackEntry>,
    pub content_json: Option<String>,
    pub errors: Vec<String>,
}

struct Loaded {
    settings: Settings,
    scan: PackScan,
    settings_error: Option<String>,
}

pub struct AppState {
    pub paths: Paths,
    loaded: Mutex<Loaded>,
    pub hit_rect: Mutex<Option<Rect>>,
    /// Bumped whenever settings change, so the scheduler re-plans.
    generation: Mutex<u64>,
    pub changed: Condvar,
}

impl AppState {
    pub fn load(paths: Paths) -> Self {
        let loaded = Self::read(&paths);
        Self {
            paths,
            loaded: Mutex::new(loaded),
            hit_rect: Mutex::new(None),
            generation: Mutex::new(0),
            changed: Condvar::new(),
        }
    }

    fn read(paths: &Paths) -> Loaded {
        let (settings, settings_error) = Settings::load_or_create(&paths.settings_file);
        Loaded {
            settings,
            scan: discover_packs(&paths.sprite_roots()),
            settings_error,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Loaded> {
        self.loaded.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn settings(&self) -> Settings {
        self.lock().settings.clone()
    }

    pub fn packs(&self) -> Vec<PackEntry> {
        self.lock().scan.packs.clone()
    }

    /// Re-read settings.json and rescan sprite folders.
    pub fn reload(&self) {
        *self.lock() = Self::read(&self.paths);
        self.bump();
    }

    /// Change settings in memory and on disk.
    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
        let mut l = self.lock();
        f(&mut l.settings);
        l.settings_error = l
            .settings
            .save(&self.paths.settings_file)
            .err()
            .map(|e| format!("saving settings: {e}"));
        drop(l);
        self.bump();
    }

    pub fn snapshot(&self) -> Snapshot {
        let l = self.lock();
        let (content_json, content_error) = match fs::read_to_string(&self.paths.content_file) {
            Ok(text) => (Some(text), None),
            Err(e) => (None, Some(format!("content.json: {e}"))),
        };
        let errors = l
            .settings_error
            .iter()
            .cloned()
            .chain(l.scan.errors.iter().cloned())
            .chain(content_error)
            .collect();
        Snapshot {
            settings: l.settings.clone(),
            packs: l.scan.packs.clone(),
            content_json,
            errors,
        }
    }

    pub fn generation(&self) -> MutexGuard<'_, u64> {
        self.generation.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn bump(&self) {
        *self.generation() += 1;
        self.changed.notify_all();
    }
}
