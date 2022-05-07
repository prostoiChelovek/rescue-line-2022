import numpy as np
import cv2 as cv


def clean_mask(mask):
    kernel_erote = np.ones((3, 3), np.uint8)
    erosion = cv.erode(mask, kernel_erote, iterations=1)

    kernel_dilate = np.ones((9, 9), np.uint8)
    dilation = cv.dilate(erosion, kernel_dilate, iterations=2)

    kernel_close = np.ones((5,5),np.uint8)
    closing = cv.morphologyEx(dilation, cv.MORPH_CLOSE, kernel_close)

    return closing


def draw_horizontal_line(img: cv.Mat, y: int):
    cv.line(img, (0, y), (img.shape[1], y), (0, 0, 255))


def draw_vertical_line(img: cv.Mat, x: int):
    cv.line(img, (x, 0), (x, img.shape[0]), (0, 0, 255))


def upper_row(mat: cv.Mat, height: int = 1) -> cv.Mat:
    return mat[0:height,:]


def lower_row(mat: cv.Mat, height: int = 1) -> cv.Mat:
    return mat[mat.shape[0] - height:,:]


def mid_row(mat: cv.Mat) -> cv.Mat:
    return mat[mat.shape[0] // 2,:]


def left_half(mat: cv.Mat) -> cv.Mat:
    return mat[:, (mat.shape[1] // 2):]


def right_half(mat: cv.Mat) -> cv.Mat:
    return mat[:, :(mat.shape[1] // 2)]


def is_mat_empty(mat: cv.Mat) -> bool:
    return bool(np.all(mat == 0))


def is_mat_filled(mat: cv.Mat) -> bool:
    return bool(np.all(mat))


def get_fill_frac(mat: cv.Mat) -> float:
    area = mat.shape[0] * mat.shape[1]
    if area == 0:
        return 0.0
    return np.count_nonzero(mat) / area
