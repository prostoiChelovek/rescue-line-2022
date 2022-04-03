from typing import Tuple

from cv2 import cv2 as cv
from simple_pid import PID

from vision import colors, line

FOLLOWING_SPEED = 500  # sps
MAX_SPEED = 1000
LINE_TARGET_X = 400


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
        self._current_speed: Tuple[int, int] = None
        self._pid = PID(Kp=0.0001, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED, FOLLOWING_SPEED))

    def update(self, img) -> Tuple[float, float]:
        if self._current_speed == None:
            self._current_speed = (FOLLOWING_SPEED, FOLLOWING_SPEED)
        else:
            offset = get_line_offset(img)
            correction = self._pid(offset)
            return (clamp_speed(self._current_speed[0] - correction),
                    clamp_speed(self._current_speed[1] + correction))
        return self._current_speed
