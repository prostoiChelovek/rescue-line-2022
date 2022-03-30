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
        asyncio.create_task(self._sender())

    async def _updater(self):
        while True:
            try:
                byte = await self._serial.read_async(size = 1)
                self.handle_received_byte(byte)

                for handle, future in list(self._command_futures.items()):
                    if self.check_finished(handle):
                        future.set_result(None)
                        self.ack_finish(handle)
                        del self._command_futures[handle]
            except Exception:
                self._logger.exception("Error while running update loop")

    async def _sender(self):
        while True:
            try:
                to_send = self.get_message_to_send()
                if to_send is not None:
                    print(list(to_send))
                    await self._serial.write_async(bytes(to_send))
                await asyncio.sleep(0.1)
            except Exception:
                self._logger.exception("Error while running send loop")

    def execute(self, cmd: PyCommand) -> asyncio.Future:
        handle = super().execute(cmd)
        future = asyncio.get_event_loop().create_future()
        self._command_futures[handle] = future
        return self._command_futures[handle]


__all__ = [
        "InterfacingManager",
        "Interfacing", "Command", "CommandId", "SetSpeedParams", "PyCommand", "MessageBuffer"
        ]
