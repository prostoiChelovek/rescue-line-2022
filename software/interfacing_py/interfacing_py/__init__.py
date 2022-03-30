import asyncio
from typing import Dict
import aioserial
import logging

from .interfacing_py import Interfacing, Command, CommandId, SetSpeedParams, PyCommand, MessageBuffer


class InterfacingManager:
    def __init__(self, port: str) -> None:
        self._interfacing = Interfacing()

        self._logger = logging.Logger(__name__)
        self._serial = aioserial.AioSerial(port, baudrate=self._interfacing.BAUD_RATE)
        self._command_futures: Dict[CommandId, asyncio.Future] = {}

        asyncio.create_task(self._updater())
        asyncio.create_task(self._sender())
        asyncio.create_task(self._retry_timed_out())

    async def _updater(self):
        while True:
            try:
                bytes = await self._serial.read_async(size = 1)
                self._interfacing.handle_received_byte(bytes[0])

                for handle, future in list(self._command_futures.items()):
                    if self._interfacing.is_finished(handle):
                        future.set_result(None)
                        self._interfacing.ack_finish(handle)
                        del self._command_futures[handle]
            except Exception:
                self._logger.exception("Error while running update loop")

    async def _sender(self):
        while True:
            try:
                to_send = self._interfacing.get_message_to_send()
                if to_send is not None:
                    print(list(to_send))
                    await self._serial.write_async(bytes(to_send))
                await asyncio.sleep(0.01)
            except Exception:
                self._logger.exception("Error while running send loop")

    async def _retry_timed_out(self):
        while True:
            try:
                self._interfacing.retry_timed_out()
                await asyncio.sleep(0.01)
            except Exception:
                self._logger.exception("Error while retrying timed out commands")


    def execute(self, cmd: PyCommand) -> asyncio.Future:
        handle = self._interfacing.execute(cmd)
        future = asyncio.get_event_loop().create_future()
        self._command_futures[handle] = future
        return self._command_futures[handle]


__all__ = [
        "InterfacingManager",
        "Interfacing", "Command", "CommandId", "SetSpeedParams", "PyCommand", "MessageBuffer"
        ]
