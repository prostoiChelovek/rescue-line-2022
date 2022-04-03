from cv2 import cv2 as cv

from .common import clean_mask

BLACK_COLOR_RANGE = (
        (0, 0, 0),
        (50, 50, 65)
        )


def find_black(img):
    mask = cv.inRange(img, *BLACK_COLOR_RANGE)
    return clean_mask(mask)
