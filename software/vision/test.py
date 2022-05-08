import math
from typing import Tuple
import pytest

import numpy as np
import cv2 as cv

from .common import draw_angled_line, left_half, lower_row, mid_row, upper_row
from .line import LINE_WINDOWS_DISTANCE_RANGE, MAX_REGIONS_DISTANCE, LineInfo, WindowPair, \
                   arange_offset, bounds_distance, find_line_window_pair, \
                   find_window, get_matching_regions, locate_line, validate_window
from .window import Window, win2px, windows_in_image

LINE_ANGLE = 15  # deg
WINDOW_WIDTH = 100
WINDOWS_IN_IMAGE = 15
IMAGE_SIZE = (200, win2px(WINDOWS_IN_IMAGE))


def approx(x: int):
    return pytest.approx(x, abs=2)


def line_img_impl(x_offset: int = 0):
    img = np.zeros(shape=IMAGE_SIZE[::-1], dtype="uint8")

    draw_angled_line(img,
                     x1=IMAGE_SIZE[0] // 2 + x_offset,
                     y1=IMAGE_SIZE[1],
                     y2=0,
                     angle=math.radians(LINE_ANGLE),
                     color=(255,),
                     thickness=3)

    return img

@pytest.fixture
def line_img(x_offset: int = 0):
    return line_img_impl(x_offset)


def line_win_impl(offset: float = 0.0, x_offset: int = 0):
    return Window(line_img_impl(x_offset), offset,)


@pytest.fixture
def line_win(offset: float = 0.0, x_offset: int = 0):
    return line_win_impl(offset, x_offset)


@pytest.fixture(params = ["offset"])
def line_win_x_offset(request):
    return line_win_impl(x_offset=request.param), request.param


def test_arange_limited_offset():
    assert np.array_equal(arange_offset(0, 10, 1), np.arange(0, 10, 1))
    assert np.array_equal(arange_offset(5, 10, 1), np.arange(5, 15, 1))
    assert np.array_equal(arange_offset(5, 2, -1), np.arange(5, 3, -1))
    with pytest.raises(ValueError):
        arange_offset(0, 10, 0)
    assert np.array_equal(arange_offset(0, 10, 1, True),
                          np.linspace(0, 10, 11, endpoint=True))
    assert np.array_equal(arange_offset(5, 2, -1, True),
                          np.linspace(5, 3, 3, endpoint=True))


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
    found_win = find_window(line_win.img, 0, step=0.5)
    assert found_win is not None


def test_find_window_no_false_positive(line_win: Window):
    line_win.img.fill(0)
    found_win = find_window(line_win.img, 0, step=0.5)
    assert found_win is None


def test_find_window_finds_lowest(line_win: Window):
    found_win = find_window(line_win.img, 0, step=0.5)
    assert found_win is not None
    assert found_win.end == line_win.img.shape[0]


def test_find_window_with_small_gap(line_win: Window):
    lower_row(line_win.img).fill(0)
    found_win = find_window(line_win.img, 0, step=0.5)

    height = line_win.img.shape[0]
    assert found_win is not None
    assert found_win.end < height
    assert found_win.end > height - win2px(1)


def test_find_window_not_too_far(line_win: Window):
    lower_row(line_win.img, height=win2px(1.5)).fill(0)
    found_win = find_window(line_win.img, 0, max_offset=3, step=0.5)

    height = line_win.img.shape[0]
    assert found_win is not None
    assert found_win.end < height
    assert found_win.end >= height - win2px(3)


def test_find_window_does_not_find_if_too_far(line_win: Window):
    lower_row(line_win.img, height=win2px(1.5)).fill(0)
    found_win = find_window(line_win.img, 0, max_offset=1, step=0.5)

    assert found_win is None


def test_find_window_negative_step(line_win: Window):
    upper_row(line_win.img, height=win2px(1.5)).fill(0)
    topmost_win_pos = windows_in_image(line_win.img)
    found_win = find_window(line_win.img,
                            start=topmost_win_pos,
                            max_offset=3,
                            step=-0.5)

    assert found_win is not None
    assert found_win.end > 0
    assert found_win.end <= win2px(3)


def test_find_window_pair_finds_something(line_win: Window):
    wins = find_line_window_pair(line_win.img)

    assert wins != WindowPair.empty()


def test_find_window_pair_no_false_positive(line_win: Window):
    line_win.img.fill(0)
    wins = find_line_window_pair(line_win.img)

    assert wins.lower is None
    assert wins.upper is None


def test_find_window_pair_not_same(line_win: Window):
    wins = find_line_window_pair(line_win.img)

    assert wins.is_complete
    assert wins.lower != wins.upper


def test_find_window_pair_correct_distance_in_px(line_win: Window):
    wins = find_line_window_pair(line_win.img)

    assert wins.is_complete
    assert wins.lower.start - wins.upper.end \
                == win2px(LINE_WINDOWS_DISTANCE_RANGE[1])


def test_find_window_pair_max_distance(line_win: Window):
    wins = find_line_window_pair(line_win.img)

    assert wins.is_complete
    assert wins.lower.pos + 1 + LINE_WINDOWS_DISTANCE_RANGE[1] == \
             wins.upper.pos


def test_find_window_pair_min_distance(line_win: Window):
    line_win.img[:line_win.start - win2px(1 + LINE_WINDOWS_DISTANCE_RANGE[0]),:].fill(0)
    wins = find_line_window_pair(line_win.img)

    assert wins.is_complete
    assert wins.lower.pos + 1 + LINE_WINDOWS_DISTANCE_RANGE[0] == \
            wins.upper.pos


def test_find_window_pair_only_one(line_win: Window):
    line_win.img[:line_win.start, :].fill(0)
    wins = find_line_window_pair(line_win.img)

    assert wins.lower is not None and wins.upper is None


def test_find_window_pair_gap_inbetween(line_win: Window):
    line_win.img[line_win.start - win2px(5):line_win.start - win2px(1), :].fill(0)
    wins = find_line_window_pair(line_win.img)

    assert wins.is_complete
    assert wins.lower.pos + 1 + LINE_WINDOWS_DISTANCE_RANGE[1] == \
            wins.upper.pos


def test_find_window_pair_gap_bellow(line_win: Window):
    line_win.roi.fill(0)
    wins = find_line_window_pair(line_win.img)

    assert wins.is_complete
    assert wins.lower.pos == 1
    assert wins.lower.pos + 1 + LINE_WINDOWS_DISTANCE_RANGE[1] == \
            wins.upper.pos


def test_bounds_distance():
    assert bounds_distance((0, 10), (0, 10)) == 0
    assert bounds_distance((0, 10), (0, 5)) == 0
    assert bounds_distance((0, 10), (5, 10)) == 0
    assert bounds_distance((5, 10), (0, 5)) == 0
    assert bounds_distance((5, 10), (10, 15)) == 0
    assert bounds_distance((0, 20), (5, 15)) == 0

    assert bounds_distance((0, 10), (1, 5)) == 1
    assert bounds_distance((0, 10), (11, 15)) == 1
    assert bounds_distance((11, 15), (0, 10)) == 1
    assert bounds_distance((0, 20), (6, 16)) == 1


def test_get_mathing_regions_single_win(line_win: Window):
    img_center = line_win.img.shape[1] // 2
    assert get_matching_regions(WindowPair(line_win, None)) \
                == [approx(img_center)]


def test_get_mathing_regions_two_windows(line_win: Window):
    line_win.img.fill(0)

    wins = WindowPair(Window(line_win.img, 0),
                      Window(line_win.img, 1))
    thickness = 20
    second_start = MAX_REGIONS_DISTANCE // 2
    wins.lower.roi[:, 0:thickness].fill(255)
    wins.upper.roi[:, second_start:second_start + thickness].fill(255)

    assert get_matching_regions(wins) == [approx(thickness // 2),
                                          approx(second_start + thickness // 2)]


def test_get_mathing_regions_too_far(line_win: Window):
    line_win.img.fill(0)

    wins = WindowPair(Window(line_win.img, 0),
                      Window(line_win.img, 1))
    thickness = 20
    second_start = thickness + MAX_REGIONS_DISTANCE + 1
    wins.lower.roi[:, 0:thickness].fill(255)
    wins.upper.roi[:, second_start:second_start + thickness].fill(255)

    assert get_matching_regions(wins) == [approx(thickness // 2)]


def test_get_mathing_regions_one_in_lower_many_in_upper(line_win: Window):
    line_win.img.fill(0)

    wins = WindowPair(Window(line_win.img, 0),
                      Window(line_win.img, 1))
    thickness = 20
    second_start = MAX_REGIONS_DISTANCE // 2
    third_start = second_start + thickness + 10

    wins.lower.roi[:, 0:thickness].fill(255)
    wins.upper.roi[:, second_start:second_start + thickness].fill(255)
    wins.upper.roi[:, third_start:third_start + thickness].fill(255)

    assert get_matching_regions(wins) == [approx(thickness // 2),
                                          approx(second_start + thickness // 2)]


def test_get_mathing_regions_many_in_lower_one_in_upper(line_win: Window):
    line_win.img.fill(0)

    wins = WindowPair(Window(line_win.img, 0),
                      Window(line_win.img, 1))
    thickness = 20
    second_start = MAX_REGIONS_DISTANCE // 2
    third_start = second_start + thickness + 10

    wins.lower.roi[:, 0:thickness].fill(255)
    wins.upper.roi[:, second_start:second_start + thickness].fill(255)
    wins.lower.roi[:, third_start:third_start + thickness].fill(255)

    assert get_matching_regions(wins) == [approx(thickness // 2),
                                          approx(second_start + thickness // 2)]


@pytest.mark.parametrize("line_win_x_offset",
                         [0, 10, -10],
                         indirect=True)
def test_locate_line_correct_offset_single_win(line_win_x_offset: Tuple[Window, int]):
    win, offset = line_win_x_offset
    assert locate_line(WindowPair(win, None)).x_offset == approx(offset)


def test_locate_line_no_angle_single_win(line_win: Window):
    assert locate_line(WindowPair(line_win, None)).angle is None


@pytest.mark.parametrize("line_win_x_offset",
                         [0, 10, -10],
                         indirect=True)
def test_locate_line_correct_offset(line_win_x_offset: Tuple[Window, int]):
    win, offset = line_win_x_offset
    wins = find_line_window_pair(win.img)
    assert locate_line(wins).x_offset == approx(offset)


def test_locate_line_correct_angle(line_win: Window):
    wins = find_line_window_pair(line_win.img)
    line = locate_line(wins)
    assert line.angle is not None
    assert math.degrees(line.angle) == approx(LINE_ANGLE)
