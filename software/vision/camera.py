import logging
import threading
import collections

from cv2 import cv2 as cv

CAPTURE_RESOLUTION = (320, 240)


class BufferlessCapture(threading.Thread):
    def __init__(self, name):
        super().__init__(daemon=True)

        self._cap = cv.VideoCapture(name)
        self._frame_buff = collections.deque(maxlen=1)

        self._cap.set(cv.CAP_PROP_FRAME_WIDTH, CAPTURE_RESOLUTION[0])
        self._cap.set(cv.CAP_PROP_FRAME_HEIGHT, CAPTURE_RESOLUTION[1])
        assert self._cap.get(cv.CAP_PROP_FRAME_WIDTH) == CAPTURE_RESOLUTION[0]
        assert self._cap.get(cv.CAP_PROP_FRAME_HEIGHT) == CAPTURE_RESOLUTION[1]

        self.start()

    def run(self):
        while True:
            ret, frame = self._cap.read()
            if not ret:
                logging.error("Failed to grab a frame")
            self._frame_buff.append(frame)

    def read(self):
        while len(self._frame_buff) == 0:
            pass
        return self._frame_buff.popleft()
