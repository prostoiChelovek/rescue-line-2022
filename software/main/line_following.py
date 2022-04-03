from typing import Tuple
from simple_pid import PID

from .robot import Robot

from vision import colors, line

FOLLOWING_SPEED = 0.8 # rps
LINE_TARGET_X = 320
MAX_SPEED = 0.2


def get_line_offset(img) -> int:
    cropped = img[:(img.shape[1] // 2 - 5), :]
    mask = colors.find_black(cropped)
    line_x = line.find(mask)
    return line_x - LINE_TARGET_X


def clamp_speed(val: float) -> float:
    return min(MAX_SPEED, max(-MAX_SPEED, val))


class LineFollower:
    def __init__(self) -> None:
        self._pid = PID(Kp=1.0, Ki=0.0, Kd=0.0, setpoint=0.0,
        self._current_speed: Tuple[float, float] = None
                        output_limits=(-FOLLOWING_SPEED, FOLLOWING_SPEED))

    def update(self, img) -> Tuple[float, float]:
        if self._current_speed == None:
            self._current_speed = (FOLLOWING_SPEED, FOLLOWING_SPEED)
        else:
            offset = get_line_offset(img)
            correction = self._pid(offset)
            self._current_speed = (clamp_speed(self._current_speed[0] - correction),
                                   clamp_speed(self._current_speed[1] + correction))
        return self._current_speed
