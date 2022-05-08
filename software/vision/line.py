from __future__ import annotations

from typing import Iterator, Optional, Union
from dataclasses import dataclass

import cv2 as cv
import numpy as np

from .colors import find_black
from .window import Window, windows_in_image
from .common import get_fill_frac, is_mat_empty, lower_row, upper_row

LINE_WINDOW_STEP = 0.5
LINE_WINDOW_FIRST_MAX_OFFSET = 5.0
LINE_WINDOWS_DISTANCE_RANGE = (2.0, 10.0)


def arange_offset(start: float, offset: float, step: float, include_end: bool = False) -> np.ndarray:
    if step > 0:
        end = start + offset
    elif step < 0:
        if offset > start:
            raise ValueError(f"offset is too big ({start=}, {offset=})")
        end = start - offset
    else:
        raise ValueError("step is 0")
    if include_end:
        end += step
    return np.arange(start, end, step)


@dataclass
class WindowPair:
    lower: Optional[Window]
    upper: Optional[Window]

    @staticmethod
    def empty() -> WindowPair:
        return WindowPair(None, None)

    @property
    def is_complete(self) -> bool:
        return all(x is not None for x in self)

    def draw(self, img: cv.Mat) -> None:
        if self.lower is not None:
            self.lower.draw(img, (0, 0, 255))
        if self.upper is not None:
            self.upper.draw(img, (0, 255, 255))

    def __iter__(self) -> Iterator[Optional[Window]]:
        return iter((self.lower, self.upper))


def validate_window(win: Union[cv.Mat, Window]) -> bool:
    if isinstance(win, Window):
        win = win.roi
    return all([
        not is_mat_empty(lower_row(win)),
        not is_mat_empty(upper_row(win)),
        get_fill_frac(win) < 0.2,
        ])


def find_window(img: cv.Mat,
                start: float = 0,
                max_offset: Optional[float] = None,
                step: Optional[float] = None) -> Optional[Window]:
    max_offset  = max_offset or windows_in_image(img)
    step = step or 1.0

    for pos in arange_offset(start, max_offset, step, include_end=True):
        win = Window(img, pos)
        if validate_window(win):
            return win
    return None


def find_line_window_pair(img: cv.Mat) -> WindowPair:
    res = WindowPair.empty()

    res.lower = find_window(img,
                            start=0.0,
                            max_offset=LINE_WINDOW_FIRST_MAX_OFFSET,
                            step=LINE_WINDOW_STEP)
    if res.lower is None:
        return res

    max_distance = res.lower.pos + 1 + LINE_WINDOWS_DISTANCE_RANGE[1]
    min_distance_offset = LINE_WINDOWS_DISTANCE_RANGE[1] \
                            - LINE_WINDOWS_DISTANCE_RANGE[0]
    res.upper = find_window(img,
                            start=max_distance,
                            max_offset=min_distance_offset,
                            step=-LINE_WINDOW_STEP)

    return res


def main():
    from . import colors
    colors.BLACK_COLOR_RANGE =  (
            (0, 0, 0),
            (120, 120, 120)
            )

    img = cv.imread("./vision/images/intersection/4.jpg")
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))

    mask = find_black(img)
    wins = find_line_window_pair(mask)
    if wins == WindowPair.empty():
        print("no window")
        return

    wins.draw(img)

    cv.imshow("mask", mask)
    cv.imshow("img", img)
    while cv.waitKey(0) != 27:
        pass


if __name__ == "__main__":
    main()
