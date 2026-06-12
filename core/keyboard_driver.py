from typing import Protocol, runtime_checkable


@runtime_checkable
class KeyboardDriver(Protocol):
    """Abstraction over physical keyboard output.

    pynput.keyboard.Controller satisfies this structurally at runtime.
    Pass a test double that also satisfies it to Player.__init__ to drive
    playback without real hardware -- no mocking library required.
    """

    def press(self, key) -> None: ...
    def release(self, key) -> None: ...
