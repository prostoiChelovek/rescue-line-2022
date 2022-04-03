from typing import Optional, Tuple
from enum import Enum

from vision import colors, intersection
from vision.intersection import MarkersPosition


class Action(Enum):
    GO_FORWARD = 0
    TURN_LEFT = 1
    TURN_RIGHT = 2
    TURN_AROUND = 3


class IntersectionsHandler:
    class State(Enum):
        WAITING = 0
        STARTED = 1
        SCANNED = 2

    def __init__(self) -> None:
        self._state = self.State.WAITING
        self._marks_pos_history = []

    def update(self, frame, line_x: int,
              window_pos: Tuple[int, int]) bool:
        green = colors.find_green(frame)
        marks_position = intersection.find(green, line_x, window_pos)
        
        if marks_position != MarkersPosition.NONE \
                and self._state == self.State.WAITING:
            self._state = self.State.STARTED

        return self._update_hisotry(marks_position)

    def finish_scanning(self):
        pos = self._reduce_hisotry()

        self._state = self.State.SCANNED
        self._marks_pos_history.clear()

        return pos

    def _update_hisotry(self, pos: MarkersPosition) -> bool:
        if pos == MarkersPosition.NONE:
            return True
        else:
            previos_pos = self._marks_pos_history[-1:]
            if [pos] != previos_pos:
                self._marks_pos_history.append(pos)
            return False

    def _reduce_hisotry(self) -> MarkersPosition:
        historty = set(self._marks_pos_history)
        historty_sum = sum(map(lambda x: x.value, historty))
        for pos in sorted(MarkersPosition,
                          key=lambda x: x.value,
                          reverse=True):
            if historty_sum >= pos.value:
                return pos

        # unreachable; just to silence pyright
        return MarkersPosition.NONE
