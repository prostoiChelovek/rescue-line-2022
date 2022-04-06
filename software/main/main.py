from enum import Enum
import time
import logging
import coloredlogs

from cv2 import cv2 as cv
import numpy as np

from simple_pid import PID

from vision import colors, line
from vision.camera import BufferlessCapture
import vision.intersection as intersection 

from .robot import Robot

field_styles = coloredlogs.DEFAULT_FIELD_STYLES
field_styles["levelname"] = {"color": "white", "bold": True}
coloredlogs.install(fmt="%(asctime)s - %(threadName)s - %(levelname)s - %(module)s - %(message)s",
                    field_styles=field_styles,
                    level=logging.DEBUG)

LOOP_INTERVAL = 1 / 10

FOLLOWING_SPEED = 80  # sps
MAX_SPEED = 120
LINE_TARGET_X = 128
INTERSECTION_FILL_FRAC = 0.7

INTERSECTION_FORWARD_TIME = 8
TURN_TIME = 11

RECOVERY_OFFSET = 40


def debug(line_x, img, mask):
    if line_x is not None:
        img[:, line_x] = (255, 0, 0)

    cv.imshow("i", img)
    cv.imshow("m", mask)


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


class RobotController:
    def __init__(self) -> None:
        self._robot = Robot()
        self._cap = BufferlessCapture(0)

        self._pid = PID(Kp=20.0, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED * 2,
                                       FOLLOWING_SPEED * 2))
        self._markers_history = []
        self._last_line_x = None
        self._state = State.FOLLOWING_LINE

        self._in_line_recovery = False

    def loop(self):
        while True:
            start = time.time()

            frame = self._cap.read()
            frame = cv.resize(frame,
                              (frame.shape[1] // 2, frame.shape[0] // 2))

            if self._state == State.FOLLOWING_LINE:
                window = frame[(frame.shape[0] - line.WINDOW_HEIGHT):]
                window_area = window.shape[0] * window.shape[1]
                silver = colors.find_silver(window)
                if np.count_nonzero(silver) / window_area >= INTERSECTION_FILL_FRAC:
                    self._intersection_forward()
                    self._state = State.COLLECTING

                self._line_loop(frame)
            elif self._state == State.COLLECTING:
                self._collecting_loop(frame)

            dt = time.time() - start
            delay = int((LOOP_INTERVAL - dt) * 1000)
            logging.debug(f"loop delay: {delay}")
            if delay > 0:
                cv.waitKey(delay)

    def _line_loop(self, frame):
        black = colors.find_black(frame)
        green = colors.find_green(frame)

        window_pos = line.get_window_pos(black)
        line_x = line.find(black, window_pos)

        debug(line_x, frame, black)

        if window_pos is None:
            self._turn_left()
            return

        if line_x is not None:
            offset = line_x - LINE_TARGET_X
            self._in_line_recovery = abs(offset) > RECOVERY_OFFSET

            if self._in_line_recovery:
                self._line_pid_loop(line_x, 0)
                return

        window = black[window_pos[0]:window_pos[1]]
        separator = line_x or self._last_line_x or window.shape[1] // 2
        parts = (window[:, :separator], window[:, separator:])
        def is_filled(part):
            area = part.shape[0] * part.shape[1]
            filled = np.count_nonzero(part) / area
            return filled >= INTERSECTION_FILL_FRAC
        filled = tuple(map(is_filled, parts))
        intersection_type = {
                (True, False): IntersectionType.LEFT_TURN,
                (False, True): IntersectionType.RIGHT_TURN,
                # TODO: maybe a cross; dunno if i have to detecti it
                (True, True): IntersectionType.T_JUNCTION
                }.get(filled, None)

        is_on_intersection = intersection_type is not None

        if is_on_intersection:
            marker = intersection.MarkersPosition.NONE
            if len(self._markers_history) > 0:
                marker = max(set(self._markers_history),
                             key=self._markers_history.count)

            self._intersection_forward()

            if marker == intersection.MarkersPosition.NONE:
                marker = {
                        IntersectionType.LEFT_TURN: intersection.MarkersPosition.LEFT,
                        IntersectionType.RIGHT_TURN: intersection.MarkersPosition.RIGHT,
                        }.get(intersection_type, intersection.MarkersPosition.NONE)

            if marker == intersection.MarkersPosition.NONE:
                pass
            elif marker == intersection.MarkersPosition.LEFT:
                self._turn_left()
            elif marker == intersection.MarkersPosition.RIGHT:
                self._turn_right()
            elif marker == intersection.MarkersPosition.BOTH:
                self._turn_around()

            if marker != intersection.MarkersPosition.NONE:
                self._intersection_backward()
                self._in_line_recovery = True

            self._markers_history.clear()
            return

        if line_x is None:
            return

        self._last_line_x = line_x

        marks_position = intersection.find(green, line_x, window_pos)
        if marks_position != intersection.MarkersPosition.NONE:
            self._markers_history.append(marks_position)

        if not is_on_intersection:
            self._line_pid_loop(line_x, FOLLOWING_SPEED)

    def _collecting_loop(self, frame):
        pass

    def _intersection_backward(self):
        self._robot.set_speed(FOLLOWING_SPEED, FOLLOWING_SPEED)
        time.sleep(INTERSECTION_FORWARD_TIME)

    def _intersection_forward(self):
        self._robot.set_speed(-FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(INTERSECTION_FORWARD_TIME)

    def _turn_left(self):
        self._robot.set_speed(FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(TURN_TIME)
        self._robot.set_speed(0, 0)

    def _turn_right(self):
        self._robot.set_speed(-FOLLOWING_SPEED, FOLLOWING_SPEED)
        time.sleep(TURN_TIME)   
        self._robot.set_speed(0, 0)

    def _turn_around(self):
        self._robot.set_speed(FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(TURN_TIME * 2)
        self._robot.set_speed(0, 0)

    def _line_pid_loop(self, line_x, speed):
        offset = line_x - LINE_TARGET_X
        correction = self._pid(offset) or 0

        new_speed = (-clamp_speed(speed - correction),
                     -clamp_speed(speed + correction))
        self._robot.set_speed(*new_speed)

        logging.debug(f"err: {offset} ; correction: {correction} ; new speed: {new_speed}")

    def shutdown(self):
        self._robot.shutdown()


def main():
    robot = RobotController()

    try:
        robot.loop()
    except KeyboardInterrupt:
        logging.info("Shutting down...")
        robot.shutdown()


if __name__ == "__main__":
    main()
