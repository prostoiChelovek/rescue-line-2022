from cv2 import cv2 as cv

from .common import clean_mask

BLACK_COLOR_RANGE = (
        (0, 0, 0),
        (60, 60, 60)
        )
GREEN_COLOR_RANGE = (
        (140, 225, 100),
        (200, 255, 180)
        )
SILVER_COLOR_RANGE = (
        (55, 70, 90),
        (90, 90, 115)
        )
OBSTACLE_COLOR_RANGE = (
        (80, 60, 55),
        (110, 70, 65)
        )



def find_black(img) -> cv.Mat:
    return find_color(img, BLACK_COLOR_RANGE)


def find_green(img) -> cv.Mat:
    return find_color(img, GREEN_COLOR_RANGE)


def find_silver(img) -> cv.Mat:
    return find_color(img, SILVER_COLOR_RANGE)


def find_obstacle(img) -> cv.Mat:
    return find_color(img, OBSTACLE_COLOR_RANGE)


def find_color(img: cv.Mat, range_) -> cv.Mat:
    mask = cv.inRange(img, *range_)
    return clean_mask(mask)
