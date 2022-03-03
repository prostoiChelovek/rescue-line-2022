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


def main():
    img = cv.imread(TEST_IMAGE)
    cv.imshow("test", img)

    mask = cv.inRange(img, *LINE_COLOR_RANGE)
    mask = clean_mask(mask)
    cv.imshow("mask", mask)

    # contours, hierarchy = find_contours(mask)
    # contours_img = draw_contours(mask.shape, contours, hierarchy, thickness=cv.FILLED)

    x, y = np.where(mask == 255)
    print(x, y)
    params = np.polyfit(y, x, 2)
    print(params)
    a, b, c = params

    start, stop = np.min(y), np.max(y)
    xs = np.linspace(start, stop, stop - start)
    ys = a * (xs ** 2) + b * xs + c
    for x_, y_ in zip(xs, ys):
        if y_ < img.shape[1]:
            img[int(y_), int(x_)] = (255, 0, 0)

    # thin = cv.ximgproc.thinning(mask)

    # cv.imshow("contours", contours_img)
    cv.imshow("img", img)

    while cv.waitKey(0) != 27: pass


if __name__ == "__main__":
    main()
