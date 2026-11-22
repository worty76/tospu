# Tospu phase 2: rename, full-Rust UI, settings window

**Status:** agreed in conversation on 2026-11-21/22. Builds on `2026-11-21-yub-companion-design.md`.

## Intent

Tospu (formerly Yub) is a small desktop companion. A cute sprite pops up a few times a day with your big goals, a quote, something you're grateful for or something to be proud of, then leaves. Phase 1 worked, but every change meant hand-editing `content.json` and `settings.json`.

Phase 2 makes Tospu ready for **other people to install**:
- Anyone can add, edit, delete and switch off their own messages, and change the schedule, character and general options, from a normal settings window. Nobody has to open a JSON file.
- The whole app is Rust. The owner wants one language across the codebase, so the TypeScript popup and the Node/npm toolchain are removed.
- The product is renamed from Yub to Tospu everywhere, without losing existing users' data.

## Goals

1. Rename Yub to Tospu (product name, crates, identifier, events, docs). Copy the old data folder over once on first launch.
2. Replace the TypeScript/Vite front end with **Leptos (Rust → WebAssembly)** inside the existing Tauri 2 shell, built with **trunk**. The repo has no `package.json`.
3. Add a **settings window**, opened from the tray, with tabs **Messages / Schedule / Character / General**.
4. Messages: add, edit, delete and enable/disable items of all four kinds (goal, quote, gratitude, proud). Each change saves instantly.
5. Public-release basics: a first-run welcome, **launch at login**, generic default content, and a privacy blur for message text in the settings window.

## Non-goals (this phase)

Starter quote packs, import/export, cloud sync, a menu-bar dropdown panel, code signing or notarization, a configurable shortcut, full-screen-app detection, and testing on Windows/Linux. The code stays portable, as before.

## Anti-goals (failure even if it "works")

- The popup becomes heavier, slower or starts stealing focus.
- Any window shows up unless the user asked for it (except the first-run welcome).
- An upgrade loses or overwrites a user's messages or settings, including hand-edited or broken files.
- Message text is readable by someone glancing at the screen while the settings window is open and privacy blur is on.
- Node, npm or hand-written JavaScript returns to the build. The only JS is what Tauri injects (`withGlobalTauri`).

## Approaches considered

**UI technology**
| Option | Trade-off | Decision |
|---|---|---|
| Tauri + Leptos (wasm) | Keeps the proven transparent, click-through, always-on-top popup. Web views handle IME and Vietnamese input correctly. Needs trunk. | **Chosen** |
| Dioxus desktop | One framework, but tray, shortcut and window code would be rewritten, and transparent click-through windows are less proven. | Rejected |
| egui (native) | No web view, but IME composition for Vietnamese is unreliable on macOS, and tray/multi-window need workarounds. | Rejected |

**Where settings live:** a separate settings window (chosen) vs. a menu-bar dropdown (too cramped for schedule and long lists) vs. JSON files plus a quick-add box (not friendly for non-developers).

**Storage:** JSON files (chosen; tiny, readable, easy to back up, already in use) vs. SQLite (overkill for at most a few hundred short strings). Writes become atomic: write a temp file, then rename.

## Architecture

```
Cargo workspace (repo root)
├─ crates/tospu-model   serde-only types + pure logic. Builds for native AND wasm32.
│                       MessageKind, HhMm, Settings(+validate), ContentFile v2(+migrate, CRUD, validate),
│                       Manifest(+validate), wire types (ShowPayload, SettingsView…), anim logic
│                       (FrameClock, Typewriter, PopupTimeline).
├─ crates/tospu-core    native only: file IO (atomic_write), settings/content stores, pack discovery
│                       (now validates manifests), scheduler (next_fire), Picker (which message to show).
├─ src-tauri            Tauri shell: windows, tray, global shortcut, autostart, commands, events.
└─ ui                   tospu-ui: Leptos CSR app. index.html?view=popup | ?view=settings.
```

The main change: the **backend picks the message and resolves the pack**, then sends both in the show event. The popup turns into a pure renderer with only two commands (`set_hit_rect`, `popup_finished`), so the popup and settings windows need no shared front-end state.

Each window has its own **capability file**. The popup can't call settings commands, and the settings window can't call popup commands.

## Data

**content.json v2**
```json
{ "version": 2,
  "messages": [ { "id": "9f2c…", "kind": "quote", "text": "…", "author": "Seneca", "enabled": true } ] }
```
- A file without `version` is v1 (`goals/quotes/gratitude/proud`). It's migrated on load, and the original is kept as `content.v1.json.bak`.
- Limits: text 1–280 characters after trimming (it must fit the bubble), author ≤ 80 characters, at most 500 messages.
- At display time a goal is shown as `Big goal: …` and gratitude as `Grateful for: …`, the same as phase 1.
- A broken or unreadable file is never overwritten. The app runs with an empty list, shows the error in Settings → General, and refuses to save messages until the user fixes the file or clicks Reload.

**settings.json** gains `privacyBlur` (default `true`) and `onboarded` (default `false`). Launch at login is **not** stored; the OS (autostart plugin) is the source of truth.

**Default content (new installs):** generic quotes and proud lines only. Goals and gratitude start empty and are filled in by the welcome screen. The owner's personal lines are removed from `defaults/content.json`.

## Settings window

A normal decorated window (760×560, min 640×480) that follows the system light/dark theme. Only one exists at a time. Opening it again focuses the existing one. On macOS the app switches to the Regular activation policy while it's open, so it gets keyboard focus and appears in Cmd+Tab, and switches back to Accessory when it closes.

- **Messages:** filter chips (All · Goals · Quotes · Gratitude · Pride). An add form with kind, text (counter /280) and author (quotes only). A list where each row has an on/off switch, the text (blurred until hover or focus when privacy blur is on), edit inline, and delete with a 6-second "Undo". Each kind has an empty state.
- **Schedule:** daily times (time + kind, up to 12, unique times, add/remove), random shift ±0–30 min, quiet hours (toggle + start/end; start ≠ end), how long a popup stays (3–60 s), and a live "Next popup: …" line.
- **Character:** a grid of packs, each showing its idle frame, name and author. Click to select. Broken packs are greyed out with the reason. Buttons for "Open sprites folder" and "How to make your own" (the docs on GitHub).
- **General:** launch at login, quiet mode (shows the shortcut ⌘⇧Y / Ctrl+Shift+Y), privacy blur, "Show a test popup", "Open data folder", "Reload files", a Problems list (file and pack errors), and the version.

**First-run welcome** (when `onboarded` is false, the settings window opens on a Welcome view): one line about what Tospu does, three optional inputs (one big goal, one thing you're grateful for, one thing you're proud of), a character picker, a launch-at-login toggle (default on), and a **Start** button. Start saves everything, sets `onboarded = true`, shows one test popup and closes the window.

## Tray menu (after)

Show one now · Quiet mode ✓ · Character ▸ · ── · Settings… · ── · Quit Tospu

## Decisions I made (flag any you disagree with)

- App identifier `io.github.worty76.tospu`. It's a reverse-DNS name for your GitHub, since you don't own a domain yet.
- Privacy blur is on by default, launch at login is on by default in the welcome screen, and the text limit is 280 characters.
- Content picking moves from the front end to Rust (`tospu-core::Picker`).
- Each message change saves instantly, with no Save button. Delete has an Undo.
- The browser-only demo mode from phase 1 is dropped. You test on the real app with `cargo tauri dev`.
- Sprite renderers become a Rust enum (`RendererKind::SpriteSheet`). A new renderer type means a new enum variant plus a component. Unknown renderer names are rejected when the pack is validated.

## Left to the builder

The exact CSS look (it should match phase 1's bubble style, be calm and readable, and use system fonts), the minor version of Leptos (use what `create-tauri-app`'s Leptos template pins), and any copy not quoted here.
