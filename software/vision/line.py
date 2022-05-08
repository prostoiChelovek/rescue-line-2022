from __future__ import annotations

import math
from typing import Iterator, List, Optional, Tuple, Union
from dataclasses import dataclass

import cv2 as cv
import numpy as np

from .colors import find_black
from .window import RegionProperties, Window, windows_in_image
from .common import draw_angled_line, get_fill_frac, is_mat_empty, lower_row, upper_row

LINE_WINDOW_STEP = 0.5
LINE_WINDOW_FIRST_MAX_OFFSET = 5.0
LINE_WINDOWS_DISTANCE_RANGE = (2.0, 10.0)
MAX_REGIONS_DISTANCE = 30  # shortest distance


def arange_offset(start: float, offset: float, step: float, include_end: bool = False) -> np.ndarray:
    if step > 0:
        end = start + offset
    elif step < 0:
        if offset > start:
            raise ValueError(f"offset is too big ({start=}, {offset=})")
        end = start - offset
    else:
        raise ValueError("step is 0")
    if include_end:
        end += step
    return np.arange(start, end, step)


@dataclass
class WindowPair:
    lower: Optional[Window]
    upper: Optional[Window]

    @staticmethod
    def empty() -> WindowPair:
        return WindowPair(None, None)

    @property
    def is_complete(self) -> bool:
        return all(x is not None for x in self)

    def draw(self, img: cv.Mat) -> None:
        if self.lower is not None:
            self.lower.draw(img, (0, 0, 255))
        if self.upper is not None:
            self.upper.draw(img, (0, 255, 255))

    def __iter__(self) -> Iterator[Optional[Window]]:
        return iter((self.lower, self.upper))


@dataclass
class LineInfo:
    x_offset: int
    angle: Optional[float]

    def draw(self, img: cv.Mat) -> None:
        draw_angled_line(img,
                x1=img.shape[1] // 2 + self.x_offset,
                y1=img.shape[0],
                y2=0,
                angle=self.angle or 0,
                color=(255, 0, 255),
                thickness=2
                )


def validate_window(win: Union[cv.Mat, Window]) -> bool:
    if isinstance(win, Window):
        win = win.roi
    return all([
        not is_mat_empty(lower_row(win)),
        not is_mat_empty(upper_row(win)),
        get_fill_frac(win) < 0.2,
        ])


def find_window(img: cv.Mat,
                start: float = 0,
                max_offset: Optional[float] = None,
                step: Optional[float] = None) -> Optional[Window]:
    max_offset  = max_offset or windows_in_image(img)
    step = step or 1.0

    for pos in arange_offset(start, max_offset, step, include_end=True):
        win = Window(img, pos)
        if validate_window(win):
            return win
    return None


def find_line_window_pair(img: cv.Mat) -> WindowPair:
    res = WindowPair.empty()

    res.lower = find_window(img,
                            start=0.0,
                            max_offset=LINE_WINDOW_FIRST_MAX_OFFSET,
                            step=LINE_WINDOW_STEP)
    if res.lower is None:
        return res

    max_distance = res.lower.pos + 1 + LINE_WINDOWS_DISTANCE_RANGE[1]
    min_distance_offset = LINE_WINDOWS_DISTANCE_RANGE[1] \
                            - LINE_WINDOWS_DISTANCE_RANGE[0]
    res.upper = find_window(img,
                            start=max_distance,
                            max_offset=min_distance_offset,
                            step=-LINE_WINDOW_STEP)

    return res


def get_best_region(regions: List[RegionProperties]) -> RegionProperties:
    # prefers bigger regions closer to left
    return max(regions,
               key=lambda r: 1 / math.sqrt(r.area) + 1 / r.centroid[1])


def reduce_region(region: RegionProperties) -> int:
    return round(region.centroid[1])


def region_width(reg: RegionProperties) -> int:
    start_x, end_x = reg.bbox[1], reg.bbox[3]
    return end_x - start_x


def bound_middle(bound: Tuple[int, int]) -> int:
    return bound[0] + (bound[1] - bound[0]) // 2


def bounds_distance(a: Tuple[int, int], b: Tuple[int, int]) -> int:
    return min(map(lambda bounds: min(map(abs, 
                                          (
                                              bounds[0][1] - bounds[1][0],
                                              bounds[0][0] - bounds[1][0],
                                              bounds[0][1] - bounds[1][1],
                                              bound_middle(bounds[0]) - bound_middle(bounds[1]),
                                          )
                                          )
                                      ),
                   ((a, b), (b, a))
                   )
               )


def regions_distance(a: RegionProperties, b: RegionProperties) -> int:
    bound_a, bound_b = (a.bbox[1], a.bbox[3]), (b.bbox[1], b.bbox[3])
    return bounds_distance(bound_a, bound_b)


def get_matching_regions(wins: WindowPair) -> List[int]:
    if wins.lower is None and wins.upper is None:
        return []
    elif wins.lower is not None and wins.upper is None:
        return [reduce_region(get_best_region(wins.lower.regions))]
    elif wins.lower is not None and wins.upper is not None:
        lower_regions = wins.lower.regions[:]
        while len(lower_regions) > 0:
            lower_region = get_best_region(lower_regions)

            upper_regions = wins.upper.regions[:]
            while len(upper_regions) > 0:
                upper_region = get_best_region(upper_regions)
                distance = regions_distance(lower_region, upper_region)
                if distance < MAX_REGIONS_DISTANCE:
                    return list(map(reduce_region, (lower_region, upper_region)))
                else:
                    upper_regions.remove(upper_region)
            else:
                lower_regions.remove(lower_region)
        else:  # no matches found
            return [reduce_region(get_best_region(wins.lower.regions))]


def locate_line(wins: WindowPair) -> LineInfo:
    if wins.lower is None and wins.upper is None:
        raise ValueError("Both windows are empty")

    regions_x = get_matching_regions(wins)
    if len(regions_x) == 0:
        raise ValueError("No matching reigons found")

    img_width = wins.lower.img.shape[1]
    x_offset = regions_x[0] - img_width // 2

    angle = None
    if len(regions_x) == 2:
        x_distance = regions_x[1] - regions_x[0]
        y_distance = wins.lower.start - wins.upper.end
        angle = math.atan(x_distance / y_distance)

    return LineInfo(x_offset, angle)


def main():
    from . import colors
    colors.BLACK_COLOR_RANGE =  (
            (0, 0, 0),
            (120, 120, 120)
            )

    img = cv.imread("./vision/images/intersection/4.jpg")
    img = cv.resize(img, (img.shape[1] // 4, img.shape[0] // 4))

    mask = find_black(img)
    wins = find_line_window_pair(mask)
    if wins == WindowPair.empty():
        print("no window")
        return

    wins.draw(img)

    line = locate_line(wins)
    print(line)
    line.draw(img)

    cv.imshow("mask", mask)
    cv.imshow("img", img)
    while cv.waitKey(0) != 27:
        pass


if __name__ == "__main__":
    main()
