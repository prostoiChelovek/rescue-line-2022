import numpy as np
from cv2 import cv2 as cv

from common import clean_mask

TEST_IMAGE = "./images/0.jpg"
LINE_COLOR_RANGE = (
        (0, 0, 0),
        (50, 50, 65)
        )


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
