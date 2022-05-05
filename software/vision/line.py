import cv2 as cv

from .colors import find_black
from .window import Window
from .common import get_fill_frac, is_mat_empty, lower_row, upper_row


def validate_window(win: cv.Mat) -> bool:
    return all([
        not is_mat_empty(lower_row(win)),
        not is_mat_empty(upper_row(win)),
        get_fill_frac(win) < 0.5,
        ])


def main():
    from . import colors
    colors.BLACK_COLOR_RANGE =  (
            (0, 0, 0),
            (120, 120, 120)
            )

    img = cv.imread("./vision/images/intersection/4.jpg")
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))

    mask = find_black(img)
    lower = Window(mask, 0)
    upper = lower + 3

    lower.draw(img)
    upper.draw(img)

    cv.imshow("mask", mask)
    cv.imshow("lower", lower.roi)
    cv.imshow("upper", upper.roi)
    cv.imshow("img", img)
    while cv.waitKey(0) != 27:
        pass


if __name__ == "__main__":
    main()
