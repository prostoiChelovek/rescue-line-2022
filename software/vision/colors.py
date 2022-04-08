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
        (0, 0, 0),
        (0, 0, 0)
        )
OBSTACLE_COLOR_RANGE = (
        (0, 0, 0),
        (0, 0, 0)
        )



def find_black(img):
    return find_color(img, BLACK_COLOR_RANGE)


def find_green(img):
    return find_color(img, GREEN_COLOR_RANGE)


def find_silver(img):
    return find_color(img, SILVER_COLOR_RANGE)


def find_obstacle(img):
    return find_color(img, OBSTACLE_COLOR_RANGE)


def find_color(img, range):
    mask = cv.inRange(img, *range)
    return clean_mask(mask)
