from cv2 import cv2 as cv

from .common import clean_mask

BLACK_COLOR_RANGE = (
        (20, 10, 85),
        (55, 45, 120)
        )


def find_black(img):
    mask = cv.inRange(img, *BLACK_COLOR_RANGE)
    return clean_mask(mask)
