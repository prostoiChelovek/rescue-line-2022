from enum import Enum
from typing import Tuple

import numpy as np
import cv2.cv2 as cv

from .colors import find_green

TEST_IMAGE = "./vision/images/intersection/0.jpg"


class MarkersPosition(Enum):
    NONE = 0
    LEFT = 1
    RIGHT = 2
    BOTH = 3


def find(img, line_x: int, black_window: Tuple[int, int]) -> MarkersPosition:
    green_window = img[black_window[0]:black_window[1],:]
    left_green = np.flip(green_window[:,:line_x], axis=1)
    right_green = green_window[:,line_x:]

    has_green = [np.any(x > 0) for x in (left_green, right_green)]
    position_num = sum(x + i * x for i, x in enumerate(has_green))

    return MarkersPosition(position_num)



def main():
    from . import line
    from .colors import find_black

    img = cv.imread(TEST_IMAGE)
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))
    img = img[:230,:]

    green = find_green(img)
    black = find_black(img)

    black_window_pos = line.get_window_pos(black)
    if black_window_pos is None:
        raise Exception("no window")

    line_x = line.find(black, black_window_pos)

    marker_pos = find(green, line_x, black_window_pos)
    print(marker_pos)

    img[:, line_x] = (255, 0, 0)
    cv.imshow("img", img)

    while cv.waitKey(0) != 27:  # esc
        pass


if __name__ == "__main__":
    main()
