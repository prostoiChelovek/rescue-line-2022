import os

NO_MOVEMENT = bool(int(os.getenv("NO_MOVE", default=0)))

LOOP_INTERVAL = 1 / 10

FOLLOWING_SPEED = 300  # sps
MAX_SPEED = 500
LINE_TARGET_X = 175 - 52

SERIAL_PORT =  "/dev/ttyACM0"

STEPS_PER_REV = 16 * 200
