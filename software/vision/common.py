import numpy as np
from cv2 import cv2 as cv


def clean_mask(mask):
    kernel_erote = np.ones((3, 3), np.uint8)
    erosion = cv.erode(mask, kernel_erote, iterations=1)

    kernel_dilate = np.ones((9, 9), np.uint8)
    dilation = cv.dilate(erosion, kernel_dilate, iterations=2)

    return dilation
