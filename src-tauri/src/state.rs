use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex, MutexGuard};

use chrono::Local;
use tospu_core::{
    describe_next, discover_packs, new_id, next_fire, settings_store, ContentError, ContentFile,
    ContentStore, Message, MessageDraft, MessageKind, OnboardingInput, Pack, PackScan, PackSummary,
    PackView, Picker, Rect, Settings, SettingsView,
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

    /// Validate settings on a clone, save atomically, and bump scheduler generation on success.
    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings, Vec<String>> {
        let mut l = self.lock();
        let mut next = l.settings.clone();
        f(&mut next);
        next.validate()?;
        settings_store::save(&self.paths.settings_file, &next)
            .map_err(|e| vec![format!("saving settings: {e}")])?;
        l.settings = next.clone();
        l.settings_error = None;
        drop(l);
        self.bump();
        Ok(next)
    }

    pub fn change_content<T>(
        &self,
        f: impl FnOnce(&mut ContentFile) -> Result<T, ContentError>,
    ) -> Result<T, String> {
        self.lock().content.change(f)
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

    pub fn settings_view(&self, launch_at_login: bool) -> SettingsView {
        let l = self.lock();
        let mut packs: Vec<PackSummary> = l
            .scan
            .packs
            .iter()
            .map(|p| PackSummary {
                id: p.id.clone(),
                name: p.manifest.name.clone(),
                author: p.manifest.author.clone(),
                view: Some(pack_to_view(p)),
                error: None,
            })
            .chain(l.scan.broken.iter().map(|b| PackSummary {
                id: b.id.clone(),
                name: b.id.clone(),
                author: None,
                view: None,
                error: Some(b.reason.clone()),
            }))
            .collect();
        packs.sort_by(|a, b| a.id.cmp(&b.id));

        let problems: Vec<String> = l
            .settings_error
            .iter()
            .cloned()
            .chain(l.content.problem().map(str::to_string))
            .chain(l.scan.errors.iter().cloned())
            .collect();

        let now = Local::now().naive_local();
        let next_popup = next_fire(now, &l.settings).map(|f| describe_next(now, f.at));

        #[cfg(target_os = "macos")]
        let shortcut_label = "⌘⇧Y".to_string();
        #[cfg(not(target_os = "macos"))]
        let shortcut_label = "Ctrl+Shift+Y".to_string();

        SettingsView {
            settings: l.settings.clone(),
            messages: l.content.file().messages.clone(),
            packs,
            monitors: Vec::new(),
            problems,
            launch_at_login,
            next_popup,
            shortcut_label,
            version: env!("CARGO_PKG_VERSION").to_string(),
            content_writable: l.content.problem().is_none(),
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

pub fn apply_onboarding(state: &AppState, input: &OnboardingInput) -> Result<(), String> {
    let drafts: Vec<MessageDraft> = [
        (MessageKind::Goal, &input.goal),
        (MessageKind::Gratitude, &input.gratitude),
        (MessageKind::Proud, &input.proud),
    ]
    .into_iter()
    .filter_map(|(kind, opt)| {
        let text = opt.as_deref()?.trim();
        (!text.is_empty()).then(|| MessageDraft {
            kind,
            text: text.to_string(),
            author: None,
        })
    })
    .collect();

    if !drafts.is_empty() {
        state.change_content(|file| {
            for draft in drafts {
                file.add(draft, new_id())?;
            }
            Ok(())
        })?;
    }

    let pack = input.active_pack.trim().to_string();
    state
        .update_settings(|s| {
            s.onboarded = true;
            if !pack.is_empty() {
                s.active_pack = pack;
            }
        })
        .map_err(|errs| errs.join("; "))?;

    Ok(())
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

    #[test]
    fn update_settings_rejects_invalid_and_keeps_disk() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::load(test_paths(dir.path()));
        let before_disk = fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let before_gen = *state.generation();

        let res = state.update_settings(|s| s.display_seconds = 1);
        assert!(res.is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json")).unwrap(),
            before_disk
        );
        assert_eq!(*state.generation(), before_gen);
    }

    #[test]
    fn update_settings_bumps_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::load(test_paths(dir.path()));
        let before_gen = *state.generation();
        state.update_settings(|s| s.display_seconds = 12).unwrap();
        assert_eq!(*state.generation(), before_gen + 1);
    }

    #[test]
    fn change_content_persists_and_view_reflects_it() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::load(test_paths(dir.path()));
        state
            .change_content(|f| {
                f.add(
                    MessageDraft {
                        kind: MessageKind::Goal,
                        text: "Ship Tospu".into(),
                        author: None,
                    },
                    new_id(),
                )
                .cloned()
            })
            .unwrap();
        let view = state.settings_view(false);
        assert!(view.messages.iter().any(|m| m.text == "Ship Tospu"));
    }

    #[test]
    fn settings_view_lists_broken_packs_with_reason() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("sprites/broken-fox");
        fs::create_dir_all(&bad).unwrap();
        fs::write(bad.join("manifest.json"), "{ invalid").unwrap();

        let state = AppState::load(test_paths(dir.path()));
        let view = state.settings_view(false);
        let summary = view
            .packs
            .iter()
            .find(|p| p.id == "broken-fox")
            .expect("broken pack listed");
        assert!(summary.error.is_some());
        assert!(summary.view.is_none());
    }

    #[test]
    fn onboarding_adds_non_empty_lines_only_and_sets_onboarded() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("content.json"),
            r#"{"version":2,"messages":[]}"#,
        )
        .unwrap();
        let state = AppState::load(test_paths(dir.path()));
        let input = OnboardingInput {
            goal: Some("Launch Tospu".into()),
            gratitude: Some("   ".into()),
            proud: None,
            active_pack: "mochi".into(),
            launch_at_login: true,
        };
        apply_onboarding(&state, &input).unwrap();
        let view = state.settings_view(true);
        assert!(view.settings.onboarded);
        assert_eq!(view.messages.len(), 1);
        assert_eq!(view.messages[0].kind, MessageKind::Goal);
        assert_eq!(view.messages[0].text, "Launch Tospu");
    }
}
