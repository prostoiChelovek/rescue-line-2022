from cv2 import cv2 as cv

from .common import clean_mask

BLACK_COLOR_RANGE = (
        (0, 0, 0),
        (50, 50, 65)
        )
GREEN_COLOR_RANGE = (
        (45, 70, 0),
        (65, 90, 20)
        )


def find_black(img):
    return find_color(img, BLACK_COLOR_RANGE)


def find_green(img):
    return find_color(img, GREEN_COLOR_RANGE)


def find_color(img, range):
    mask = cv.inRange(img, *range)
    return clean_mask(mask)
