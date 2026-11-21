# Yub — a desktop companion for big goals & gratitude (Design)

*Date: 2026-11-21 · Status: draft for approval · "Yub" is a working name (my call, easy to change).*

## 1. Intent

A small, private macOS companion. A few times a day a cute animated sprite pops into a corner of the screen with a speech bubble. It reminds you of your **big future goals** and nudges you to be **grateful** and **proud** of how far you've come. Then it leaves on its own.

Why it exists: a goals wallpaper didn't work because everyone walking past could read it. This tool has to stay **personal and discreet** while still showing up without you doing anything.

## 2. Goals, non-goals, anti-goals

**Goals**
- Timed popups with an animated sprite and a speech bubble. The popup dismisses itself.
- Four message kinds: **Goal**, **Quote**, **Gratitude**, **Proud**.
- A **quiet mode** that shows only a neutral line ("Keep going."). The real text appears only when you click.
- **Swappable sprites.** A new character is a folder of images plus one manifest file, with no code changes. Switching characters takes one click in the menu bar.
- Clean, layered code that's easy to extend (new sprite formats, message kinds and content sources).

**Non-goals (v1)**
- Sync across devices, a mobile app or accounts.
- AI-generated messages (possible later through the content-source interface).
- Windows/Linux support in v1. All OS-specific window behaviour sits behind one Rust `PopupWindow` trait, so each platform is a new implementation: macOS first, then Windows, then Linux (X11 before Wayland), each after its own spike.

**Anti-goals (failures even if the code works)**
- Steals keyboard focus or blocks clicks on your work.
- Shows private text while someone is next to you or while you're screen sharing.
- Feels naggy: too frequent, the same message repeated, or appearing over full-screen meetings.
- Ships a copy of an existing anime character. Packs must be original or properly licensed.

## 3. Constraints
- macOS first. It must behave correctly with full-screen apps, multiple Spaces and multiple displays.
- Low resource use: it runs all day, so it should be near zero CPU when idle.
- All data stays local in plain files you can edit by hand.

## 4. Approaches considered

| Approach | Trade-off |
|---|---|
| Native Swift (SwiftUI/AppKit) | Best macOS feel, but a new language and Mac-only lock-in. |
| **Tauri 2 (Rust shell + TypeScript UI)** ✅ | Light, with good tray and transparent-window support. Animation is plain web tech. Some macOS window quirks need tuning. |
| Wails (Go + web) | Keeps your Go stack, but its tray/menu bar support is weaker. |

**Chosen: Tauri 2.** You already know TypeScript, the Rust part stays small, and sprite animation on the web is simple and flexible.

## 5. Architecture

```
┌─────────────────────── Rust (src-tauri) ───────────────────────┐
│ tray        menu bar icon + menu (quiet mode, pick sprite, ...)│
│ scheduler   decides WHEN to pop (times, jitter, quiet hours)   │
│ windows     transparent, always-on-top, non-focusing popup     │
│ storage     reads/writes config + content files (app data dir) │
│ packs       discovers sprite-pack folders, serves their assets │
└──────────────── events / commands (typed IPC) ─────────────────┘
┌────────────────────── TypeScript (src) ────────────────────────┐
│ domain      Message, MessageKind, SpritePack, Animation types  │
│ content     ContentSource interface → pick WHAT to say         │
│ sprites     SpriteRenderer interface → HOW the character moves │
│ popup       bubble + sprite stage; enter → talk → exit         │
│ panel       menu bar panel: goals, today's quote, grateful-for │
└────────────────────────────────────────────────────────────────┘
```

Rule: **the popup UI never knows which character or file format it's showing.** It asks for a *mood* ("cheer") and the active pack decides what that looks like.

### 5.1 Sprite packs (the extensibility core)

A pack is a folder:

```
sprites/
  mochi/                  # bundled default, an original character
    manifest.json
    idle.png  wave.png  cheer.png  sleep.png
    preview.png
```

`manifest.json`:
```json
{
  "schemaVersion": 1,
  "id": "mochi",
  "name": "Mochi",
  "author": "you",
  "license": "CC-BY-4.0",
  "renderer": "spritesheet",
  "frame": { "width": 64, "height": 64, "scale": 2 },
  "animations": {
    "enter": { "sheet": "wave.png",  "frames": 8, "fps": 12, "loop": false },
    "idle":  { "sheet": "idle.png",  "frames": 6, "fps": 8,  "loop": true },
    "cheer": { "sheet": "cheer.png", "frames": 8, "fps": 12, "loop": false },
    "exit":  { "sheet": "sleep.png", "frames": 6, "fps": 10, "loop": false }
  },
  "moods": {
    "goal": "cheer", "proud": "cheer", "gratitude": "idle", "quote": "idle"
  },
  "bubble": { "anchor": "top-left", "offset": [8, -12] }
}
```

- **Required animations:** `idle`. All other animations fall back to `idle`, so a minimal pack is a single sprite sheet.
- **`renderer` field** selects a `SpriteRenderer` implementation through a registry. v1 ships `spritesheet`. `gif`, `css` and `lottie` can be added later as new classes with no changes to the popup.
- **Validation:** manifests are checked against a schema (zod). A broken pack is skipped and the reason is shown in the menu. It never crashes the app.
- **Where packs live:** the bundled default plus `~/Library/Application Support/Yub/sprites/`. "Open sprites folder" and "Reload sprites" are in the menu. The menu lists packs using their `preview.png`.

### 5.2 Content
- One editable file, `content.json` (or YAML) in the app data folder, with `goals[]` (title and optional progress), `quotes[]`, `gratitude[]` and `proud[]`.
- A `ContentSource` interface (`next(kind): Message`) is implemented in v1 by `LocalFileSource`. A remote or AI source can plug in later.
- Picking avoids repeats: the app remembers the last N messages shown per kind.

### 5.3 Scheduling & discretion
- Config: a list of time slots, each with a message kind (e.g. 09:00 goal, 14:00 quote, 21:00 gratitude/proud), ±N minutes of random jitter, and quiet hours.
- **Quiet mode** can be turned on manually from the menu or with a global hotkey. In quiet mode the bubble shows "Keep going." and reveals the real text when clicked.
- **Skip or defer** when a full-screen app is in front (meetings, presentations). Detecting screen sharing automatically is a *stretch goal*, because macOS makes it unreliable.
- "Show one now" in the menu, for testing and for those moments you want one.

### 5.4 Popup behavior
- Transparent, borderless, always-on-top window in the bottom-right of the active display, shown on all Spaces.
- Never takes focus. Clicks pass through everywhere except the bubble and the sprite.
- Lifecycle: `enter` animation → bubble types in → `idle` for ~8s (paused while you hover) → `exit` → window hides. Click the bubble to dismiss early (or to reveal the text in quiet mode).
- Respects macOS "Reduce Motion" by showing a static frame and fading the bubble.

## 6. Code quality rules
- Clear layers: `domain` (pure types and logic) ← `content` / `sprites` ← `popup` / `panel`. Dependencies point inward only.
- New characters, renderers and content sources are added by registration, not by editing existing modules (open/closed).
- Typed IPC contracts shared between Rust and TypeScript, kept in one place.
- Tests: Vitest for manifest parsing, mood fallback and content picking. Rust unit tests for scheduler time math (jitter, quiet hours, day rollover).
- A `docs/creating-a-sprite-pack.md` guide, so people (or future you) can make a new character in minutes.

## 7. Decided vs left to the builder
**Decided with you:** timed popups as the core; an animated original sprite; quiet mode for privacy; macOS; Tauri; sprites must be easy to swap often.

**My calls (correct me):** working name "Yub"; default character "Mochi" (the round fluffy spirit from your mockup prompt); gentle-friend tone; JSON pack manifests; three default slots per day; vanilla TypeScript + Vite (no UI framework, since the UI is small).

**Decided 2026-11-21 (after first draft):** macOS first behind a `PopupWindow` trait; art comes from your generated sheets (reference row + 4 magenta strips: idle 6, wave 8, cheer 8, sleep 6), cut into packs by `tools/slice_sprites.py`; all three generated variants ship as packs (`mochi`, `mochi-bounce`, `mochi-fluffy`) to prove the swap works. The menu bar panel (goals/quote/grateful view) moves to v1.1; v1 menu bar is a native menu.

**Left to the builder:** exact animation timings, bubble styling, file watcher vs. manual reload, the settings UI layout for v1.1.

## 8. First risk to test
A throwaway spike in the first task: a transparent, non-focusing, click-through Tauri window that stays on top over a full-screen app and across Spaces. If that fails, we fall back to a small Swift helper for the window, and the rest of the design stays the same.
