from __future__ import annotations
from typing import List, Union

import cv2 as cv
import skimage.measure
import numpy as np

from .common import draw_horizontal_line

WINDOW_HEIGHT = 5

RegionProperties = skimage.measure._regionprops.RegionProperties


def win2px(pos: float) -> int:
    return round(pos * WINDOW_HEIGHT)


def px2win(px: int) -> float:
    return px / WINDOW_HEIGHT


def windows_in_image(img: cv.Mat) -> float:
    return px2win(img.shape[0]) - 1


class Window:
    def __init__(self, img: cv.Mat, start: float) -> None:
        self.img = img
        self.pos = start

        if self.start < 0 or self.end > self.img.shape[0]:
            raise ValueError("out of bounds")

    @property
    def end(self) -> int:
        return self.img.shape[0] - win2px(self.pos)

    @property
    def start(self) -> int:
        return self.end - WINDOW_HEIGHT

    @property
    def roi(self) -> cv.Mat:
        return self.img[self.start:self.end, :]

    @property
    def regions(self) -> List[RegionProperties]:
        labels = skimage.measure.label(self.roi)
        return skimage.measure.regionprops(label_image=labels)

    def draw(self, img: cv.Mat, color = (0, 0, 255)):
        mid = img.shape[1] // 2
        cv.line(img, (mid, self.start), (mid, self.end), color)
        draw_horizontal_line(img, self.start - 1, color)
        draw_horizontal_line(img, self.end, color)

    def __repr__(self) -> str:
        return f"Window(pos={self.pos}, start={self.start}, end={self.end})"

    def __eq__(self, o: Union[Window, None]) -> bool:
        if isinstance(o, Window):
            return self.pos == o.pos and np.array_equal(self.img, o.img)
        if o is None:
            return False
        else:
            raise NotImplementedError
