import asyncio

import aioconsole
import asyncio
import contextlib
import sys
import termios

from interfacing_py import *


SPEED = 500


@contextlib.contextmanager
def raw_mode(file):
    old_attrs = termios.tcgetattr(file.fileno())
    new_attrs = old_attrs[:]
    new_attrs[3] = new_attrs[3] & ~(termios.ECHO | termios.ICANON)
    try:
        termios.tcsetattr(file.fileno(), termios.TCSADRAIN, new_attrs)
        yield
    finally:
        termios.tcsetattr(file.fileno(), termios.TCSADRAIN, old_attrs)


async def main():
    m = InterfacingManager("/dev/ttyACM0", asyncio.get_event_loop())

    with raw_mode(sys.stdin):
        reader = asyncio.StreamReader()
        loop = asyncio.get_event_loop()
        await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(reader), sys.stdin)

        while not reader.at_eof():
            ch = await reader.read(1)
            # '' means EOF, chr(4) means EOT (sent by CTRL+D on UNIX terminals)
            if not ch or ord(ch) <= 4:
                break
            if ch == b"w":
                await m.execute(PyCommand(Command.SetSpeed, SetSpeedParams(SPEED, SPEED)))
            elif ch == b"s":
                await m.execute(PyCommand(Command.SetSpeed, SetSpeedParams(-SPEED, -SPEED)))
            elif ch == b"a":
                await m.execute(PyCommand(Command.SetSpeed, SetSpeedParams(-SPEED, SPEED)))
            elif ch == b"d":
                await m.execute(PyCommand(Command.SetSpeed, SetSpeedParams(SPEED, -SPEED)))
            elif ch == b"q":
                await m.execute(PyCommand(Command.Stop))

    # print("start")

    # c = m.execute(PyCommand(Command.SetSpeed, SetSpeedParams(1000, 1000)))
    # await c
    # await asyncio.sleep(1)

    # c = m.execute(PyCommand(Command.SetSpeed, SetSpeedParams(-1000, 1000)))
    # await c
    # await asyncio.sleep(1)

    # c = m.execute(PyCommand(Command.Stop))
    # await c


asyncio.run(main())
