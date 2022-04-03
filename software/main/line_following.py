from typing import Tuple
import logging

from cv2 import cv2 as cv
from simple_pid import PID

from vision import colors, line

FOLLOWING_SPEED = 500  # sps
MAX_SPEED = 600
LINE_TARGET_X = 228


def debug(line_x, img, mask):
    if line_x is not None:
        img[:, line_x] = (255, 0, 0)

    cv.imshow("i", img)
    cv.imshow("m", mask)


def get_line_offset(img) -> int:
    cropped = img[:(img.shape[1] // 2 - 5), :]
    mask = colors.find_black(cropped)
    line_x = line.find(mask)

    debug(line_x, cropped, mask)

    if line_x is None:
        return img.shape[1]  # TODO: handle it properly
    return line_x - LINE_TARGET_X


def clamp_speed(val: float) -> float:
    return int(min(MAX_SPEED, max(-MAX_SPEED, val)))


class LineFollower:
    def __init__(self) -> None:
        self._pid = PID(Kp=20.0, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED * 2,
                                       FOLLOWING_SPEED * 2))

    def update(self, img) -> Tuple[float, float]:
        offset = get_line_offset(img)
        correction = self._pid(offset)

        new_speed = (clamp_speed(FOLLOWING_SPEED - correction),
                     clamp_speed(FOLLOWING_SPEED + correction))
        logging.debug(f"err: {offset} ; correction: {correction} ; new speed: {new_speed}")
        return new_speed
