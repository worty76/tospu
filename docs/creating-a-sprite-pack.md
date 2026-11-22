# Creating a sprite pack

A character is just a folder. Tospu discovers it, validates `manifest.json` in Rust, and lists it under **Settings → Character** and the **Character** menu bar submenu. You don't need to change any code.

```
~/Library/Application Support/io.github.worty76.tospu/sprites/
  my-fox/
    manifest.json
    idle.png  wave.png  cheer.png  sleep.png
    preview.png
```

Open this folder from **Settings → Character → Open sprites folder**, then click **Reload files** in **Settings → General**. A pack in your user folder overrides a bundled pack with the same `id`.

## Fast path: generate the art, then slice it

### 1. Lock in the character (one image)

```
Design an original ultra-cute kawaii mascot for a desktop companion app:
<DESCRIBE YOUR CHARACTER: e.g. "a plump round rice-cake spirit with a tiny green
sprout on its head">. Huge head, tiny body, big glossy eyes with two white
highlights, small smiling mouth, soft pink blush, stubby arms and feet.
Soft pastel palette, thin dark-brown outline, clean chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

### 2. Ask for the animation sheet (same chat, so the character stays the same)

```
Using exactly this character, make ONE image with 4 horizontal strips on a flat
solid magenta (#FF00FF) background, separated by thin white gaps:
  Strip 1 "idle"  – 6 frames: gentle breathing bob, loops seamlessly.
  Strip 2 "wave"  – 8 frames: hops in place, waves one arm, settles.
  Strip 3 "cheer" – 8 frames: jumps with both arms up, sparkles, lands softly.
  Strip 4 "sleep" – 6 frames: yawns, curls up, falls asleep with a small "z".
Rules for every frame:
- frames evenly spaced, same size, character fully inside its frame with
  empty margin all around (nothing touches the frame edge or the strip edge)
- same character scale in every frame, feet on the same baseline
  (only jumps leave the baseline)
- NO ground shadows, NO motion/speed lines, NO text, NO borders
- effects (sparkles, z's) small and close to the character
Pixel-art style, crisp edges.
```

### 3. Slice it into a pack

```bash
python3 tools/slice_sprites.py my-sheet.png --id my-fox --name "Little Fox" \
  --out ~/Library/Application\ Support/io.github.worty76.tospu/sprites
```

The slicer finds the magenta strips, removes the background and shadows, centres each frame, aligns all strips on one baseline, and writes the PNGs plus `manifest.json`. Requires `pip install pillow numpy scipy`. Other frame counts: `--frames 4,6,6,4`.

### 4. Test it in the app

Run `cargo tauri dev`, open **Settings → Character**, select your pack, and click **Show a test popup** (or **Show one now** from the menu bar).

## `manifest.json` reference

```json
{
  "schemaVersion": 1,
  "id": "my-fox",
  "name": "Little Fox",
  "author": "you",
  "license": "Personal use",
  "renderer": "spritesheet",
  "frame": { "width": 198, "height": 170, "displayHeight": 120 },
  "pixelated": false,
  "animations": {
    "idle":  { "sheet": "idle.png",  "frames": 6, "fps": 8,  "loop": true },
    "enter": { "sheet": "wave.png",  "frames": 8, "fps": 12 },
    "cheer": { "sheet": "cheer.png", "frames": 8, "fps": 12 },
    "exit":  { "sheet": "sleep.png", "frames": 6, "fps": 8 }
  },
  "moods": { "goal": "cheer", "proud": "cheer", "gratitude": "idle", "quote": "idle" },
  "bubble": { "anchor": "top" }
}
```

- Every sheet is **one horizontal row** of equal-size frames (`frame.width × frames` equals the image width).
- Only `idle` is required. Missing `enter`, `cheer`, or `exit` animations fall back to `idle`.
- `moods` maps each message kind (`goal`, `quote`, `gratitude`, `proud`) to an animation name (defaults to `idle`).
- A broken pack is never fatal: it appears greyed out in **Settings → Character** and in **Settings → General → Problems** with the exact validation error.

## Validation rules & error messages

Manifest validation runs in Rust (`tospu_model::Manifest::parse` and `tospu_core::packs::discover_packs`) and reports errors as `<field>: <reason>`:

- `schemaVersion: must be 1`
- `id: use lowercase letters, digits and dashes` (must match `^[a-z0-9][a-z0-9-]*$` and equal the folder name)
- `name: must not be empty`
- `renderer: unsupported renderer ...` (currently only `"spritesheet"`)
- `frame.width` / `frame.height` / `frame.displayHeight`: must be greater than `0`
- `animations.idle: missing field`
- `animations.<name>.sheet: must be a file name inside the pack folder` (`.png`, `.webp`, or `.gif`, no `..` or path separators)
- `animations.<name>.frames: must be greater than 0`
- `animations.<name>.fps: must be between 0 and 60`

## Adding a new renderer (developers)

1. Add a new variant to `RendererKind` in `crates/tospu-model/src/manifest.rs`.
2. Add the corresponding Leptos component in `ui/src/popup/sprite.rs`.
