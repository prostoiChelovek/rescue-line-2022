import asyncio
import threading
import concurrent.futures

from interfacing_py import InterfacingManager, PyCommand, Command, SetSpeedParams

SERIAL_PORT =  "/dev/ttyACM0"


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

    def stop(self):
        self._interfacing.stop()
        self._loop.call_soon_threadsafe(self._loop.stop)
        self._loop_th.join()

    @staticmethod
    def _background_loop(loop: asyncio.AbstractEventLoop,
                         interfacing: concurrent.futures.Future) -> None:
        asyncio.set_event_loop(loop)
        interfacing.set_result(InterfacingManager(SERIAL_PORT, loop))
        loop.run_forever()

    def set_speed(self, left: float, right: float):
        params = SetSpeedParams(self._to_steps(left), self._to_steps(right))
        
        fut = asyncio.run_coroutine_threadsafe(
                        self._run_command(PyCommand(Command.SetSpeed, params)),
                        self._loop)
        fut.result()

    async def _run_command(self, cmd: PyCommand):
        await self._interfacing.execute(cmd)

    def _to_steps(self, speed: float) -> int:
        return int(speed * self.steps_per_rev)
