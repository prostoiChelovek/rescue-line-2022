import math
from typing import Optional, Tuple
import logging

import numpy as np
from cv2 import cv2 as cv
import skimage.measure

from .common import clean_mask

TEST_IMAGE = "./vision/images/0.jpg"
LINE_COLOR_RANGE = (
        (0, 0, 0),
        (50, 50, 65)
        )

WINDOW_HEIGHT = 20


def segment_unconnected(img):
    labels = skimage.measure.label(img)
    return skimage.measure.regionprops(label_image=labels)


def get_best_region(regions):
    def region_score(region):
        return math.sqrt(region.area) * 0.25 + region.centroid[1]
    return max(regions, key=region_score, default=None)


def validate_region(image_shape, region):
    max_area = image_shape[0] * image_shape[1]
    is_too_high = region.centroid[0] < (image_shape[1] // 3) * 2
    is_too_big = region.area > max_area // 2
    return not (is_too_high and is_too_big)


def get_window_pos(img) -> Optional[Tuple[int, int]]:
    white = np.argwhere(img)
    if white.size == 0:
        return None
    lowest_white_y = white[-1][0]
    return lowest_white_y - WINDOW_HEIGHT, lowest_white_y


def find(mask,
         window_pos: Optional[Tuple[int, int]] = None) -> Optional[int]:
    window_pos = window_pos or get_window_pos(mask)
    if window_pos is None:
        return None
    window = mask[window_pos[0]:window_pos[1]]

    regions = segment_unconnected(window)
    region = get_best_region(regions)

    if not validate_region(mask.shape, region):
        logging.debug("Ignoring and invalid region")
        return None

    line_x = int(region.centroid[1])
    return line_x


def main():
    img = cv.imread(TEST_IMAGE)
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))

    mask = cv.inRange(img, *LINE_COLOR_RANGE)
    mask = clean_mask(mask)

    line_x = find(mask)

    img[:, line_x] = (255, 0, 0)

    cv.imshow("mask", mask)
    cv.imshow("img", img)
    while cv.waitKey(0) != 27:
        pass


if __name__ == "__main__":
    main()
