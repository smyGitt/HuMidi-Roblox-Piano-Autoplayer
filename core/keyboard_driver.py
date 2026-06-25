import sys
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


def build_driver() -> "KeyboardDriver":
    """Return the appropriate KeyboardDriver for the current platform.

    On Linux, wraps pynput Controller in _LinuxController so that single-character
    string keys are converted to KeyCode(vk=ord(char), char=char) before dispatch.
    This forces pynput to use XTest.fake_input() instead of XSendEvent(). pynput's
    X11 backend checks `key.vk is not None` to choose the dispatch path; XSendEvent
    is rejected by Wayland compositors and some XWayland setups, while XTest works
    reliably in both. X11 keysym values for printable ASCII (0x20-0x7E) equal their
    Unicode/ASCII code points, so ord(char) is the correct keysym for every character
    produced by KeyMapper.

    On macOS, checks that Accessibility permissions are granted before returning
    the Controller. Without them, pynput key injection silently does nothing.
    Raises PermissionError with actionable instructions if the check fails.

    On Windows and all other platforms, returns the raw Controller unchanged.
    """
    from pynput.keyboard import Controller
    ctrl = Controller()

    if sys.platform.startswith("linux"):
        return _LinuxController(ctrl)

    if sys.platform == "darwin":
        if not _check_macos_accessibility():
            raise PermissionError(
                "macOS Accessibility permission is required for key injection.\n"
                "Open System Settings > Privacy & Security > Accessibility, "
                "enable this application, then restart HuMidi."
            )

    return ctrl


def _check_macos_accessibility() -> bool:
    """Return True if this process has macOS Accessibility (AX) permission.

    Uses ctypes to call AXIsProcessTrusted() from the ApplicationServices
    framework. Returns True on any import/call failure so that the check does
    not produce false negatives in unusual environments (e.g. CI, bundled apps
    that self-elevate). Only called on darwin.
    """
    try:
        import ctypes
        import ctypes.util
        lib = ctypes.util.find_library("ApplicationServices")
        if not lib:
            return True
        appservices = ctypes.cdll.LoadLibrary(lib)
        appservices.AXIsProcessTrusted.restype = ctypes.c_bool
        return bool(appservices.AXIsProcessTrusted())
    except Exception:
        return True


class _LinuxController:
    """Thin wrapper around pynput Controller for Linux X11/Wayland compatibility.

    Converts single-character string keys to KeyCode(vk=ord(char), char=char)
    before dispatching press/release, forcing pynput to use XTest.fake_input()
    rather than XSendEvent(). All other attributes (including the pressed()
    context manager used for modifier keys) delegate to the underlying Controller
    via __getattr__. Modifier keys (Key.ctrl, Key.shift, Key.alt) are Key enum
    values, not strings, so _coerce passes them through unchanged.
    """

    def __init__(self, ctrl) -> None:
        self._ctrl = ctrl

    def __getattr__(self, name):
        return getattr(self._ctrl, name)

    @staticmethod
    def _coerce(key):
        from pynput.keyboard import KeyCode
        if isinstance(key, str) and len(key) == 1:
            return KeyCode(vk=ord(key), char=key)
        return key

    def press(self, key) -> None:
        self._ctrl.press(self._coerce(key))

    def release(self, key) -> None:
        self._ctrl.release(self._coerce(key))
