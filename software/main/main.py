import logging, coloredlogs

from .robot import Robot

field_styles = coloredlogs.DEFAULT_FIELD_STYLES
field_styles["levelname"] = {"color": "white", "bold": True}
coloredlogs.install(fmt="%(asctime)s - %(threadName)s - %(levelname)s - %(module)s - %(message)s",
                    field_styles=field_styles)


class RobotController:
    def __init__(self) -> None:
        self._robot = Robot()

    def shutdown(self):
        self._robot.shutdown()


def main():
    robot = RobotController()

    try:
        robot._robot.set_speed(1, 1)
        while True:
            pass
    except KeyboardInterrupt:
        logging.info("Shutting down...")
        robot.shutdown()


if __name__ == "__main__":
    main()
