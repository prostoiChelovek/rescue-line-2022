import asyncio
from typing import Dict
import aioserial
import logging

from .interfacing_py import Interfacing, Command, CommandId, SetSpeedParams, PyCommand, MessageBuffer


class InterfacingManager(Interfacing):
    def __init__(self, port: str) -> None:
        super().__init__()

        self._logger = logging.Logger(__name__)
        self._serial = aioserial.AioSerial(port, baudrate=self.BAUD_RATE)
        self._command_futures: Dict[CommandId, asyncio.Future] = {}

        asyncio.create_task(self._updater())

    async def _read_message(self):
        await self._serial.read_until_async(self.START_BYTE)
        len = await self._serial.read_async(size = 1)
        return await self._serial.read_async(size = int(len))

    async def _updater(self):
        while True:
            try:
                message = await self._read_message()
                self.set_received_message(MessageBuffer(message))

                self.update()

                for handle, future in list(self._command_futures.items()):
                    if self.check_finished(handle):
                        future.set_result(None)
                        self.ack_finish(handle)
                        del self._command_futures[handle]

                to_send = self.get_message_to_send()
                if to_send is not None:
                    await self._serial.write_async(bytes(to_send))
            except Exception:
                self._logger.exception("Error while running update loop")

    def execute(self, cmd: PyCommand) -> asyncio.Future:
        handle = super().execute(self, cmd)
        future = asyncio.get_event_loop().create_future()
        self._command_futures[handle] = future
        return self._command_futures[handle]


__all__ = [
        "InterfacingManager",
        "Interfacing", "Command", "CommandId", "SetSpeedParams", "PyCommand", "MessageBuffer"
        ]
