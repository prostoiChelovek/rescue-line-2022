import aioserial

from .interfacing_py import Interfacing, Command, SetSpeedParams, PyCommand, MessageBuffer


class InterfacingManager(Interfacing):
    def __init__(self, port: str) -> None:
        super().__init__(self)

        self._serial = aioserial.AioSerial(port)

