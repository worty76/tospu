use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex, MutexGuard};

use tospu_core::{
    discover_packs, settings_store, ContentStore, Message, MessageKind, Pack, PackScan, PackView,
    Picker, Rect, Settings,
};

use crate::assets;
use crate::paths::Paths;

const DEFAULT_CONTENT: &str = include_str!("../../defaults/content.json");
const PICKER_HISTORY: usize = 3;

struct Loaded {
    settings: Settings,
    scan: PackScan,
    settings_error: Option<String>,
    content: ContentStore,
    picker: Picker,
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
        let loaded = Self::read(&paths, Picker::new(PICKER_HISTORY));
        Self {
            paths,
            loaded: Mutex::new(loaded),
            hit_rect: Mutex::new(None),
            generation: Mutex::new(0),
            changed: Condvar::new(),
        }
    }

    fn read(paths: &Paths, picker: Picker) -> Loaded {
        let (settings, settings_error) = settings_store::load_or_create(&paths.settings_file);
        let scan = discover_packs(&paths.sprite_roots());
        let content = ContentStore::open(&paths.content_file, DEFAULT_CONTENT);
        Loaded {
            settings,
            scan,
            settings_error,
            content,
            picker,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Loaded> {
        self.loaded.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn settings(&self) -> Settings {
        self.lock().settings.clone()
    }

    pub fn packs(&self) -> Vec<Pack> {
        self.lock().scan.packs.clone()
    }

    /// Re-read settings.json, content.json, and rescan sprite folders.
    pub fn reload(&self) {
        let mut l = self.lock();
        let (settings, settings_error) = settings_store::load_or_create(&self.paths.settings_file);
        l.settings = settings;
        l.settings_error = settings_error;
        l.scan = discover_packs(&self.paths.sprite_roots());
        l.content.reload();
        drop(l);
        self.bump();
    }

    /// Change settings in memory and on disk.
    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
        let mut l = self.lock();
        f(&mut l.settings);
        l.settings_error = settings_store::save(&self.paths.settings_file, &l.settings)
            .err()
            .map(|e| format!("saving settings: {e}"));
        drop(l);
        self.bump();
    }

    pub fn pick(&self, kind: MessageKind) -> Message {
        let mut l = self.lock();
        let Loaded {
            content, picker, ..
        } = &mut *l;
        picker.next(kind, content.file(), &mut |n| fastrand::usize(..n))
    }

    pub fn pack_view(&self) -> Option<PackView> {
        let l = self.lock();
        l.scan.choose(&l.settings.active_pack).map(pack_to_view)
    }

    pub fn generation(&self) -> MutexGuard<'_, u64> {
        self.generation.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn bump(&self) {
        *self.generation() += 1;
        self.changed.notify_all();
    }
}

pub fn pack_to_view(pack: &Pack) -> PackView {
    let sheet_urls: BTreeMap<String, String> = pack
        .manifest
        .sheet_names()
        .into_iter()
        .map(|sheet| (sheet.to_string(), assets::asset_url(&pack.dir.join(sheet))))
        .collect();
    PackView {
        id: pack.id.clone(),
        manifest: pack.manifest.clone(),
        sheet_urls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tospu_core::FALLBACK_LINE;

    fn test_paths(dir: &std::path::Path) -> Paths {
        Paths {
            settings_file: dir.join("settings.json"),
            content_file: dir.join("content.json"),
            user_sprites: dir.join("sprites"),
            bundled_sprites: None,
        }
    }

    #[test]
    fn pack_view_has_a_url_for_every_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let pack_dir = dir.path().join("sprites/mochi");
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(
            pack_dir.join("manifest.json"),
            include_str!("../../sprites/mochi/manifest.json"),
        )
        .unwrap();

        let state = AppState::load(test_paths(dir.path()));
        let view = state.pack_view().expect("mochi pack should be found");
        assert_eq!(view.id, "mochi");
        for sheet in ["idle.png", "wave.png", "cheer.png", "sleep.png"] {
            assert!(
                view.sheet_urls.contains_key(sheet),
                "missing url for {sheet}"
            );
        }
    }

    #[test]
    fn pick_returns_fallback_with_empty_content() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("content.json"),
            r#"{"version":2,"messages":[]}"#,
        )
        .unwrap();
        let state = AppState::load(test_paths(dir.path()));
        let msg = state.pick(MessageKind::Goal);
        assert_eq!(msg.text, FALLBACK_LINE);
    }
}
