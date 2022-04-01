from typing import Tuple
from simple_pid import PID

from .robot import Robot

from vision import colors, line

FOLLOWING_SPEED = 0.8 # rps
LINE_TARGET_X = 320


def get_line_offset(img) -> int:
        mask = colors.find_black(img)
        line_x = line.find(mask)
        return line_x - LINE_TARGET_X


class LineFollower:
    def __init__(self) -> None:
        self._current_speed: Tuple[float, float]
        self._pid = PID(Kp=1.0, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED, FOLLOWING_SPEED))

    def update(self, img) -> Tuple[float, float]:
        if self._current_speed == None:
            self._current_speed = (FOLLOWING_SPEED, FOLLOWING_SPEED)
        else:
            offset = get_line_offset(img)
            correction = self._pid(offset)
            self._current_speed = (self._current_speed[0] + correction,
                                   self._current_speed[1] - correction)
        return self._current_speed
