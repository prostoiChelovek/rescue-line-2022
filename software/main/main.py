import time
import logging, coloredlogs

from cv2 import cv2 as cv

from vision.camera import BufferlessCapture

from .robot import Robot
from .line_following import LineFollower
from .intersections import IntersectionsHandler

field_styles = coloredlogs.DEFAULT_FIELD_STYLES
field_styles["levelname"] = {"color": "white", "bold": True}
coloredlogs.install(fmt="%(asctime)s - %(threadName)s - %(levelname)s - %(module)s - %(message)s",
                    field_styles=field_styles,
                    level=logging.DEBUG)

LOOP_INTERVAL = 1 / 15


class RobotController:
    def __init__(self) -> None:
        self._robot = Robot()
        self._cap = BufferlessCapture(0)

    def loop(self):
        follower = LineFollower()
        intersections = IntersectionsHandler()
        while True:
            start = time.time()

            frame = self._cap.read()
            frame = cv.resize(frame, (frame.shape[1] // 2, frame.shape[0] // 2))

            new_speed, black_win, line_x = follower.update(frame)
            self._robot.set_speed(*map(lambda x: -x, new_speed))

            intersections.update(frame, line_x, black_win)

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
