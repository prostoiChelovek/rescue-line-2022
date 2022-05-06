import math
import pytest

import numpy as np
import cv2 as cv

from .common import left_half, lower_row, mid_row, upper_row
from .line import find_window, validate_window
from .window import Window, win2px

LINE_ANGLE = -15  # deg
WINDOW_WIDTH = 100
WINDOWS_IN_IMAGE = 15
IMAGE_SIZE = (200, win2px(WINDOWS_IN_IMAGE))


@pytest.fixture
def line_img():
    img = np.zeros(shape=IMAGE_SIZE[::-1], dtype="uint8")

    x1 = IMAGE_SIZE[0] // 2
    y1 = 0
    y2 = IMAGE_SIZE[1]
    angle = math.radians(LINE_ANGLE)
    x2 = x1 + y2 * math.tan(angle)

    x1, y1, x2, y2 = map(int, (x1, y1, x2, y2))

    cv.line(img, (x1, y1), (x2, y2), (255,), 3)

    return img


@pytest.fixture
def line_win(line_img, offset: float = 0.0):
    return Window(line_img, offset)


def test_empty_invalid(line_win: Window):
    line_win.roi.fill(0)
    assert not validate_window(line_win.roi)


def test_fully_filled_invalid(line_win: Window):
    line_win.roi.fill(255)
    assert not validate_window(line_win.roi)


def test_empty_lower_row_invalid(line_win: Window):
    lower_row(line_win.roi).fill(0)
    assert not validate_window(line_win.roi)


def test_empty_upper_row_invalid(line_win: Window):
    upper_row(line_win.roi).fill(0)
    assert not validate_window(line_win.roi)


def test_half_filled_invalid(line_win: Window):
    left_half(line_win.roi).fill(255)
    assert not validate_window(line_win.roi)


def test_empty_mid_row_valid(line_win: Window):
    mid_row(line_win.roi).fill(0)
    assert validate_window(line_win.roi)


@pytest.mark.parametrize("line_win",
                         range(0, WINDOWS_IN_IMAGE, 2),
                         indirect=True)
def test_line_window_valid(line_win: Window):
    assert validate_window(line_win.roi)


def test_find_window_finds_something(line_win: Window):
    found_win = find_window(line_win.img, 0)
    assert found_win is not None


def test_find_window_no_false_positive(line_win: Window):
    line_win.img.fill(0)
    found_win = find_window(line_win.img, 0)
    assert found_win is None


def test_find_window_finds_lowest(line_win: Window):
    found_win = find_window(line_win.img, 0)
    assert found_win is not None
    assert found_win.end == line_win.img.shape[0]


def test_find_window_with_small_gap(line_win: Window):
    lower_row(line_win.img).fill(0)
    found_win = find_window(line_win.img, 0)

    height = line_win.img.shape[0]
    assert found_win is not None
    assert found_win.end < height
    assert found_win.end > height - win2px(1)


def test_find_window_not_too_far(line_win: Window):
    lower_row(line_win.img, height=win2px(1.5)).fill(0)
    found_win = find_window(line_win.img, 0, max_offset=3)

    height = line_win.img.shape[0]
    assert found_win is not None
    assert found_win.end < height
    assert found_win.end >= height - win2px(3)


def test_find_window_does_not_find_if_too_far(line_win: Window):
    lower_row(line_win.img, height=win2px(1.5)).fill(0)
    found_win = find_window(line_win.img, 0, max_offset=1)

    assert found_win is None
