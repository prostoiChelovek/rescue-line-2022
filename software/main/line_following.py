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


def clamp_speed(val: float) -> float:
    return int(min(MAX_SPEED, max(-MAX_SPEED, val)))


class LineFollower:
    def __init__(self) -> None:
        self._pid = PID(Kp=20.0, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED * 2,
                                       FOLLOWING_SPEED * 2))

    def update(self, img) -> Tuple[Tuple[float, float], Tuple[int, int], int]:
        cropped = img[:(img.shape[1] // 2 - 5), :]
        mask = colors.find_black(cropped)

        window_pos = line.get_window_pos(mask)
        line_x = line.find(mask, window_pos)

        debug(line_x, cropped, mask)

        offset = img.shape[1]  # TODO: handle it properly     offset, window_pos, line_x = get_line_offset(img)
        if line_x is not None:
            offset = line_x - LINE_TARGET_X

        correction = self._pid(offset)

        new_speed = (clamp_speed(FOLLOWING_SPEED - correction),
                     clamp_speed(FOLLOWING_SPEED + correction))
        logging.debug(f"err: {offset} ; correction: {correction} ; new speed: {new_speed}")
        # TODO: should not return all of that from here
        return new_speed, window_pos, line_x
