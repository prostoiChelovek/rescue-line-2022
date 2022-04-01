import numpy as np
from cv2 import cv2 as cv

import random as rng

from common import clean_mask

rng.seed(42)

TEST_IMAGE = "./images/0.jpg"
LINE_COLOR_RANGE = (
        (0, 0, 0),
        (50, 50, 65)
        )


def find_contours(img):
    contours, hierarchy = cv.findContours(img, cv.RETR_CCOMP,
                                          cv.CHAIN_APPROX_SIMPLE)
    return contours, hierarchy


def draw_contours(img_size, contours, hierarchy, thickness=1):
    drawing = np.zeros((img_size[0], img_size[1], 3), dtype=np.uint8)

    for i in range(len(contours)):
        color = (rng.randint(0, 256), rng.randint(0, 256), rng.randint(0, 256))
        cv.drawContours(drawing, contours, i, color, thickness,
                        cv.LINE_8, hierarchy, 0)

    return drawing


def draw_contour(img_size, contours, hierarchy, i, thickness=1):
    drawing = np.zeros((img_size[0], img_size[1]), dtype=np.uint8)
    cv.drawContours(drawing, contours, i, (255,),
                    thickness, cv.LINE_8, hierarchy, 0)
    return drawing


def segment_unconnected(img):
    contours, hierarchy = find_contours(img)
    return [draw_contour(img.shape, contours, hierarchy, i, cv.FILLED)
            for i in range(len(contours))]


def sample_function(params, values_range, resolution=1):
    start, stop = values_range
    xs = np.linspace(start, stop, (stop - start) // resolution)
    return xs, np.poly1d(params)(xs)


def polyfit_find(img):
    white_points = np.where(img == 255)[::-1]
    # равносильно повороту изображения на 90 градусов
    white_points = np.flip(white_points)
    values_range = np.min(white_points[0]), np.max(white_points[0])

    params = np.polyfit(*white_points, deg=6, full=True)
    params, residuals = params[:2]

    cords = np.array(sample_function(params, values_range)).astype(int).T
    cords = np.flip(cords, axis=1)

    greater_zero_filter = np.all(np.zeros((2,)) <= cords, axis=1)
    less_bound_filter = np.all(cords < img.shape[::-1], axis=1)
    cords = cords[greater_zero_filter & less_bound_filter]

    return cords[:, 0], cords[:, 1], residuals


def set_border(a, val):
    a[0, :] = val
    a[-1, :] = val
    a[:, 0] = val
    a[:, -1] = val


def main():
    img = cv.imread(TEST_IMAGE)
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))

    mask = cv.inRange(img, *LINE_COLOR_RANGE)
    mask = clean_mask(mask)

    white_points = np.array(np.where(mask == 255)).T
    lowest_white = np.max(white_points[:, 0])

    window = white_points[white_points[:, 0] > lowest_white - 1]
    line_x = int(np.median(window, axis=0)[1])

    img[:, line_x] = (255, 0, 0)

    cv.imshow("mask", mask)
    cv.imshow("img", img)
    while cv.waitKey(0) != 27:
        pass


if __name__ == "__main__":
    main()
