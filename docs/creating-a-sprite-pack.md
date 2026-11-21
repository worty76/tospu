# Creating a sprite pack

A character is just a folder. Yub finds it, validates it and lists it under **Character** in the menu bar. You don't need to change any code.

```
~/Library/Application Support/dev.yub.companion/sprites/
  my-fox/
    manifest.json
    idle.png  wave.png  cheer.png  sleep.png
    preview.png
```

Open this folder from the menu bar with **Open sprites folder**. Choose **Reload** after adding a pack. A pack in this folder replaces a bundled pack with the same id.

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

Why these rules? Your first sheets had three problems the slicer has to fix or work around:
- In two of them, the first wave frame **hops in from outside the strip** and gets clipped. "Fully inside its frame" prevents that.
- The **purple ground shadows** are removed automatically, but asking for no shadows keeps the outline clean.
- **Effects far from the body** can spill into the neighbouring frame. The slicer warns you if that happens.

### 3. Slice it into a pack

```bash
python3 tools/slice_sprites.py my-sheet.png --id my-fox --name "Little Fox" \
  --out ~/Library/Application\ Support/dev.yub.companion/sprites
```

The slicer finds the magenta strips, removes the background and shadows, centres each frame, aligns all strips on one baseline, and writes the PNGs plus `manifest.json`. It prints a warning when art crosses a frame border. If you see one, regenerate the sheet or fix the PNG by hand.

Requires `pip install pillow numpy scipy`. Other frame counts: `--frames 4,6,6,4`.

### 4. Preview it in a browser before installing

Put the pack in the repo's `sprites/` folder, run `npm run dev`, and open
`http://localhost:1430/?pack=my-fox`. Add `&quiet=1` for quiet mode and `&seconds=4` to change how long it stays.

## manifest.json reference

```json
{
  "schemaVersion": 1,
  "id": "my-fox",                       // must equal the folder name; a-z 0-9 -
  "name": "Little Fox",
  "author": "you",
  "license": "Personal use",
  "renderer": "spritesheet",            // which renderer draws it
  "frame": { "width": 198, "height": 170, "displayHeight": 120 },
  "pixelated": false,                   // true = crisp nearest-neighbour scaling
  "animations": {
    "idle":  { "sheet": "idle.png",  "frames": 6, "fps": 8,  "loop": true },  // required
    "enter": { "sheet": "wave.png",  "frames": 8, "fps": 12 },                // optional
    "cheer": { "sheet": "cheer.png", "frames": 8, "fps": 12 },                // optional
    "exit":  { "sheet": "sleep.png", "frames": 6, "fps": 8 }                  // optional
  },
  "moods": { "goal": "cheer", "proud": "cheer", "gratitude": "idle", "quote": "idle" },
  "bubble": { "anchor": "top" }
}
```

(Real JSON has no comments. They're only here to explain the fields.)

- Every sheet is **one horizontal row** of equal-size frames. `frame.width × frames` must equal the image width.
- A missing animation **falls back to `idle`**, so the smallest valid pack is one sheet.
- `moods` picks the animation that plays while each kind of message is on screen.
- A broken pack is skipped, never fatal. The reason appears in the popup's dev console and in the browser preview console.

## Adding a new animation format (developers)

1. Create `src/sprites/<format>-renderer.ts` implementing `SpriteRenderer` (`mount`, `play`, `stop`, `destroy`).
2. Call `registerRenderer("<format>", (pack) => new YourRenderer(pack))` at the bottom of that file.
3. Add `import "./<format>-renderer";` to `src/sprites/builtin.ts`.

Packs then opt in with `"renderer": "<format>"`. Nothing else changes.
