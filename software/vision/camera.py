import logging
import threading
import collections

from cv2 import cv2 as cv


class BufferlessCapture(threading.Thread):
    def __init__(self, name):
        super().__init__(daemon=True)

        self._cap = cv.VideoCapture(name)
        self._frame_buff = collections.deque(maxlen=1)

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
