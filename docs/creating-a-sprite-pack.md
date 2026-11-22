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

> **Color tip:** Avoid purple or magenta anywhere on the character itself. The slicer keys out `#FF00FF` and purple fringes. Cream, orange, yellow, brown, mint/green, sky blue, charcoal, and soft peach/pink work best.

### 1. Lock in the character (one image)

Start a new image-generation chat (e.g. ChatGPT) with this template—or pick one of the ready-to-paste examples below:

```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
<DESCRIBE YOUR CHARACTER: e.g. "a plump round rice-cake spirit with a tiny green
sprout on its head">. Huge head, tiny body, big glossy eyes with two white
highlights, small smiling mouth, soft peach blush, stubby arms and feet.
Soft pastel palette, thin dark-brown outline, NO purple or magenta anywhere on
the character. Clean chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

<details>
<summary>Ready-to-paste character prompt examples</summary>

**Cozy Red Fox (`cozy-fox`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a tiny chibi red fox wearing an oversized sage-green scarf, with a big fluffy
cream-tipped tail, oversized head, big glossy dark-brown eyes with two white
highlights, tiny smiling mouth, soft peach blush, stubby paws.
Warm autumn palette (terracotta orange, cream, sage green, dark-brown outline),
NO purple or magenta anywhere on the character. Clean chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

**Matcha Frog (`matcha-frog`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a round chubby baby tree frog with a tiny golden yuzu fruit resting on its head,
oversized head, big glossy eyes with two white highlights, happy little "w" mouth,
soft coral-pink blush, tiny webbed hands and feet.
Fresh pastel palette (matcha green, cream belly, warm yellow yuzu, dark-olive
outline), NO purple or magenta anywhere on the character. Clean chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

**Baby Cloud Dragon (`cloud-dragon`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a chubby baby eastern cloud dragon with tiny golden antlers, round cream belly,
mint-teal scales, fluffy white mane tuft, stubby little wings and paws, big glossy
eyes with two white highlights, happy little smile and peach blush.
Soft palette (mint teal, warm cream, soft gold, dark-teal outline),
NO purple or magenta anywhere on the character. Clean chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

**Tiny Capybara (`yuzu-capy`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a plump, super-chill chibi capybara balancing a tiny bright tangerine with a green
leaf on top of its head. Huge round head, small cute ears, half-lidded happy glossy
eyes with white highlights, peaceful smile, soft blush, stubby legs.
Warm earthy palette (caramel brown, biscuit cream, tangerine orange, leaf green,
dark-brown outline), NO purple or magenta anywhere on the character. Clean chibi
pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

**Mini Astronaut Cat (`astro-cat`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a tiny white chibi kitten in a chunky retro cream-and-orange spacesuit with an
open clear bubble helmet showing its cute face, cat ears poking into little
helmet bumps, tiny jetpack on back, big glossy eyes with two white highlights.
Retro NASA palette (snow white, warm cream, tangerine orange, sky-blue visor
shine, charcoal outline), NO purple or magenta anywhere on the character. Clean
chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

**Little Red Panda (`red-panda`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a super-fluffy chibi red panda with a huge striped bushy tail, cream eyebrows and
cheeks, dark espresso paws, wearing a tiny golden star pin on one ear, big glossy
eyes with two white highlights, excited open smile.
Rich warm palette (cinnamon rust, cream, espresso brown, gold star, dark-brown
outline), NO purple or magenta anywhere on the character. Clean chibi pixel-art
style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

**Acorn Woodland Spirit (`acorn-sprite`)**
```text
Design an original ultra-cute kawaii mascot for a desktop companion app:
a tiny round cream-colored woodland spirit wearing an oversized brown acorn cap
as a hat, holding a tiny warm glowing yellow lantern, stubby little arms and feet,
big glossy dark eyes with two white highlights, gentle smile and peach blush.
Forest palette (warm cream, chestnut brown acorn cap, moss green leaf, honey-gold
glow, dark-brown outline), NO purple or magenta anywhere on the character. Clean
chibi pixel-art style.
Reference sheet on plain white: front view, 3/4 view, and 4 expressions
(happy, proud, sleepy, cheering). No text.
```

</details>

### 2. Ask for the animation sheet (same chat, so the character stays consistent)

```text
Using exactly this character, make ONE wide image with 4 horizontal strips on a
flat solid magenta (#FF00FF) background, separated by thin white horizontal gaps:
  Strip 1 "idle"  – 6 frames: gentle breathing bob, loops seamlessly.
  Strip 2 "wave"  – 8 frames: hops in place, waves one arm happily, settles.
  Strip 3 "cheer" – 8 frames: jumps with both arms up, tiny gold sparkles close to head, lands softly.
  Strip 4 "sleep" – 6 frames: yawns, curls up, falls asleep with a tiny yellow "z".
Strict rules for every frame:
- evenly spaced identical cells with WIDE empty magenta padding between adjacent
  frames (at least 20% empty magenta on left and right of every character, including
  tail, wings, and sparkles — nothing may touch or cross cell boundaries)
- identical character scale in every frame, feet on the same baseline (only jumps leave it)
- NO ground shadows, NO motion lines, NO borders, NO text labels
- keep sparkles and "z" tiny, close to the body, warm yellow/white (NO purple)
Pixel-art style, crisp edges, flat #FF00FF background only.
```

### 3. Slice it into a pack

```bash
python3 -m venv .venv && .venv/bin/pip install pillow numpy scipy
.venv/bin/python3 tools/slice_sprites.py my-sheet.png --id my-fox --name "Little Fox" \
  --out ~/Library/Application\ Support/io.github.worty76.tospu/sprites
```

The slicer finds the magenta strips, removes the background and shadows, centres each frame, aligns all strips on one baseline, and writes the PNGs (`idle.png`, `wave.png`, `cheer.png`, `sleep.png`, `preview.png`) plus `manifest.json`. Other frame counts: `--frames 4,6,6,4`.

If the slicer warns that art crosses a frame border, ask the generator for wider magenta spacing between frames (or touch up the PNG) and re-slice.

### 4. Test it in the app

Run `cargo tauri dev`, click **Settings → General → Reload files**, select your pack in **Settings → Character**, and click **Show a test popup** (or **Show one now** from the menu bar). The menu bar tray icon will also update to your pack's `preview.png`.

## Contributing a sprite pack

Pull requests with new original characters are welcome!

1. Make sure the character is **original** (no copyrighted anime/game characters) and set `"author"` and `"license"` (for example `"CC-BY-4.0"` or `"CC0-1.0"`) in `manifest.json`.
2. Slice the pack into `sprites/<id>/` in the repository root.
3. Verify it loads cleanly with `cargo tauri dev` and passes `cargo test --workspace`.
4. Open a PR with a quick screenshot or GIF of the character in action.

## `manifest.json` reference

```json
{
  "schemaVersion": 1,
  "id": "my-fox",
  "name": "Little Fox",
  "author": "you",
  "license": "CC-BY-4.0",
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
- `preview.png` is used for the menu bar tray icon (cropped and scaled to 44×44).
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
