# Tospu Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename Yub to Tospu, replace the TypeScript popup with a Rust (Leptos/wasm) UI, and add a settings window where users manage their messages, schedule, character and general options.

**Architecture:** A root Cargo workspace with four members: `tospu-model` (serde types and pure logic, builds for native and wasm32), `tospu-core` (file IO, stores, scheduler, picker), `src-tauri` (shell) and `ui` (Leptos CSR app built by trunk, routed by `?view=popup|settings`). The backend picks the message and resolves the sprite pack, so the popup is a pure renderer.

**Tech Stack:** Rust stable (edition 2021), Tauri 2, Leptos 0.8 `csr` (use the version `create-tauri-app`'s Leptos template pins), trunk, tauri-cli via cargo, `tauri-plugin-autostart` 2, `fastrand`, `unicode-segmentation`, `percent-encoding`, `tempfile` (tests).

**Spec:** `docs/superpowers/specs/2026-11-22-tospu-phase2-design.md` (read it first; it has the UI copy and the reasons). Phase 1 spec: `docs/superpowers/specs/2026-11-21-yub-companion-design.md`.

## Global Constraints

- Product name `Tospu`. Crates `tospu-model`, `tospu-core`, `tospu` (lib `tospu_lib`), `tospu-ui`. Identifier `io.github.worty76.tospu`. Legacy identifier `dev.yub.companion`.
- Event names: `tospu://show`, `tospu://data-changed`. Global shortcut stays `CmdOrCtrl+Shift+Y`. Quiet text stays `Keep going.`. Fallback line stays `You're doing better than you think.`.
- Dev server port **1430** (`devUrl: http://localhost:1430`).
- No Node, npm, `package.json` or hand-written `.js`/`.ts` after Task 5. Tooling: `rustup target add wasm32-unknown-unknown`, `cargo install trunk --locked`, `cargo install tauri-cli --version "^2" --locked`, then run with `cargo tauri dev`.
- `tospu-model` depends only on `serde`, `serde_json` and `unicode-segmentation`. No `std::fs`, no chrono, nothing that fails on `wasm32-unknown-unknown`.
- JSON on disk and on the wire uses `camelCase` keys (`#[serde(rename_all = "camelCase")]`). Tauri command argument keys are camelCase on the JS side (Tauri's default).
- All writes to `settings.json` and `content.json` go through `tospu_core::fsutil::atomic_write` (temp file in the same folder, then rename).
- A file that fails to parse is **never** overwritten automatically.
- Limits: message text 1–280 characters after trim (count `chars()`), author ≤ 80 characters, ≤ 500 messages, ≤ 12 schedule slots, `displaySeconds` 3–60, `jitterMinutes` 0–30.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo clippy -p tospu-ui --target wasm32-unknown-unknown -- -D warnings` are clean at the end of every task. `cargo fmt` applied.
- The app is runnable at the end of Task 1 and from Task 5 on. Tasks 2–4 are library work verified by `cargo test`. The old TS popup may misbehave in between, and that's expected.
- Commit messages end with the attribution lines your harness requires.

## Review Focus

1. **Hand-edited or broken `content.json` / `settings.json`:** the app starts with an empty list or defaults, leaves the file byte-for-byte untouched, lists the problem in Settings → General, and refuses message saves until Reload. Tests: Task 2 (settings), Task 3 (content).
2. **Vietnamese and emoji text:** the typewriter reveals whole graphemes (never splits `a` + combining hook), and the 280 limit counts characters, not bytes. Tests: Task 3 (`280 × "ộ"` accepted), Task 4 (combining marks).
3. **Legacy Yub data:** copied once on the first Tospu launch, and never copied over existing Tospu data. Tests: Task 1.
4. **Schedule edits apply without restart:** `save_settings` bumps the scheduler generation. Test: Task 6.
5. **Everything deleted or disabled, or the shown message deleted mid-popup:** the picker returns the fallback line, and the popup keeps showing its own copy of the text. Tests: Task 3 (picker). The payload carries the text by design (Task 5).

---

### Task 1: Rename to Tospu and migrate legacy data

**Files:**
- Move: `src-tauri/crates/yub-core/` → `crates/tospu-core/` (crate name `tospu-core`)
- Create: `Cargo.toml` (repo root workspace: `members = ["crates/tospu-core", "src-tauri"]`, `resolver = "2"`); remove `[workspace]` from `src-tauri/Cargo.toml`
- Create: `crates/tospu-core/src/legacy.rs`
- Modify: `src-tauri/Cargo.toml` (package `tospu`, lib `tospu_lib`), `src-tauri/src/main.rs`, every `yub_core::` import, `src-tauri/tauri.conf.json` (`productName` "Tospu", window `title` "Tospu", `identifier` `io.github.worty76.tospu`), `src-tauri/src/tray.rs` (`TRAY_ID` "tospu", tooltip "Tospu", "Quit Tospu"), `src-tauri/src/popup.rs` + `src/ipc/contract.ts` (events `tospu://…`), `src-tauri/src/lib.rs` (expect message), `src-tauri/src/paths.rs`, `package.json` name `tospu`, `console` prefixes `[tospu]`, README and docs (`git mv` the phase-1 spec/plan only if you want; their content may keep "Yub" as history)
- Test: `crates/tospu-core/src/legacy.rs` (unit tests)

**Interfaces:**
- Produces: `tospu_core::legacy::migrate_legacy_data(old_dir: &Path, new_dir: &Path) -> std::io::Result<bool>`, and `Paths::resolve` calls it with `old_dir = new_dir.parent()/"dev.yub.companion"`.

- [ ] **Step 1: Write the failing tests** in `legacy.rs`:
```rust
#[test] fn copies_settings_content_and_sprites_when_new_dir_is_fresh() {
    // old has settings.json, content.json, sprites/fox/manifest.json; new dir does not exist
    assert!(migrate_legacy_data(&old, &new).unwrap());
    assert_eq!(read(new.join("settings.json")), read(old.join("settings.json")));
    assert!(new.join("sprites/fox/manifest.json").is_file());
    assert!(old.join("settings.json").is_file()); // old kept as backup
}
#[test] fn does_nothing_when_new_dir_already_has_settings() {
    // new/settings.json = "{\"activePack\":\"new\"}"
    assert!(!migrate_legacy_data(&old, &new).unwrap());
    assert_eq!(read(new.join("settings.json")), "{\"activePack\":\"new\"}");
}
#[test] fn does_nothing_when_old_dir_is_missing() { assert!(!migrate_legacy_data(&missing, &new).unwrap()); }
```
- [ ] **Step 2: Run** `cargo test -p tospu-core legacy`. Expected: FAIL (not defined).
- [ ] **Step 3: Implement `migrate_legacy_data`**. It returns `Ok(false)` if `old_dir` is missing or `new_dir/settings.json` exists. Otherwise it creates `new_dir`, copies `settings.json` and `content.json` if present, recursively copies `sprites/`, and returns `Ok(true)`. Call it from `Paths::resolve` before `ensure()`. Log a failure with `eprintln!` and continue.
- [ ] **Step 4: Do the rename** listed in Files. Then confirm `grep -rni "yub" --exclude-dir={target,node_modules,.git} . | grep -v "docs/superpowers/.*2026-11-21"` shows only the legacy identifier string and the event-migration comment.
- [ ] **Step 5: Verify** with `cargo test --workspace`, `npm test`, `npm run typecheck` (all PASS), then `npm run tauri dev`: the tray shows "Tospu", "Show one now" works, and on the Mac the old goals appear (migrated).
- [ ] **Step 6: Commit** `refactor: rename Yub to Tospu and migrate legacy data folder`.

---

### Task 2: `tospu-model` crate: settings, manifest, atomic writes

**Files:**
- Create: `crates/tospu-model/{Cargo.toml, src/lib.rs, src/kind.rs, src/time.rs, src/settings.rs, src/manifest.rs}`; add to workspace members
- Create: `crates/tospu-core/src/fsutil.rs`, `crates/tospu-core/src/settings_store.rs`
- Modify: `crates/tospu-core/src/settings.rs` (delete; types move to model, IO moves to `settings_store.rs` with its tests), `crates/tospu-core/src/packs.rs`, `crates/tospu-core/src/scheduler.rs` (imports), `crates/tospu-core/src/lib.rs` (`pub use tospu_model as model;`), `src-tauri/src/state.rs` (`Snapshot` keeps working: `manifest_json = serde_json::to_string(&pack.manifest)`)
- Test: unit tests in each new module

**Interfaces:**
- Produces (model):
  - `enum MessageKind { Goal, Quote, Gratitude, Proud }` (serde lowercase, `Hash`, `Copy`) with `pub const ALL: [MessageKind; 4]`
  - `struct HhMm` (moved unchanged)
  - `struct Slot { time: HhMm, kind: MessageKind }`, `struct QuietHours { start, end }` with `contains`
  - `struct Settings { slots, jitter_minutes, quiet_hours: Option<QuietHours>, quiet_mode, active_pack: String, display_seconds, privacy_blur: bool /*default true*/, onboarded: bool /*default false*/ }` with `#[serde(rename_all="camelCase", default)]`, and `fn validate(&self) -> Result<(), Vec<String>>`
  - `enum AnimationName { Idle, Enter, Cheer, Exit }` (serde lowercase), `enum RendererKind { #[serde(rename="spritesheet")] SpriteSheet }`
  - `struct AnimationSpec { sheet: String, frames: u32, fps: f32, #[serde(default)] r#loop: bool }` (JSON key `loop`)
  - `struct Manifest { schema_version: u32, id, name, author: Option<String>, license: Option<String>, renderer: RendererKind, frame: Frame { width: f32, height: f32, display_height: f32 /*default 120*/ }, pixelated: bool, animations: Animations { idle: AnimationSpec, enter/cheer/exit: Option<AnimationSpec> }, moods: HashMap<MessageKind, AnimationName>, bubble: Bubble { anchor: Anchor /*Top|Left|Right, default Top*/ } }`
  - `Manifest::parse(json: &str) -> Result<Manifest, String>`, `fn resolve_animation(&self, AnimationName) -> &AnimationSpec`, `fn animation_for_kind(&self, MessageKind) -> AnimationName`, `fn display_size(&self) -> (f32 /*w*/, f32 /*h*/)` where `w = round(width/height * display_height)`
- Produces (core): `fsutil::atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()>`; `settings_store::{load_or_create(path) -> (Settings, Option<String>), save(path, &Settings) -> io::Result<()>}`; `packs::{Pack { id, dir: PathBuf, manifest: Manifest }, BrokenPack { id, reason: String }, PackScan { packs: Vec<Pack>, broken: Vec<BrokenPack>, errors: Vec<String> }}` with `PackScan::choose(&self, active: &str) -> Option<&Pack>` (active, else first valid)

- [ ] **Step 1: Write the failing tests.**
  - `settings.rs`: `validate_accepts_defaults`; `validate_rejects` checks each of these returns an `Err` whose messages mention the field: 13 slots → `"slots"`; two slots both `"09:00"` → `"09:00"`; `display_seconds = 2` and `61` → `"displaySeconds"`; `jitter_minutes = 31` → `"jitterMinutes"`; quiet hours `22:00–22:00` → `"quietHours"`; `active_pack = ""` → `"activePack"`. Plus `old_file_without_new_fields_gets_privacy_blur_true_and_onboarded_false`.
  - `manifest.rs` (ports `tests/manifest.test.ts`): `accepts_minimal_pack_with_only_idle`; `accepts_bundled_mochi_manifest` (`include_str!("../../../sprites/mochi/manifest.json")`); `rejects_missing_idle_naming_the_field` (`err.contains("animations.idle")`); `rejects_ids_that_are_not_folder_safe` (`"Mochi Cat"` → `err.starts_with("id:")`); `rejects_sheet_paths_outside_the_pack` (`"../x.png"` → `err.contains("sheet")`); `rejects_unknown_renderer` (`"lottie"` → `err.contains("renderer")`); `falls_back_to_idle_when_animation_missing`; `mood_map_defaults_to_idle`; `display_size_keeps_aspect` (198×170 @120 → `(140.0, 120.0)`).
  - `fsutil.rs`: `atomic_write_replaces_content_and_leaves_no_temp_file`.
  - `packs.rs`: keep the existing discovery tests, adapted, plus `invalid_manifest_goes_to_broken_with_reason`, `manifest_id_must_match_folder`, `choose_falls_back_to_first_valid_pack`.
- [ ] **Step 2: Run** `cargo test -p tospu-model -p tospu-core`. Expected: FAIL (compile errors for missing items).
- [ ] **Step 3: Implement** the items listed in Interfaces. Error strings use the form `"<camelCase.path>: <reason>"`. Use hand-written checks: id `^[a-z0-9][a-z0-9-]*$`; sheet `^[A-Za-z0-9_.-]+\.(png|webp|gif)$` case-insensitive with no `..`; `schemaVersion == 1`; `frames > 0`; `0 < fps <= 60`; frame sizes `> 0`. Parse with serde first, then validate. Map serde's "missing field" into the same path form.
- [ ] **Step 4: Run** `cargo test --workspace` and `cargo build -p tospu-model --target wasm32-unknown-unknown`. Expected: PASS and a successful build.
- [ ] **Step 5: Commit** `feat(model): shared settings and manifest types; atomic writes; validated pack scan`.

---

### Task 3: Content v2, migration, `ContentStore`, `Picker`

**Files:**
- Create: `crates/tospu-model/src/content.rs`, `crates/tospu-core/src/content_store.rs`, `crates/tospu-core/src/picker.rs`
- Modify: `defaults/content.json` (v2 format, generic lines only: 6 quotes with authors where known, 3 proud lines; **no goals, no gratitude**; ids `"default-q1"`…, `"default-p1"`…)
- Test: unit tests in those three files

**Interfaces:**
- Produces (model `content.rs`):
  - `const MAX_TEXT_CHARS: usize = 280; const MAX_AUTHOR_CHARS: usize = 80; const MAX_MESSAGES: usize = 500;`
  - `struct MessageItem { id: String, kind: MessageKind, text: String, author: Option<String>, enabled: bool }`
  - `struct MessageDraft { kind: MessageKind, text: String, author: Option<String> }`
  - `struct ContentFile { version: u32 /*2*/, messages: Vec<MessageItem> }`
  - `enum ContentError { EmptyText, TextTooLong, AuthorTooLong, TooMany, NotFound(String), DuplicateId(String), UnsupportedVersion(u32) }` + `Display` (user-facing English, such as `"Text can be at most 280 characters."`)
  - `ContentFile::parse(json: &str, new_id: &mut dyn FnMut() -> String) -> Result<(ContentFile, bool /*was v1*/), String>`
  - `fn add(&mut self, d: MessageDraft, id: String) -> Result<&MessageItem, ContentError>`, `fn update(&mut self, item: MessageItem) -> Result<&MessageItem, ContentError>`, `fn remove(&mut self, id: &str) -> Result<MessageItem, ContentError>`, `fn validate(&self) -> Result<(), ContentError>`. `add` and `update` trim text and author, and turn an empty author into `None`.
  - `struct Message { kind: MessageKind, text: String, author: Option<String> }` (what the popup shows) and `const QUIET_TEXT: &str = "Keep going.";`
- Produces (core):
  - `content_store::ContentStore::open(path: &Path, defaults_json: &str) -> ContentStore`, `fn file(&self) -> &ContentFile`, `fn problem(&self) -> Option<&str>`, `fn change<T>(&mut self, f: impl FnOnce(&mut ContentFile) -> Result<T, ContentError>) -> Result<T, String>`, `fn reload(&mut self)`
  - `content_store::new_id() -> String` (12 lowercase hex chars from `fastrand`)
  - `picker::{FALLBACK_LINE, Picker::new(history: usize), Picker::next(&mut self, kind: MessageKind, content: &ContentFile, rand_index: &mut dyn FnMut(usize) -> usize) -> Message}`

- [ ] **Step 1: Write the failing tests.**
  - model: `migrates_v1_all_four_lists` (input `{"goals":[{"title":"A"},"B"],"quotes":["q",{"text":"t","author":"S"}],"gratitude":["g"],"proud":["p"]}` → 6 items, kinds `[Goal,Goal,Quote,Quote,Gratitude,Proud]`, texts `["A","B","q","t","g","p"]`, the item with text `t` has `author == Some("S")`, all enabled, `was_v1 == true`); `parses_v2_unchanged`; `rejects_version_3`; `add_trims_and_drops_empty_author`; `add_rejects_empty_and_whitespace_text`; `text_limit_counts_chars_not_bytes` (`"ộ".repeat(280)` ok, `281` → `TextTooLong`); `add_rejects_501st`; `update_unknown_id_is_not_found`; `remove_returns_item`; `duplicate_ids_fail_validate`.
  - core `content_store`: `missing_file_is_created_from_defaults`; `v1_file_is_migrated_and_backed_up` (`content.v1.json.bak` equals the original bytes; the new file has `"version": 2`); `broken_file_is_left_untouched_and_changes_are_refused` (bytes unchanged; `problem().is_some()`; `change(..)` returns `Err` containing `"Reload"`); `failed_change_does_not_touch_disk`; `successful_change_round_trips`.
  - core `picker` (ports `tests/content.test.ts`): `goal_and_gratitude_get_prefixes` (`"Big goal: A"`, `"Grateful for: g"`); `quotes_keep_author`; `does_not_repeat_within_history` (4 quotes, history 3, `rand_index = |_| 0`, 8 picks: no id twice within any window of 4); `falls_back_to_another_kind_in_ALL_order_when_empty`; `disabled_items_are_never_picked`; `returns_fallback_line_when_everything_is_empty_or_disabled`.
- [ ] **Step 2: Run** `cargo test -p tospu-model -p tospu-core`. Expected: FAIL.
- [ ] **Step 3: Implement.** `ContentStore::open` behaves like this:
  - missing file → write `defaults_json` (parsed and re-serialized pretty)
  - v1 → copy the original bytes to `content.v1.json.bak`, then write v2
  - parse error → empty file plus `problem = "content.json: <err>"`

  `change` refuses with `"content.json could not be read. Fix it or click Reload."` when `problem` is set. Otherwise it works on a clone: apply, `validate`, `atomic_write`, then swap in. The picker keeps a `VecDeque<id>` per kind; the history length is `min(history, pool_len - 1)`.
- [ ] **Step 4: Run** `cargo test --workspace` and `cargo build -p tospu-model --target wasm32-unknown-unknown`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(content): v2 message list with ids, v1 migration, safe store and picker`.

---

### Task 4: Pure animation logic in `tospu-model`

**Files:**
- Create: `crates/tospu-model/src/anim.rs`
- Test: unit tests in `anim.rs` (they port `tests/popup-controller.test.ts` and `tests/renderer-registry.test.ts`)

**Interfaces:**
- Produces:
  - `const TYPE_MS_PER_GRAPHEME: f64 = 22.0;`
  - `struct FrameClock { frames: u32, fps: f32, looped: bool }` with `FrameClock::from(&AnimationSpec)` and `fn at(&self, elapsed_ms: f64) -> FrameAt { frame: u32, cycles: u32, finished: bool }`
  - `fn typed(text: &str, elapsed_ms: f64) -> (&str, bool /*done*/)`, using grapheme clusters
  - `enum Phase { Entering, Typing, Showing, Exiting, Done }`
  - `enum Effect { PlayEnter, Say { text: String, author: Option<String> }, ReportHitRect, PlayMood(AnimationName), PlayIdleLoop, HideBubble, PlayExit, Finished }`
  - `struct PopupTimeline` with `fn start(message: Message, quiet: bool, display_seconds: u32, mood: AnimationName) -> (Self, Vec<Effect>)`, `fn on_animation_done(&mut self) -> Vec<Effect>`, `fn on_typing_done(&mut self) -> Vec<Effect>`, `fn tick(&mut self, dt_ms: u32, hovered: bool) -> Vec<Effect>`, `fn on_click(&mut self) -> Vec<Effect>`, `fn phase(&self) -> Phase`, `fn is_active(&self) -> bool`

- [ ] **Step 1: Write the failing tests.**
  - `frame_clock_one_shot`: 8 frames @12 fps, not looped. `at(0).frame == 0`; `at(100.0).frame == 1`; `at(700.0)` → `frame == 7 && finished`.
  - `frame_clock_loop_counts_cycles`: 6 frames @8 fps, looped. `at(750.0)` → `frame == 0, cycles == 1, !finished`.
  - `typed_reveals_whole_graphemes`: `typed("Ca\u{0309}m", 44.0).0 == "Ca\u{0309}"`, and `typed("🙏ok", 22.0).0 == "🙏"`.
  - `typed_done_after_all_graphemes`.
  - `happy_path_effects`: the sequence is `start` → `[PlayEnter]`; `on_animation_done` → `[Say{text}]`; `on_typing_done` → `[ReportHitRect, PlayMood(Cheer)]`; `on_animation_done` → `[PlayIdleLoop]`; `tick(8000,false)` → `[HideBubble, PlayExit]`; `on_animation_done` → `[Finished]`; then `!is_active()`.
  - `quiet_shows_keep_going_until_clicked`: the first `Say.text == QUIET_TEXT`; in Showing, `on_click` → `[Say{real text}]` with the countdown reset to the full display time (`tick(7900,false)` returns no exit).
  - `click_on_revealed_bubble_exits_early`.
  - `hover_pauses_countdown`: `tick(60_000, true)` → `[]`.
  - `clicks_ignored_while_entering_or_exiting`.
  - `idle_loop_cycle_completions_are_ignored`.
- [ ] **Step 2: Run** `cargo test -p tospu-model anim`. Expected: FAIL.
- [ ] **Step 3: Implement** the timeline as a plain state machine. It holds the message, `quiet_revealed`, `remaining_ms`, `display_ms = max(1, display_seconds) * 1000`, and a `mood_played` flag. No clocks inside; callers pass `dt_ms`.
- [ ] **Step 4: Run** `cargo test -p tospu-model` and the wasm build. Expected: PASS.
- [ ] **Step 5: Commit** `feat(model): frame clock, grapheme typewriter and popup timeline`.

---

### Task 5: Leptos popup, backend show payload, remove Node

**Files:**
- Create: `ui/{Cargo.toml, Trunk.toml, index.html, styles/popup.css (moved from src/popup/styles.css), src/main.rs, src/ipc.rs, src/popup/{mod.rs, sprite.rs, bubble.rs}}`; add `ui` to workspace `members`, and set `default-members = ["crates/tospu-model","crates/tospu-core","src-tauri"]`
- Create: `crates/tospu-model/src/wire.rs`, `src-tauri/src/assets.rs`, `src-tauri/capabilities/popup.json` (replaces `default.json`)
- Modify: `src-tauri/src/{popup.rs, commands.rs, state.rs, lib.rs}`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `.gitignore` (`ui/dist`, `target`)
- Delete: `package.json`, `package-lock.json`, `node_modules/`, `tsconfig.json`, `vite.config.ts`, root `index.html`, `src/`, `tests/`

**Interfaces:**
- Consumes: `Manifest`, `PackScan::choose`, `ContentStore`, `Picker`, `PopupTimeline`, `FrameClock`, `typed`.
- Produces (model `wire.rs`):
  - `struct PackView { id: String, manifest: Manifest, sheet_urls: BTreeMap<String, String> }`
  - `struct ShowPayload { message: Message, quiet: bool, display_seconds: u32, pack: Option<PackView> }`
  - `struct Rect { x: f64, y: f64, width: f64, height: f64 }` with `contains` (moved from `state.rs`)
- Produces (shell):
  - `assets::asset_url(path: &Path) -> String`. On Windows: `http://asset.localhost/<enc>`; elsewhere: `asset://localhost/<enc>`. `<enc>` is percent-encoded like JS `encodeURIComponent`.
  - `AppState` now owns `ContentStore` and `Picker`, with `fn pick(&self, kind) -> Message` and `fn pack_view(&self) -> Option<PackView>`. `Snapshot` and `get_snapshot` are deleted.
  - Popup commands: `set_hit_rect(rect: Option<Rect>)` and `popup_finished()` only.
  - `popup::show(app, kind)` emits `tospu://show` with a `ShowPayload` to window `popup`.
- Produces (ui): `ipc::invoke<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Result<R, String>` and `ipc::listen<T: DeserializeOwned>(event: &str, f: impl Fn(T) + 'static)`, through `wasm_bindgen` externs on `window.__TAURI__.core.invoke` and `window.__TAURI__.event.listen` (as in the create-tauri-app Leptos template).

- [ ] **Step 1: Write the failing tests.**
  - `assets.rs`: `encodes_like_encode_uri_component` (`#[cfg(not(windows))]` `asset_url(Path::new("/a b/ộ.png")) == "asset://localhost/%2Fa%20b%2F%E1%BB%99.png"`).
  - `state.rs` (tempdir `Paths`): `pack_view_has_a_url_for_every_sheet`; `pick_returns_fallback_with_empty_content`.
- [ ] **Step 2: Run** `cargo test -p tospu`. Expected: FAIL.
- [ ] **Step 3: Backend.**
  - Implement the items in Interfaces.
  - `build.rs`: `tauri_build::try_build(Attributes::new().app_manifest(AppManifest::new().commands(&["set_hit_rect","popup_finished"])))`. Task 6 extends this list.
  - `capabilities/popup.json`: `windows: ["popup"]`, `permissions: ["core:event:allow-listen","core:event:allow-unlisten","allow-set-hit-rect","allow-popup-finished"]`.
  - `tauri.conf.json`:
    - `app.withGlobalTauri: true`
    - popup window `url: "index.html?view=popup"`
    - `build.beforeDevCommand: "trunk serve --config ui/Trunk.toml"`
    - `beforeBuildCommand: "trunk build --release --config ui/Trunk.toml"`
    - `devUrl: "http://localhost:1430"`
    - `frontendDist: "../ui/dist"`
    - CSP: `"default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; style-src 'self' 'unsafe-inline'"`

    Check which folder Tauri runs before-commands from. If trunk can't find its config, switch to the object form `{ "script": "trunk serve", "cwd": "../ui" }`.
  - `Trunk.toml`: `[build] target = "index.html"`, `dist = "dist"`; `[serve] port = 1430`.
- [ ] **Step 4: Popup UI.**
  - `main.rs` reads `?view=` and mounts `PopupApp` (and, from Task 7, `SettingsApp`).
  - `PopupApp` listens for `tospu://show`. It ignores the payload while a timeline `is_active()`.
  - It runs a `request_animation_frame` loop that computes `dt` and feeds `tick(dt, hovered)` (hovered comes from `mouseenter`/`mouseleave` on the stage). It executes each `Effect`:
    - Play* sets the sprite's animation and start time. The `Sprite` component calls back on `FrameAt.finished`, or on each new cycle for loops.
    - `Say` restarts the `Bubble` typewriter, which calls back when done.
    - `ReportHitRect` unions the bubble and sprite `getBoundingClientRect()` and invokes `set_hit_rect`.
    - `Finished` invokes `set_hit_rect(None)` then `popup_finished`.
  - Reduced motion (`matchMedia("(prefers-reduced-motion: reduce)")`): animations report done immediately and show idle frame 0, and text appears instantly.
  - Sprite: a `div` sized by `display_size`, with `background-image: url(sheet_urls[spec.sheet])`, `background-size: {w*frames}px {h}px`, `background-position: -{frame*w}px 0`, and `image-rendering: pixelated` if set.
  - Keep phase 1's DOM classes so `popup.css` works unchanged: `stage`, `visible`, `bubble`, `bubble-text`, `bubble-author`, `sprite-host`, `sprite`. The bubble keeps `role="status"`.
- [ ] **Step 5: Verify.**
  - `cargo test --workspace` and both clippy commands pass.
  - `trunk build --config ui/Trunk.toml` succeeds.
  - `git ls-files | grep -E '\.(ts|js)$|package.json'` prints nothing.
- [ ] **Step 6: CHECKPOINT (manual, on the Mac).** Run `cargo tauri dev` and use "Show one now" four times. Check that:
  - Mochi enters, the text types, the bubble stays above full-screen apps and on every Space, it never takes focus, and clicks pass through outside the sprite and bubble.
  - Hover pauses the countdown, quiet mode shows "Keep going." until clicked, and Vietnamese text types correctly.

  **Stop and report if the transparent window or click-through regressed.** Don't continue to Task 6.
- [ ] **Step 7: Commit** `feat(ui)!: popup rewritten in Leptos; backend picks message and pack; drop Node toolchain`.

---

### Task 6: Settings backend: commands, window, tray, autostart

**Files:**
- Create: `src-tauri/src/settings_window.rs`, `src-tauri/capabilities/settings.json`, `crates/tospu-core/src/describe.rs`
- Modify: `crates/tospu-model/src/wire.rs`, `src-tauri/src/{state.rs, commands.rs, tray.rs, lib.rs}`, `src-tauri/build.rs` (add the commands), `src-tauri/Cargo.toml` (`tauri-plugin-autostart = "2"`)

**Interfaces:**
- Produces (model `wire.rs`):
  - `struct PackSummary { id: String, name: String, author: Option<String>, view: Option<PackView>, error: Option<String> }`
  - `struct SettingsView { settings: Settings, messages: Vec<MessageItem>, packs: Vec<PackSummary>, problems: Vec<String>, launch_at_login: bool, next_popup: Option<String>, shortcut_label: String, version: String, content_writable: bool }`
  - `struct OnboardingInput { goal: Option<String>, gratitude: Option<String>, proud: Option<String>, active_pack: String, launch_at_login: bool }`
- Produces (core): `describe::describe_next(now: NaiveDateTime, at: NaiveDateTime) -> String`, returning `"Today 14:07"`, `"Tomorrow 09:03"`, or `"Mon 09:00"` within 7 days.
- Produces (shell):
  - `AppState::update_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings, Vec<String>>` validates on a clone before saving, and bumps the generation only on success.
  - `AppState::change_content<T>(…) -> Result<T, String>`, `AppState::settings_view(&self, launch_at_login: bool) -> SettingsView`, `AppState::reload(&self)`.
  - Commands (all return `Result<_, String>` except `save_settings`, which returns `Result<Settings, Vec<String>>`): `get_settings_view() -> SettingsView`, `add_message(draft: MessageDraft) -> MessageItem`, `update_message(item: MessageItem) -> MessageItem`, `delete_message(id: String) -> MessageItem`, `save_settings(settings: Settings) -> Settings`, `set_launch_at_login(enabled: bool) -> bool`, `show_test_popup()`, `open_data_folder()`, `open_sprites_folder()`, `open_pack_guide()` (opens `https://github.com/worty76/tospu/blob/main/docs/creating-a-sprite-pack.md`), `reload_files() -> SettingsView`, `finish_onboarding(input: OnboardingInput)`.
  - After any change, including tray quiet/character changes and reload, the backend emits `tospu://data-changed` (no payload) to window `settings`.
  - `settings_window::open(app: &AppHandle)`:
    - If the window already exists, `show()` + `set_focus()`.
    - Otherwise build `WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html?view=settings".into()))` with title `"Tospu Settings"`, size 760×560, min 640×480, centered, focused.
    - On macOS, set `ActivationPolicy::Regular` before showing, and set it back to `Accessory` on `WindowEvent::Destroyed`.

- [ ] **Step 1: Write the failing tests.**
  - `describe.rs`: `today_tomorrow_and_weekday`.
  - `state.rs` (tempdir):
    - `update_settings_rejects_invalid_and_keeps_disk` (`display_seconds = 1` → `Err`; the file is unchanged; the generation is unchanged).
    - `update_settings_bumps_generation`.
    - `change_content_persists_and_view_reflects_it`.
    - `settings_view_lists_broken_packs_with_reason`.
    - `onboarding_adds_non_empty_lines_only_and_sets_onboarded` (via a pure helper `apply_onboarding(&AppState, &OnboardingInput) -> Result<(), String>`; the autostart call stays in the command).
- [ ] **Step 2: Run** `cargo test -p tospu-core -p tospu`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - Register `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)`. Read and toggle it with `app.autolaunch()`.
  - `capabilities/settings.json`: `windows: ["settings"]`, `permissions: ["core:default", "allow-<each settings command>"]`.
  - The tray menu becomes: Show one now · Quiet mode · Character ▸ · ─ · `Settings…` (id `settings`) · ─ · Quit Tospu. Remove the JSON-editing items and Reload.
  - In `setup`, if `!settings.onboarded`, call `settings_window::open`.
  - `finish_onboarding` calls `apply_onboarding`, sets autostart, closes the settings window, then calls `popup::show(app, MessageKind::Goal)`.
- [ ] **Step 4: Verify.** `cargo test --workspace` and clippy pass. In `cargo tauri dev`, the tray item "Settings…" opens one window (clicking it twice focuses the same window), and the window appears in Cmd+Tab while open. The window is blank until Task 7.
- [ ] **Step 5: Commit** `feat(settings): backend commands, settings window, autostart, simplified tray`.

---

### Task 7: Settings UI shell and Messages tab

**Files:**
- Create: `ui/src/settings/{mod.rs, store.rs, messages.rs}`, `ui/styles/settings.css`
- Modify: `ui/src/main.rs`, `ui/index.html` (link `settings.css`)

**Interfaces:**
- Consumes: the Task 6 commands and event, plus `MessageItem`, `MessageDraft`, `MAX_TEXT_CHARS`.
- Produces: `settings::store::Store` (provided via context) with `view: RwSignal<Option<SettingsView>>`, `error: RwSignal<Option<String>>`, `fn refresh()`, and `fn run<R>(fut)`. `run` executes a command, puts an `Err` into `error` (shown as a dismissible banner) and refreshes on success. The store subscribes to `tospu://data-changed` and calls `refresh()`. `SettingsApp` renders the tab bar (Messages, Schedule, Character, General) with tab state in a signal. Task 9 adds Welcome when `!settings.onboarded`.

- [ ] **Step 1: Build the shell.** Use a left tab column on wide windows that becomes a top row under 700 px, follow `prefers-color-scheme`, use system font, and give the tabs keyboard focus styles. Use real `<button>`s and `role="tablist"`/`"tab"` with `aria-selected`.
- [ ] **Step 2: Build the Messages tab** as the spec describes:
  - **Filter chips:** All, Goals, Quotes, Gratitude, Pride.
  - **Add form:**
    - Kind `<select>`, defaulting to the active filter.
    - `<textarea maxlength=280>` with a live `n/280` counter.
    - Author input shown only for Quote.
    - Add button. Cmd/Ctrl+Enter submits.
    - Clear the form on success.
  - **Rows:**
    - An on/off switch that calls `update_message` with `enabled` flipped.
    - The text, with class `blur` when `settings.privacyBlur`. The CSS is `filter: blur(5px)`, removed on `:hover` and `:focus-within`.
    - Edit swaps the row to inline inputs with Save/Cancel (`update_message`).
    - Delete calls `delete_message` and shows a toast "Deleted · Undo" for 6 s. Undo calls `add_message` with the returned item's kind, text and author.
  - **Empty state per filter:** e.g. "No goals yet. What's one big thing you're working toward?"
  - If `!view.content_writable`, disable all inputs and show the problem text with a "Reload files" button.
- [ ] **Step 3: Verify.**
  - `trunk build --config ui/Trunk.toml` and the wasm clippy pass.
  - **Manual:** in `cargo tauri dev`, add one of each kind, including a Vietnamese line (`"Cảm ơn gia đình"`), edit it, disable it, delete it and undo.
  - Check that `content.json` reflects each change, that "Show one now" never shows a disabled line, and that the blur hides text until hover.
- [ ] **Step 4: Commit** `feat(ui): settings window shell and messages manager`.

---

### Task 8: Schedule, Character and General tabs

**Files:**
- Create: `ui/src/settings/{schedule.rs, character.rs, general.rs}`
- Modify: `ui/src/settings/mod.rs`, `ui/styles/settings.css`, `ui/src/popup/sprite.rs` (export a `SpriteStill` component that shows a paused idle frame 0 for a `PackView`)

**Interfaces:**
- Consumes: `save_settings`, `set_launch_at_login`, `show_test_popup`, `open_*`, `reload_files`, and `SettingsView` fields.

- [ ] **Step 1: Build the Schedule tab.** Each control saves on change through `save_settings`, sending the whole `Settings` with one field changed. Show the returned `Vec<String>` errors inline next to the matching control (match on the camelCase field name in the message) and roll the control back.
  - **Slots:** a list of `<input type=time>` plus a kind `<select>` and a remove button. "Add time" is disabled at 12 slots and adds the next free hour.
  - **Random shift:** a slider 0–30 ("±N min").
  - **Quiet hours:** a toggle plus start and end time inputs.
  - **Display time:** a number input 3–60 s.
  - Show `next_popup` as "Next popup: Today 14:07" ("No popups scheduled" if `None`).
- [ ] **Step 2: Build the Character tab.**
  - A grid of `PackSummary` cards: a `SpriteStill`, the name, "by {author}", and a selected ring for `active_pack`. Clicking a card saves `active_pack`.
  - Broken packs are greyed out with the `error` text and aren't clickable.
  - Buttons: "Open sprites folder" and "How to make your own".
- [ ] **Step 3: Build the General tab.**
  - Toggles: Launch at login (`set_launch_at_login`), Quiet mode (with `shortcut_label`), and Hide message text until hover (`privacyBlur`).
  - Buttons: Show a test popup, Open data folder, and Reload files.
  - A "Problems" list from `problems`, hidden when empty, and a "Tospu {version}" footer.
- [ ] **Step 4: Verify (manual).**
  - Change a slot to 2 minutes from now and confirm a popup arrives without a restart.
  - Try a duplicate time and confirm the inline error shows and the value rolls back.
  - Switch character from the tab and confirm the tray check mark follows; switch from the tray and confirm the tab follows.
  - Toggle launch at login, then confirm it in System Settings → General → Login Items.
  - Break `content.json` by hand, click Reload, and confirm the problem shows and Messages is read-only.
- [ ] **Step 5: Commit** `feat(ui): schedule, character and general settings`.

---

### Task 9: First-run welcome

**Files:**
- Create: `ui/src/settings/welcome.rs`
- Modify: `ui/src/settings/mod.rs` (render `Welcome` instead of the tabs when `!view.settings.onboarded`)

**Interfaces:**
- Consumes: `finish_onboarding(input: OnboardingInput)`, `PackSummary`, `SpriteStill`.

- [ ] **Step 1: Build the Welcome view.**
  - Title "Hi, I'm Tospu." with the line "I'll pop up a few times a day with your big goals and a little encouragement, then get out of your way."
  - Three optional inputs: "One big goal", "Something you're grateful for", "Something you're proud of". Each has `maxlength=280`.
  - A character picker built from the same cards as the Character tab.
  - "Start Tospu when I log in" (checked).
  - A **Start** button that invokes `finish_onboarding`. On error, show it and stay on the view.
- [ ] **Step 2: Verify (manual).**
  - Quit, set `"onboarded": false` in `settings.json`, and relaunch. The welcome opens.
  - Fill in only the goal and click Start. The window closes, a goal popup shows your goal, `content.json` has one new goal, and nothing was added for the empty fields.
  - Relaunch and confirm no welcome appears.
  - With a fresh data folder (rename it temporarily), confirm defaults hold only generic quotes and proud lines.
- [ ] **Step 3: Commit** `feat(ui): first-run welcome`.

---

### Task 10: Docs and CI

**Files:**
- Modify: `README.md` (what Tospu is, a screenshot placeholder, prerequisites from Global Constraints, `cargo tauri dev`, `cargo tauri build`, where data lives per OS, privacy notes), `docs/creating-a-sprite-pack.md` (validation is now in Rust; the error messages list matches Task 2)
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the CI workflow** on `macos-latest`, triggered on push and PR:
  - Checkout, then `dtolnay/rust-toolchain@stable` with `targets: wasm32-unknown-unknown` and `components: clippy, rustfmt`.
  - `Swatinem/rust-cache@v2` and `jetli/trunk-action@v0.5`.
  - `trunk build --config ui/Trunk.toml`, because `generate_context!` needs `ui/dist`.
  - `cargo fmt --all --check`.
  - `cargo clippy --workspace --all-targets -- -D warnings`.
  - `cargo clippy -p tospu-ui --target wasm32-unknown-unknown -- -D warnings`.
  - `cargo test --workspace`.
- [ ] **Step 2: Verify** that the workflow is valid YAML and that the same commands pass locally.
- [ ] **Step 3: Commit** `docs: README for Tospu; ci: build, lint and test on macOS`.

---

## After this plan (not in scope)

macOS signing and notarization plus a GitHub Release workflow, Windows/Linux checks, starter quote packs, import/export, a configurable shortcut, and full-screen-app detection.
