from typing import Optional, Union

import cv2 as cv
import numpy as np

from .colors import find_black
from .window import Window, px2win, win2px, windows_in_image
from .common import get_fill_frac, is_mat_empty, lower_row, upper_row

FIND_WINDOW_STEP = 0.5


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
                max_offset: Optional[float] = None) -> Optional[Window]:
    max_pos = max_offset or windows_in_image(img)
    for pos in np.arange(start, max_pos, FIND_WINDOW_STEP):
        win = Window(img, pos)
        if validate_window(win):
            return win
    return None


def main():
    from . import colors
    colors.BLACK_COLOR_RANGE =  (
            (0, 0, 0),
            (120, 120, 120)
            )

    img = cv.imread("./vision/images/intersection/4.jpg")
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))

    mask = find_black(img)
    win = find_window(mask)
    if win is None:
        print("no window")
        return

    win.draw(img)

    cv.imshow("mask", mask)
    cv.imshow("win", win.roi)
    cv.imshow("img", img)
    while cv.waitKey(0) != 27:
        pass


if __name__ == "__main__":
    main()
