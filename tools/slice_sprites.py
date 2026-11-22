#!/usr/bin/env python3
"""Turn a generated character sheet into a Tospu sprite pack.

Expected input: an image with one or more solid-magenta (#FF00FF) strips,
each holding N evenly spaced frames of one animation (top to bottom:
idle, wave, cheer, sleep by default). Anything outside the strips, such as
a reference row on white, is ignored.

Output: <out>/<id>/ with one horizontal sheet per animation (all frames the
same size, background transparent, strips aligned on a shared baseline),
a preview.png and a manifest.json.

Usage:
  python3 tools/slice_sprites.py SHEET.png --id mochi --name "Mochi" --out sprites
  python3 tools/slice_sprites.py SHEET.png --id fox --frames 6,8,8,6 \
      --animations idle,enter,cheer,exit

Requires: Pillow, numpy, scipy.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

STRIP_ROW_MAGENTA_FRACTION = 0.15
MIN_STRIP_HEIGHT = 40
EDGE_INSET = 3
PADDING = 6
# A frame whose biggest blob is smaller than this share of the typical body
# (for example only speed lines) keeps its position instead of being centred.
MIN_BODY_SHARE = 0.35

DEFAULT_ANIMATIONS = ["idle", "enter", "cheer", "exit"]
DEFAULT_FRAMES = [6, 8, 8, 6]
SHEET_FILE = {"idle": "idle.png", "enter": "wave.png", "cheer": "cheer.png", "exit": "sleep.png"}
TIMING = {
    "idle": {"fps": 8, "loop": True},
    "enter": {"fps": 12, "loop": False},
    "cheer": {"fps": 12, "loop": False},
    "exit": {"fps": 8, "loop": False},
}


def _magenta_mask(rgb: np.ndarray) -> np.ndarray:
    r, g, b = (rgb[..., i].astype(int) for i in range(3))
    return (r > 200) & (b > 200) & (g < 80)


def find_strips(rgb: np.ndarray) -> list[tuple[int, int]]:
    """Return (y0, y1) row ranges of the magenta strips, top to bottom."""
    rows = _magenta_mask(rgb).mean(axis=1) > STRIP_ROW_MAGENTA_FRACTION
    strips, start = [], None
    for y, on in enumerate([*rows, False]):
        if on and start is None:
            start = y
        elif not on and start is not None:
            if y - start >= MIN_STRIP_HEIGHT:
                strips.append((start, y))
            start = None
    return strips


def key_out(rgb: np.ndarray) -> np.ndarray:
    """RGB -> RGBA with the magenta background and its purple shadows removed.

    A pixel goes transparent when both red and blue clearly exceed green,
    which covers pure magenta, the darker purple drop shadows and the
    magenta fringe on outlines, while cream, blush, brown, green and navy
    character colours survive.
    """
    r, g, b = (rgb[..., i].astype(int) for i in range(3))
    background = (r - g > 30) & (b - g > 30)
    alpha = np.where(background, 0, 255).astype(np.uint8)
    return np.dstack([rgb[..., :3].astype(np.uint8), alpha])


def cell_border_warnings(rgba: np.ndarray, frames: int) -> list[int]:
    """Indices of inner cell borders (1..frames-1) that foreground crosses."""
    width = rgba.shape[1]
    opaque_cols = rgba[..., 3].max(axis=0) > 0
    hits = []
    for i in range(1, frames):
        x = round(i * width / frames)
        if opaque_cols[max(0, x - 1) : x + 1].any():
            hits.append(i)
    return hits


def _drop_edge_blobs(rgba: np.ndarray) -> np.ndarray:
    """Remove blobs touching the strip's left/right edge (clipped art, bars)."""
    labels, _ = ndimage.label(rgba[..., 3] > 0)
    edge_ids = set(np.unique(labels[:, 0])) | set(np.unique(labels[:, -1]))
    edge_ids.discard(0)
    out = rgba.copy()
    if edge_ids:
        out[np.isin(labels, list(edge_ids)), 3] = 0
    return out


@dataclass
class Frame:
    pixels: np.ndarray  # RGBA, full cell
    anchor_x: float  # body centre within the cell
    body_bottom: int | None  # lowest body row, None if no real body
    body_area: int


def _measure(cell: np.ndarray) -> Frame:
    labels, count = ndimage.label(cell[..., 3] > 0)
    if count == 0:
        return Frame(cell, cell.shape[1] / 2, None, 0)
    sizes = ndimage.sum(np.ones_like(labels), labels, index=range(1, count + 1))
    biggest = int(np.argmax(sizes)) + 1
    ys, xs = np.nonzero(labels == biggest)
    return Frame(cell, float(xs.mean()), int(ys.max()), int(sizes[biggest - 1]))


@dataclass
class Strip:
    name: str
    frames: list[Frame]
    baseline: int


def _cut_strip(rgb: np.ndarray, y0: int, y1: int, name: str, n: int) -> tuple[Strip, list[int]]:
    region = rgb[y0:y1]
    cols = np.nonzero(_magenta_mask(region).mean(axis=0) > 0.3)[0]
    x0, x1 = int(cols.min()), int(cols.max()) + 1
    rgba = key_out(region[EDGE_INSET:-EDGE_INSET, x0 + EDGE_INSET : x1 - EDGE_INSET])
    rgba = _drop_edge_blobs(rgba)
    warnings = cell_border_warnings(rgba, n)

    width = rgba.shape[1]
    frames = [_measure(rgba[:, round(i * width / n) : round((i + 1) * width / n)]) for i in range(n)]
    typical = float(np.median([f.body_area for f in frames if f.body_area])) if any(f.body_area for f in frames) else 0
    for f in frames:
        if f.body_area < typical * MIN_BODY_SHARE:
            f.anchor_x = f.pixels.shape[1] / 2
            f.body_bottom = None
    bottoms = [f.body_bottom for f in frames if f.body_bottom is not None]
    baseline = int(np.median(bottoms)) if bottoms else rgba.shape[0] - 1
    return Strip(name, frames, baseline), warnings


def _opaque_bounds(px: np.ndarray) -> tuple[int, int, int, int] | None:
    ys, xs = np.nonzero(px[..., 3] > 0)
    if len(xs) == 0:
        return None
    return int(xs.min()), int(xs.max()), int(ys.min()), int(ys.max())


def _layout(strips: list[Strip]) -> tuple[int, int, int]:
    """Shared cell (width, height, baseline_y) that fits every frame."""
    half_w = above = below = 1
    for s in strips:
        for f in s.frames:
            b = _opaque_bounds(f.pixels)
            if b is None:
                continue
            x_min, x_max, y_min, y_max = b
            half_w = max(half_w, f.anchor_x - x_min, x_max - f.anchor_x)
            above = max(above, s.baseline - y_min)
            below = max(below, y_max - s.baseline)
    width = int(np.ceil(half_w)) * 2 + 2 * PADDING
    baseline_y = int(above) + PADDING
    height = baseline_y + int(below) + PADDING
    return width, height, baseline_y


def _render_strip(s: Strip, width: int, height: int, baseline_y: int) -> Image.Image:
    sheet = Image.new("RGBA", (width * len(s.frames), height), (0, 0, 0, 0))
    for i, f in enumerate(s.frames):
        dx = round(width / 2 - f.anchor_x)
        dy = baseline_y - s.baseline
        sheet.alpha_composite(Image.fromarray(f.pixels, "RGBA"), (i * width + dx, dy))
    return sheet


def slice_sheet(
    path: Path,
    pack_id: str,
    name: str,
    out_root: Path,
    animations: list[str],
    frame_counts: list[int],
    author: str,
    display_height: int,
) -> list[str]:
    rgb = np.asarray(Image.open(path).convert("RGB"))
    found = find_strips(rgb)
    if len(found) < len(animations):
        raise SystemExit(f"{path}: expected {len(animations)} magenta strips, found {len(found)}")

    problems: list[str] = []
    strips = []
    for (y0, y1), anim, n in zip(found, animations, frame_counts):
        strip, borders = _cut_strip(rgb, y0, y1, anim, n)
        strips.append(strip)
        for b in borders:
            problems.append(f"{anim}: art crosses the border between frame {b} and {b + 1}")

    width, height, baseline_y = _layout(strips)
    out = out_root / pack_id
    out.mkdir(parents=True, exist_ok=True)
    manifest_anims = {}
    for s in strips:
        file = SHEET_FILE.get(s.name, f"{s.name}.png")
        _render_strip(s, width, height, baseline_y).save(out / file)
        timing = TIMING.get(s.name, {"fps": 10, "loop": False})
        manifest_anims[s.name] = {"sheet": file, "frames": len(s.frames), **timing}

    idle = next((s for s in strips if s.name == "idle"), strips[0])
    _render_strip(Strip(idle.name, idle.frames[:1], idle.baseline), width, height, baseline_y).save(out / "preview.png")

    manifest = {
        "schemaVersion": 1,
        "id": pack_id,
        "name": name,
        "author": author,
        "license": "Personal use",
        "renderer": "spritesheet",
        "frame": {"width": width, "height": height, "displayHeight": display_height},
        "pixelated": False,
        "animations": manifest_anims,
        "moods": {"goal": "cheer", "proud": "cheer", "gratitude": "idle", "quote": "idle"},
        "bubble": {"anchor": "top"},
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return problems


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("sheet", type=Path)
    p.add_argument("--id", required=True, help="pack id, also the folder name")
    p.add_argument("--name", help="display name (default: id)")
    p.add_argument("--out", type=Path, default=Path("sprites"))
    p.add_argument("--animations", default=",".join(DEFAULT_ANIMATIONS))
    p.add_argument("--frames", default=",".join(map(str, DEFAULT_FRAMES)))
    p.add_argument("--author", default="you")
    p.add_argument("--display-height", type=int, default=120, help="on-screen height in CSS px")
    a = p.parse_args(argv)

    animations = a.animations.split(",")
    frames = [int(x) for x in a.frames.split(",")]
    if len(animations) != len(frames):
        p.error("--animations and --frames need the same number of entries")

    problems = slice_sheet(a.sheet, a.id, a.name or a.id, a.out, animations, frames, a.author, a.display_height)
    print(f"wrote {a.out / a.id}")
    for msg in problems:
        print(f"warning: {msg}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
