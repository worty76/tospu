import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from slice_sprites import cell_border_warnings, find_strips, key_out  # noqa: E402

MAGENTA = (255, 0, 255)


def _canvas(h=400, w=300):
    return np.full((h, w, 3), 255, dtype=np.uint8)


def test_find_strips_returns_each_magenta_band():
    img = _canvas()
    for y0 in (10, 110, 210, 310):
        img[y0 : y0 + 60] = MAGENTA
    assert find_strips(img) == [(10, 70), (110, 170), (210, 270), (310, 370)]


def test_find_strips_survives_characters_covering_most_of_a_row():
    img = _canvas()
    img[50:150] = MAGENTA
    img[80:120, 10:260] = (250, 235, 215)  # a wide character
    assert find_strips(img) == [(50, 150)]


def test_key_out_removes_background_and_shadow_but_keeps_character_colours():
    px = np.array(
        [[MAGENTA, (120, 20, 120), (250, 235, 215), (240, 160, 150), (60, 30, 20), (40, 40, 80)]],
        dtype=np.uint8,
    )
    alpha = key_out(px)[..., 3]
    assert alpha.tolist() == [[0, 0, 255, 255, 255, 255]]


def test_cell_border_warning_when_foreground_crosses_a_border():
    rgba = np.zeros((20, 100, 4), dtype=np.uint8)
    rgba[5:15, 45:55, 3] = 255  # straddles the border at x=50 for 2 cells
    assert cell_border_warnings(rgba, frames=2) == [1]


def test_no_warning_when_frames_sit_inside_cells():
    rgba = np.zeros((20, 100, 4), dtype=np.uint8)
    rgba[5:15, 10:30, 3] = 255
    rgba[5:15, 60:80, 3] = 255
    assert cell_border_warnings(rgba, frames=2) == []
