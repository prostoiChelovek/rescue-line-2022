import os

import cv2 as cv
import numpy as np

from .common import clean_mask, flatten, group

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
ALL_RANGES = [
              BLACK_COLOR_RANGE,
              GREEN_COLOR_RANGE,
              SILVER_COLOR_RANGE,
              OBSTACLE_COLOR_RANGE
              ]
RANGE_NAMES = ("black", "green", "silver", "obstacle")


def _copy_all_ranges():
    # who needs references when you can just do this shit

    global BLACK_COLOR_RANGE, GREEN_COLOR_RANGE
    global SILVER_COLOR_RANGE, OBSTACLE_COLOR_RANGE

    (BLACK_COLOR_RANGE,
     GREEN_COLOR_RANGE,
     SILVER_COLOR_RANGE,
     OBSTACLE_COLOR_RANGE) = ALL_RANGES

def load_color_ranges(path: str):
    global ALL_RANGES

    data = np.loadtxt(path)

    ALL_RANGES = group(data, 2)
    _copy_all_ranges()


def save_color_ranges(path: str):
    data = flatten(ALL_RANGES)
    _copy_all_ranges()
    np.savetxt(path, data, fmt="%i")


def find_black(img) -> cv.Mat:
    return find_color(img, BLACK_COLOR_RANGE)


def find_green(img) -> cv.Mat:
    return find_color(img, GREEN_COLOR_RANGE)


def find_silver(img) -> cv.Mat:
    return find_color(img, SILVER_COLOR_RANGE)


def find_obstacle(img) -> cv.Mat:
    return find_color(img, OBSTACLE_COLOR_RANGE)


def find_color(img: cv.Mat, range_) -> cv.Mat:
    img = cv.cvtColor(img, cv.COLOR_BGR2LAB)
    mask = cv.inRange(img, *range_)
    return clean_mask(mask)


def main():
    global BLACK_COLOR_RANGE, GREEN_COLOR_RANGE
    global SILVER_COLOR_RANGE, OBSTACLE_COLOR_RANGE

    script_dir = os.path.dirname(os.path.realpath(__file__))
    color_ranges_path = os.path.abspath(os.path.join(script_dir, "../colors.csv"))

    load_color_ranges(color_ranges_path)

    cap = cv.VideoCapture(0)

    current_range = 0
    while True:
        name = "mask"
        cv.namedWindow(name)

        trackbars = [f"{n} {e}" for e in ("min", "max") for n in ("L", "A", "B")]
        for v, t in zip(flatten(ALL_RANGES[current_range]), trackbars):
            cv.createTrackbar(t, name, int(v), 255, lambda _: None)

        while True:
            _, frame = cap.read()

            values = list(map(lambda t: cv.getTrackbarPos(t, name), trackbars))
            ALL_RANGES[current_range] = group(values, 3)
            _copy_all_ranges()

            mask = find_color(frame, ALL_RANGES[current_range])

            both = cv.bitwise_and(frame, frame, mask=mask)
            cv.putText(frame,
                       RANGE_NAMES[current_range],
                       org=(30, 50),
                       fontFace=cv.FONT_HERSHEY_COMPLEX,
                       fontScale=2.0,
                       color=(255, 0, 0),
                       thickness=2)

            cv.imshow("frame", frame)
            cv.imshow("mask", mask)
            cv.imshow(RANGE_NAMES[current_range], both)

            key = cv.waitKey(1)
            if key == 27:  # esc
                save_color_ranges(color_ranges_path)
                return
            elif key == ord("n"):
                current_range = (current_range + 1) % len(ALL_RANGES)
                cv.destroyAllWindows()
                break


if __name__ == "__main__":
    main()
