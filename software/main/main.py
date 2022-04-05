import time
import logging
import coloredlogs

from cv2 import cv2 as cv
import numpy as np

from simple_pid import PID

from vision import colors, line
from vision.camera import BufferlessCapture
from vision.intersection import MarkersPosition

from .robot import Robot
from .intersections import IntersectionsHandler
from .intersections import Action as IntersectionAction

field_styles = coloredlogs.DEFAULT_FIELD_STYLES
field_styles["levelname"] = {"color": "white", "bold": True}
coloredlogs.install(fmt="%(asctime)s - %(threadName)s - %(levelname)s - %(module)s - %(message)s",
                    field_styles=field_styles,
                    level=logging.DEBUG)

LOOP_INTERVAL = 1 / 15

FOLLOWING_SPEED = 500  # sps
MAX_SPEED = 600
LINE_TARGET_X = 228
INTERSECTION_FILL_FRAC = 0.7


def debug(line_x, img, mask):
    if line_x is not None:
        img[:, line_x] = (255, 0, 0)

    cv.imshow("i", img)
    cv.imshow("m", mask)


def clamp_speed(val: float) -> float:
    return int(min(MAX_SPEED, max(-MAX_SPEED, val)))


def can_follow_line(mask, window_pos, line_x):
    if window_pos is None and line_x is None:
        return False

    window = mask[window_pos[0]:window_pos[1]]
    window_area = window.shape[0] * window.shape[1]
    filled_frac = np.count_nonzero(window) / window_area
    return filled_frac < INTERSECTION_FILL_FRAC


class RobotController:
    def __init__(self) -> None:
        self._robot = Robot()
        self._cap = BufferlessCapture(0)
        self._is_following_line = True

        self._pid = PID(Kp=20.0, Ki=0.0, Kd=0.0, setpoint=0.0,
                        output_limits=(-FOLLOWING_SPEED * 2,
                                       FOLLOWING_SPEED * 2))

    def loop(self):
        intersections = IntersectionsHandler()

        while True:
            start = time.time()

            frame = self._cap.read()
            frame = cv.resize(frame,
                              (frame.shape[1] // 2, frame.shape[0] // 2))

            cropped = frame[:(frame.shape[1] // 2 - 5), :]
            mask = colors.find_black(cropped)

            window_pos = line.get_window_pos(mask)
            line_x = line.find(mask, window_pos)

            debug(line_x, cropped, mask)

            if can_follow_line(mask, window_pos, line_x):
                offset = line_x - LINE_TARGET_X
                correction = self._pid(offset) or 0

                new_speed = (clamp_speed(FOLLOWING_SPEED - correction),
                             clamp_speed(FOLLOWING_SPEED + correction))
                logging.debug(f"err: {offset} ; correction: {correction} ; new speed: {new_speed}")

            dt = time.time() - start
            delay = int((LOOP_INTERVAL - dt) * 1000)
            logging.debug(f"loop delay: {delay}")
            if delay > 0:
                cv.waitKey(delay)

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
