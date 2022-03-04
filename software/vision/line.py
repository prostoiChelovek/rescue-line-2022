import numpy as np
from cv2 import cv2 as cv
from cv2 import ximgproc

import random as rng

rng.seed(42)

TEST_IMAGE = "test.png"
LINE_COLOR_RANGE = (
        (0, 0, 100),
        (15, 35, 185)
        )


def clean_mask(mask):
    kernel_erote = np.ones((3, 3),np.uint8)
    erosion = cv.erode(mask, kernel_erote, iterations = 1)

    kernel_dilate = np.ones((9, 9),np.uint8)
    dilation = cv.dilate(erosion, kernel_dilate, iterations = 2)

    return dilation


def find_contours(img):
    contours, hierarchy = cv.findContours(img, cv.RETR_CCOMP , cv.CHAIN_APPROX_SIMPLE)
    return contours, hierarchy


def draw_contours(img_size, contours, hierarchy, thickness=1):
    drawing = np.zeros((img_size[0], img_size[1], 3), dtype=np.uint8)

    for i in range(len(contours)):
        color = (rng.randint(0,256), rng.randint(0,256), rng.randint(0,256))
        cv.drawContours(drawing, contours, i, color, thickness, cv.LINE_8, hierarchy, 0)

    return drawing


def draw_contour(img_size, contours, hierarchy, i, thickness=1):
    drawing = np.zeros((img_size[0], img_size[1]), dtype=np.uint8)
    cv.drawContours(drawing, contours, i, (255,), thickness, cv.LINE_8, hierarchy, 0)
    return drawing



def sample_function(params, values_range, resolution = 1):
    start, stop = values_range
    xs = np.linspace(start, stop, (stop - start) // resolution)
    return xs, np.poly1d(params)(xs)


def polyfit_find(img):
    white_points = np.where(img == 255)[::-1]
    y_range = np.min(white_points[0]), np.max(white_points[0])

    params = np.polyfit(*white_points, deg=2, full=True)
    params, residuals = params[:2]

    cords = np.array(sample_function(params, y_range)).astype(int).T

    greater_zero_filter = np.all(np.zeros((2,)) <= cords, axis=1)
    less_bound_filter = np.all(cords < img.shape[::-1], axis=1)
    cords = cords[greater_zero_filter & less_bound_filter]

    return cords[:, 0], cords[:, 1], residuals


def main():
    img = cv.imread(TEST_IMAGE)

    mask = cv.inRange(img, *LINE_COLOR_RANGE)
    mask = clean_mask(mask)
    cv.imshow("mask", mask)

    contours, hierarchy = find_contours(mask)
    regions = [draw_contour(mask.shape, contours, hierarchy, i, cv.FILLED) for i in range(len(contours))]

    for region in regions:
        x, y, residuals = polyfit_find(region)
        print(residuals)

        img[y, x] = (255, 0, 0)

    cv.imshow("img", img)

    while cv.waitKey(0) != 27: pass


if __name__ == "__main__":
    main()
