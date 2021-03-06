from enum import Enum
import time
import logging
from typing import Optional
import coloredlogs
import functools

import cv2 as cv
import numpy as np

from simple_pid import PID
import RPi.GPIO as GPIO

from vision import colors, line
from vision.camera import BufferlessCapture
from vision.window import win2px

from .robot import Robot
from .settings import *

field_styles = coloredlogs.DEFAULT_FIELD_STYLES
field_styles["levelname"] = {"color": "white", "bold": True}
coloredlogs.install(fmt="%(asctime)s - %(threadName)s - %(levelname)s - %(module)s - %(message)s",
                    field_styles=field_styles,
                    level=logging.DEBUG)


def clamp_speed(val):
    return int(min(MAX_SPEED, max(-MAX_SPEED, val)))


class IntersectionType(Enum):
    LEFT_TURN = 0,
    RIGHT_TURN = 1,
    T_JUNCTION = 2
    CROSS = 3,


class State(Enum):
    IDLE = 0,
    FOLLOWING_LINE = 1,
    COLLECTING = 2


def maybe_no_move(fn):
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        if NO_MOVEMENT:
            logging.debug(f"Performing {fn.__name__}")
            return
        return fn(*args, **kwargs)
    return wrapper


def filled_frac(region):
    area = region.shape[0] * region.shape[1]
    if area == 0:
        return 0.0
    return np.count_nonzero(region) / area


class RobotController:
    def __init__(self) -> None:
        self._robot = Robot()
        self._cap = BufferlessCapture(0)

        self._pid = PID(Kp=1.0, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED / 2,
                                       FOLLOWING_SPEED / 2))

        self._state = State.FOLLOWING_LINE
        self._intersection_type: Optional[IntersectionType] = None

        self._can_go = False

        GPIO.setmode(GPIO.BOARD)
        GPIO.setup(37, GPIO.IN, pull_up_down=GPIO.PUD_UP)
        GPIO.add_event_detect(37, GPIO.RISING,
                              callback=self._button_handler,
                              bouncetime=1000)

    def loop(self):
        while True:
            start = time.time()

            frame = self._cap.read()
            line_win_max_y = win2px(line.LINE_WINDOW_FIRST_MAX_OFFSET + 1 \
                                    + line.LINE_WINDOWS_DISTANCE_RANGE[1] + 1)
            frame_line = frame[frame.shape[0] - line_win_max_y:, :]

            black = colors.find_black(frame_line)
            wins = line.find_line_window_pair(black)
            if wins == line.WindowPair.empty():
                pass  # TODO
            else:
                line_info = line.locate_line(wins)

                frame_half_width = frame.shape[1] // 2
                x_offset_normalized = line_info.x_offset / frame_half_width
                error = x_offset_normalized + (line_info.angle or 0)
                correction = self._pid(error) or 0

                new_speed = (-clamp_speed(FOLLOWING_SPEED + correction),
                             -clamp_speed(FOLLOWING_SPEED - correction))
                self._robot.set_speed(*new_speed)

                logging.debug(f"{line_info=} ; {error=} ; {correction=} ; {new_speed=}")

                wins.draw(frame)
                line_info.draw(frame)

            cv.imshow("frame", frame)
            cv.imshow("black", black)

            dt = time.time() - start
            delay = LOOP_INTERVAL - dt
            delay_ms = int(delay * 1000)
            if delay_ms > 0:
                cv.waitKey(delay_ms)
            else:
                logging.debug(f"loop delay: {delay}")

    def _button_handler(self, _):
        self._can_go = not self._can_go
        time.sleep(0.2)
        self._robot.stop()

    def shutdown(self):
        self._robot.shutdown()
        GPIO.cleanup()


def main():
    script_dir = os.path.dirname(os.path.realpath(__file__))
    color_ranges_path = os.path.abspath(os.path.join(script_dir, "../colors.csv"))
    logging.info(f"Loading color ranges from {color_ranges_path}")
    try:
        colors.load_color_ranges(color_ranges_path)
        colors.save_color_ranges(color_ranges_path)
    except Exception as e:
        logging.error(f"Could not load color ranges: {e}")

    robot = RobotController()

    try:
        robot.loop()
    except KeyboardInterrupt:
        logging.info("Shutting down...")
        robot.shutdown()


if __name__ == "__main__":
    main()
