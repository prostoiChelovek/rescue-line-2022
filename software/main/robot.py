import asyncio
import threading
import concurrent.futures
import time
from typing import Optional
import logging
import math

from interfacing_py import InterfacingManager, PyCommand, Command, SetSpeedParams

SERIAL_PORT =  "/dev/ttyS0"
MAX_SPEED_DIFFERENCE = 1000


def make_closer_to_zero(num: int, decrement: int) -> int:
    return int(math.copysign(abs(num) - decrement, num))


class Robot:
    def __init__(self) -> None:
        self.steps_per_rev = 16 * 200

        self._loop = asyncio.new_event_loop()
        interfacing_future = concurrent.futures.Future()
        self._loop_th = threading.Thread(target=Robot._background_loop,
                                         args=(self._loop, interfacing_future),
                                         daemon=True)
        self._loop_th.start()

        self._interfacing = interfacing_future.result(timeout=1)

    def shutdown(self):
        try:
            self.stop(timeout=1)
        except concurrent.futures.TimeoutError:
            logging.error("Cannot stop the robot")

        self._loop.call_soon_threadsafe(self._interfacing.stop)
        time.sleep(0.1)
        self._loop.call_soon_threadsafe(self._loop.stop)
        self._loop_th.join()

    @staticmethod
    def _background_loop(loop: asyncio.AbstractEventLoop,
                         interfacing: concurrent.futures.Future) -> None:
        asyncio.set_event_loop(loop)
        interfacing.set_result(InterfacingManager(SERIAL_PORT, loop))
        loop.run_forever()

    def set_speed(self, left: int, right: int, timeout: Optional[float] = None):
        diff = abs(left - right)
        if diff > MAX_SPEED_DIFFERENCE:
            diff_excess = diff - MAX_SPEED_DIFFERENCE
            # TODO: do it like a cool guy (without branching)
            if left == 0:
                right = make_closer_to_zero(right, diff_excess)
            elif right == 0:
                left = make_closer_to_zero(left, diff_excess)
            else:
                correction = diff_excess // 2
                left = make_closer_to_zero(left, correction)
                right = make_closer_to_zero(right, correction)
            logging.debug(f"Speed diff too big ({diff}); Corrected: ({left}, {right})")

        self._execute_command(PyCommand(Command.SetSpeed,
                              SetSpeedParams(left, right)),
                              timeout)

    def stop(self, timeout: Optional[float] = None):
        self._execute_command(PyCommand(Command.Stop), timeout)

    def _execute_command(self, cmd: PyCommand, timeout: Optional[float] = None):
        fut = asyncio.run_coroutine_threadsafe(
                        self._command_future_wrapper(cmd),
                        self._loop)
        fut.result(timeout=timeout)

    async def _command_future_wrapper(self, cmd: PyCommand):
        await self._interfacing.execute(cmd)

    def _to_steps(self, speed: float) -> int:
        return int(speed * self.steps_per_rev)
