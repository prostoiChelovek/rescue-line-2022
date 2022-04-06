import os

NO_MOVEMENT = bool(int(os.getenv("NO_MOVE", default=0)))

LOOP_INTERVAL = 1 / 10

FOLLOWING_SPEED = 80  # sps
MAX_SPEED = 120
LINE_TARGET_X = 120
INTERSECTION_FILL_FRAC = 0.7

INTERSECTION_FORWARD_TIME = 9
TURN_TIME = 10.5

RECOVERY_OFFSET = 40
RECOVERY_TARGET_OFFSET = 4

SERIAL_PORT =  "/dev/ttyACM0"

STEPS_PER_REV = 16 * 200

LINE_WIDTH = 10

MAYBE_INTERSECTION_FILL_DIFF = 1.3
