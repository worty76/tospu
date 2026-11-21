# Yub Companion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A macOS menu-bar app that pops up an animated sprite with a goal/quote/gratitude/proud message on a schedule, with swappable sprite packs and a quiet mode.

**Architecture:** Tauri 2. A pure Rust core crate (`yub-core`: scheduler, settings, pack discovery) is wrapped by a thin Tauri shell (tray, popup window behind a `PopupWindow` trait, commands). A vanilla TypeScript front end handles manifest validation, renderer registry, content picking and the popup lifecycle.

**Tech Stack:** Tauri 2, Rust (chrono, serde), TypeScript + Vite, zod, Vitest, Python 3 + Pillow (art tooling).

**Spec:** `docs/superpowers/specs/2026-11-21-yub-companion-design.md`

## Global Constraints
- macOS first; no OS-specific code outside `src-tauri/src/popup_window/`.
- Adding a character = a folder with `manifest.json` + PNGs. No code edits.
- A broken pack or broken content file never crashes the app; it is skipped, and the reason is reported.
- Popup never takes keyboard focus; clicks pass through except over the sprite/bubble.
- Quiet mode bubble text is exactly `Keep going.`; the real text appears only on click.
- Defaults: slots `09:00 goal`, `14:00 quote`, `21:00 gratitude`; jitter ±10 min; quiet hours 23:00–07:30; display 8 s; active pack `mochi`.
- All user data lives in the app data dir as plain JSON.

## Review Focus
1. **Manifest missing optional animations** → falls back to `idle` (test in Task 2).
2. **Quiet hours crossing midnight (23:00–07:30)** → slots inside are skipped across the day boundary (test in Task 4).
3. **Empty content category** (e.g. no `proud` entries) → falls back to any non-empty kind, then to a built-in line, never throws (test in Task 3).
4. **Pack ids on disk colliding** between bundled and user folders → user pack wins, deterministically (test in Task 4).
5. **Generated sheet with characters straddling equal-width cell borders** → the slicer warns with the strip and border index (test in Task 1).

---

### Task 1: Sprite slicer + bundled packs
**Files:** Create `tools/slice_sprites.py`, `tools/tests/test_slice_sprites.py`, output `sprites/<id>/{manifest.json,idle.png,wave.png,cheer.png,sleep.png,preview.png}`.
**Produces:** packs following the manifest schema in Task 2 (`renderer: "spritesheet"`, horizontal strips, one shared frame size per pack).
- [ ] Tests: `find_strips` on a synthetic image with 4 magenta bands returns 4 `(y0,y1)` ranges; `key_out` makes `(255,0,255)` and dark purple shadow `(120,20,120)` transparent and keeps cream `(250,235,215)`, blush `(240,160,150)`, outline `(60,30,20)`; `cell_border_warnings` flags a strip where foreground crosses a border.
- [ ] Implement: detect strips (rows where >15% pixels magenta, height >40), split each into N equal cells (default 6,8,8,6), key out magenta, re-centre each frame horizontally on its largest component, align strips on a shared baseline (median body bottom per strip), write strips + manifest.
- [ ] Run on the three source sheets → `mochi`, `mochi-bounce`, `mochi-fluffy`. Visually inspect a contact sheet.
- [ ] Commit.

### Task 2: Domain types, manifest schema, renderer registry (TS)
**Files:** `src/domain/types.ts`, `src/sprites/manifest.ts`, `src/sprites/renderer.ts`, `src/sprites/spritesheet-renderer.ts`, `tests/manifest.test.ts`, `tests/renderer-registry.test.ts`.
**Produces:**
- `type MessageKind = "goal" | "quote" | "gratitude" | "proud"`; `interface Message { kind; text; author?: string }`.
- `parseManifest(json: unknown): { ok: true; manifest: SpriteManifest } | { ok: false; error: string }`.
- `resolveAnimation(m, name: AnimationName): AnimationSpec` (falls back to `idle`), `animationForKind(m, kind): AnimationName`.
- `interface SpriteRenderer { mount(el: HTMLElement): void; play(name: AnimationName): Promise<void>; stop(): void; destroy(): void }`, `registerRenderer(kind: string, factory)`, `createRenderer(pack: LoadedPack): SpriteRenderer` (throws `UnknownRendererError`).
- [ ] Tests: a valid manifest parses; one missing `idle` fails with a message naming `animations.idle`; `resolveAnimation(m,"cheer")` returns idle when cheer is absent; unknown mood → `idle`; `createRenderer` with `renderer:"lottie"` throws `UnknownRendererError`.
- [ ] Implement; the spritesheet renderer steps `background-position` using a timer, and resolves `play()` when a non-looping animation ends (looping ones resolve after one cycle).
- [ ] `npx vitest run` passes. Commit.

### Task 3: Content schema + picker (TS)
**Files:** `src/content/schema.ts`, `src/content/content-source.ts`, `tests/content.test.ts`.
**Produces:** `parseContent(json): Content` (invalid → `DEFAULT_CONTENT` + error), `interface ContentSource { next(kind: MessageKind): Message }`, `class LocalContentSource(content, { historySize = 3, random = Math.random })`.
- [ ] Tests: no immediate repeat when ≥2 items exist (seeded random); empty kind falls back to another non-empty kind; all empty → `"You're doing better than you think."`; goals render as `"Big goal: <title>"`.
- [ ] Implement, run, commit.

### Task 4: Rust core crate (`yub-core`)
**Files:** `src-tauri/crates/yub-core/src/{lib.rs,settings.rs,scheduler.rs,packs.rs}`.
**Produces:** `Settings` (serde, defaults above, camelCase), `Slot { time: HhMm, kind: MessageKind }`, `next_fire(now: NaiveDateTime, s: &Settings) -> Option<Fire { at, kind }>`, `discover_packs(roots: &[PathBuf]) -> Vec<PackEntry { id, dir, manifest_json }>`.
- [ ] Tests: next slot today; after last slot → first slot tomorrow; jitter deterministic per (date, slot) and within ±jitter; slot at 23:30 with quiet 23:00–07:30 skipped; empty slots → `None`; later root overrides an earlier pack with the same id; folder without manifest ignored.
- [ ] Implement, `cargo test -p yub-core`, commit.

### Task 5: Tauri shell
**Files:** `src-tauri/{Cargo.toml,tauri.conf.json,build.rs,capabilities/default.json}`, `src-tauri/src/{main.rs,lib.rs,state.rs,commands.rs,tray.rs,scheduler_loop.rs,popup_window/{mod.rs,macos.rs,generic.rs}}`.
**Interfaces:** commands `get_snapshot() -> Snapshot { settings, packs, contentJson, errors }`, `set_hit_rect(rect: Option<Rect>)`, `popup_finished()`; events `yub://show { kind, quiet }`, `yub://snapshot-changed`.
`trait PopupWindow { fn show_at_corner(&self); fn hide(&self); fn set_click_through(&self, on: bool); fn foreground_is_fullscreen(&self) -> bool }`.
- [ ] Tray menu: Show one now, Quiet mode ✓, Character ▸ (packs), Open content file, Open sprites folder, Reload, Quit. Global shortcut `CmdOrCtrl+Shift+Y` toggles quiet mode.
- [ ] Scheduler thread sleeps until `next_fire`, re-plans on settings change, emits `yub://show`.
- [ ] Hit-testing loop: while popup visible, every 50 ms compare the cursor with the hit rect and toggle click-through.
- [ ] `cargo check` (Linux CI may lack webkit libs; record the result). Commit.

### Task 6: Popup front end
**Files:** `index.html`, `src/main.ts`, `src/ipc/contract.ts`, `src/popup/{popup-controller.ts,bubble.ts,styles.css}`, `tests/popup-controller.test.ts`.
**Produces:** `PopupController.show(kind, quiet)` running enter → type-in → idle (paused while hovered) → exit → `popup_finished`.
- [ ] Tests (fake renderer + fake timers): play order is `enter, idle…, exit`; quiet shows `Keep going.` and reveals text on click; hovering extends display; click dismisses early.
- [ ] Implement, `npx vitest run`, `npx tsc --noEmit`, `npx vite build`. Commit.

### Task 7: Docs + verification
- [ ] `README.md` (run/build), `docs/creating-a-sprite-pack.md` (prompts, slicer usage, manifest reference).
- [ ] Full test run (vitest, cargo test core, pytest), production build of the front end. Commit.
