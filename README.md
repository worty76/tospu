# Tospu

A tiny, private desktop companion. A few times a day a cute sprite hops into the corner of your screen and reminds you of your **big goals**, a **quote** to live by, something to be **grateful** for, or why you should be **proud**. Then it falls asleep and disappears.

- **Private by design.** Quiet mode (`⌘⇧Y`, or from the menu bar) shows only "Keep going."; click to reveal the real message.
- **Never in your way.** It doesn't take focus, clicks pass through everywhere except the sprite and bubble, and it stays quiet at night.
- **Swap characters any time.** A character is a folder of PNGs plus a `manifest.json`. See [docs/creating-a-sprite-pack.md](docs/creating-a-sprite-pack.md).

## Run it

Requirements: Node 20+, Rust (stable), and Xcode Command Line Tools on macOS.

```bash
npm install
npm run tauri dev       # run the app (menu bar icon appears; try "Show one now")
npm run tauri build     # build Tospu.app / .dmg into src-tauri/target/release/bundle
```

To work on sprites or styling without the app, run `npm run dev` and open http://localhost:1430/?pack=mochi-fluffy. This loops popups in the browser.

## Make it yours

From the menu bar icon:

| Menu item | Opens |
|---|---|
| Edit goals & quotes… | `content.json`: goals, quotes, gratitude, proud lines |
| Edit schedule… | `settings.json`: popup times, jitter, quiet hours, display time |
| Open sprites folder | where your own character packs go |
| Reload | re-reads everything after you edit |

Files live in `~/Library/Application Support/io.github.worty76.tospu/`. Changes to `content.json` apply on the next popup.

`settings.json` example:
```json
{
  "slots": [
    { "time": "09:00", "kind": "goal" },
    { "time": "14:00", "kind": "quote" },
    { "time": "21:00", "kind": "gratitude" }
  ],
  "jitterMinutes": 10,
  "quietHours": { "start": "23:00", "end": "07:30" },
  "quietMode": false,
  "activePack": "mochi",
  "displaySeconds": 8
}
```
`kind` is one of `goal`, `quote`, `gratitude` or `proud`. Set `"quietHours": null` to disable quiet hours.

## How it's built

```
crates/tospu-core           pure Rust: settings, scheduler, pack discovery (unit tested)
src-tauri/src               Tauri shell: tray menu, popup window, scheduler thread, IPC
  popup_window/             the ONLY OS-specific layer (PopupWindow trait)
src/domain                  shared types
src/sprites                 manifest schema, renderer registry, sprite-sheet renderer
src/content                 content schema + no-repeat picker (ContentSource interface)
src/popup                   popup lifecycle + speech bubble
tools/slice_sprites.py      generated sheet → sprite pack
sprites/                    bundled packs: mochi, mochi-bounce, mochi-fluffy
```

Extension points, all add-only:
- **New character:** add a folder.
- **New animation format:** add a `SpriteRenderer` and one import in `src/sprites/builtin.ts`.
- **New content source** (remote, AI): implement `ContentSource`.
- **New OS behaviour:** implement `PopupWindow`.

## Tests

```bash
npm test                                   # TypeScript (Vitest)
cd src-tauri && cargo test --workspace     # Rust core
python3 -m pytest tools/tests              # sprite slicer
```

## Known limits (v1)

- Tested to compile on Linux. **Not yet run on a real Mac**, so the first job is checking that the see-through popup stays on top of full-screen apps and across Spaces.
- Full-screen/meeting detection isn't implemented yet (`PopupWindow::foreground_is_fullscreen` returns `false`), so a popup can appear during a presentation. Use quiet mode for now.
- Windows and Linux use the same generic window code but aren't tested.
- The menu bar panel that shows all goals at a glance is planned for v1.1.
