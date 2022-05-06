from __future__ import annotations

import cv2 as cv

from .common import draw_horizontal_line

WINDOW_HEIGHT = 5


def win2px(pos: float) -> int:
    return round(pos * WINDOW_HEIGHT)


def px2win(px: int) -> float:
    return px / WINDOW_HEIGHT


def windows_in_image(img: cv.Mat) -> float:
    return px2win(img.shape[0]) - 1


class Window:
    def __init__(self, img: cv.Mat, start: float) -> None:
        self.img = img
        self.end = img.shape[0] - win2px(start)
        self.start = self.end - WINDOW_HEIGHT

    @property
    def roi(self) -> cv.Mat:
        return self.img[self.start:self.end, :]

    def draw(self, img: cv.Mat, color = (0, 0, 255)):
        draw_horizontal_line(img, self.start - 1)
        draw_horizontal_line(img, self.end)
        mid = img.shape[1] // 2
        cv.line(img, (mid, self.start), (mid, self.end), color)

    def __str__(self) -> str:
        return f"Window(start={self.start}, end={self.end})"

    def __add__(self, offset: float) -> Window:
        w = Window(self.img, 0)
        w.start = self.start - win2px(offset)
        w.end = self.end - win2px(offset)
        return w

    def __sub__(self, offset: float) -> Window:
        return self + (-offset)
