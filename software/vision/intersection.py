import numpy as np
import cv2.cv2 as cv

TEST_IMAGE = "./intersection-images/0.jpg"


def filter_green(img):
    b, g, r = cv.split(img)

    img[np.where(b >= 140)] = 0
    img[np.where(r >= 50)] = 0
    img[np.where(g < 95)] = 0


def main():
    img = cv.imread(TEST_IMAGE)
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))
    cv.imshow("orig", img)

    filter_green(img)

    cv.imshow("img", img)

    while cv.waitKey(0) != 27:  # esc
        pass


if __name__ == "__main__":
    main()
