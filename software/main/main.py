from enum import Enum
import time
import logging
import coloredlogs
import functools

from cv2 import cv2 as cv
import numpy as np

from simple_pid import PID

from vision import colors, line
from vision.camera import BufferlessCapture
import vision.intersection as intersection 

from .robot import Robot
from .settings import *

field_styles = coloredlogs.DEFAULT_FIELD_STYLES
field_styles["levelname"] = {"color": "white", "bold": True}
coloredlogs.install(fmt="%(asctime)s - %(threadName)s - %(levelname)s - %(module)s - %(message)s",
                    field_styles=field_styles,
                    level=logging.DEBUG)

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

        self._pid = PID(Kp=10.0, Ki=1.0, Kd=5.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED / 2,
                                       FOLLOWING_SPEED / 2))
        self._markers_history = []
        self._last_line_x = None
        self._state = State.FOLLOWING_LINE

        self._in_line_recovery = False
        # TODO: flags suck
        self._intersection_maybe_seen = False
        self._ready_for_intersection = False

        self._intersection_type = None

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
        
        obstacle_win = frame[(frame.shape[0] // 2):]
        obstacle_mask = colors.find_obstacle(obstacle_win)
        if filled_frac(obstacle_mask) > OBSTACLE_FRAC:
            self._turn_left()
            self._obstacle_forward()
            self._turn_right()
            self._intersection_forward()
            self._turn_left()
            return

        window_pos = line.get_window_pos(black)
        line_x = line.find(black, window_pos)

        debug(line_x, frame, black)

        if window_pos is None:
            return

        if line_x is not None:
            offset = abs(line_x - LINE_TARGET_X)

            if not self._in_line_recovery:
                self._in_line_recovery = offset > RECOVERY_OFFSET
            else:
                self._in_line_recovery = offset > RECOVERY_TARGET_OFFSET

            if self._in_line_recovery:
                self._line_pid_loop(line_x, 0)
                return
            elif self._intersection_maybe_seen:
                self._ready_for_intersection = True
                self._intersection_maybe_seen = False

        black_window = black[window_pos[0]:window_pos[1]]
        black_window_fill_frac = filled_frac(black_window)

        intersection_win_pos = (window_pos[0] - LINE_WIDTH,
                                window_pos[1] - LINE_WIDTH)
        window = black[intersection_win_pos[0]:intersection_win_pos[1]]
        separator = line_x or self._last_line_x or window.shape[1] // 2
        parts = (window[:, :(separator - LINE_WIDTH // 2)],
                 window[:, (separator + LINE_WIDTH // 2):])
        parts_fill_frac = tuple(map(filled_frac, parts))

        if self._ready_for_intersection:
            self._ready_for_intersection = False
            is_filled = tuple(map(lambda f: f >= INTERSECTION_FILL_FRAC, parts_fill_frac))
            self._intersection_type = {
                    (True, False): IntersectionType.LEFT_TURN,
                    (False, True): IntersectionType.RIGHT_TURN,
                    # TODO: maybe a cross; dunno if i have to detecti it
                    (True, True): IntersectionType.T_JUNCTION
                    }.get(is_filled, None)  # type: ignore
        else:
            self._intersection_type = None
            parts_both_fill_frac = sum(parts_fill_frac)
            if (parts_both_fill_frac / black_window_fill_frac) \
                    > MAYBE_INTERSECTION_FILL_DIFF:
                self._in_line_recovery = True
                self._intersection_maybe_seen = True
                return

        if self._intersection_type is not None:
            self._intersection_forward()

            marker = intersection.MarkersPosition.NONE
            if len(self._markers_history) > 0:
                marker = max(set(self._markers_history),
                             key=self._markers_history.count)

            logging.debug(f"Marker: {marker}")

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
            self._intersection_type = None
            return

        if line_x is None:
            return

        self._last_line_x = line_x

        marks_position = intersection.find(green, line_x, intersection_win_pos)
        if marks_position != intersection.MarkersPosition.NONE:
            self._markers_history.append(marks_position)

        self._line_pid_loop(line_x, FOLLOWING_SPEED)

    def _collecting_loop(self, frame):
        pass

    @maybe_no_move
    def _intersection_backward(self):
        self._robot.set_speed(FOLLOWING_SPEED, FOLLOWING_SPEED)
        time.sleep(INTERSECTION_BACKWARD_TIME)

    @maybe_no_move
    def _intersection_forward(self):
        self._robot.set_speed(-FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(INTERSECTION_FORWARD_TIME)

    @maybe_no_move
    def _obstacle_forward(self):
        self._robot.set_speed(-FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(OBSTACLE_FORWARD_TIME)

    @maybe_no_move
    def _turn_left(self):
        self._robot.set_speed(-FOLLOWING_SPEED, FOLLOWING_SPEED)
        time.sleep(TURN_TIME)
        self._robot.set_speed(0, 0)

    @maybe_no_move
    def _turn_right(self):
        self._robot.set_speed(FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(TURN_TIME)   
        self._robot.set_speed(0, 0)

    @maybe_no_move
    def _turn_around(self):
        self._robot.set_speed(FOLLOWING_SPEED, -FOLLOWING_SPEED)
        time.sleep(TURN_TIME * 2)
        self._robot.set_speed(0, 0)

    def _line_pid_loop(self, line_x, speed):
        offset = line_x - LINE_TARGET_X
        correction = self._pid(offset) or 0

        new_speed = (-clamp_speed(speed + correction),
                     -clamp_speed(speed - correction))
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
