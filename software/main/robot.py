import asyncio
import threading
import concurrent.futures
import time
from typing import Optional
import logging

from interfacing_py import InterfacingManager, PyCommand, Command, SetSpeedParams

from .settings import *


class Robot:
    def __init__(self) -> None:
        self.steps_per_rev = STEPS_PER_REV

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
        if NO_MOVEMENT:
            return

        self._execute_command(PyCommand(Command.SetSpeed,
                              SetSpeedParams(left,
                                             right)),
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
