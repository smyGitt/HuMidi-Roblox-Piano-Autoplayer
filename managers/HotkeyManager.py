from PyQt6.QtCore import QObject, pyqtSignal as Signal
from pynput import keyboard
from pynput.keyboard import Key

_MODIFIER_NORMALIZE = {
    Key.shift_l: Key.shift, Key.shift_r: Key.shift,
    Key.ctrl_l:  Key.ctrl,  Key.ctrl_r:  Key.ctrl,
    Key.alt_l:   Key.alt,   Key.alt_r:   Key.alt,
    Key.cmd_l:   Key.cmd,   Key.cmd_r:   Key.cmd,
}
_CANONICAL_MODS = {Key.shift, Key.ctrl, Key.alt, Key.cmd}

def _normalize(key):
    return _MODIFIER_NORMALIZE.get(key, key)


class HotkeyManager(QObject):
    toggle_requested = Signal()
    bound_updated = Signal(str)

    def __init__(self):
        super().__init__()
        self.current_mods = frozenset()
        self.current_key  = Key.f6
        self._held_mods   = set()
        self.listening_for_bind = False
        self.listener = None
        self._start_listener()

    def _start_listener(self):
        self.listener = keyboard.Listener(
            on_press=self.on_press,
            on_release=self.on_release,
        )
        self.listener.start()

    def format_hotkey_string(self):
        parts = []
        for mod, label in [(Key.ctrl, "Ctrl"), (Key.alt, "Alt"),
                           (Key.shift, "Shift"), (Key.cmd, "Cmd")]:
            if mod in self.current_mods:
                parts.append(label)
        k = self.current_key
        if hasattr(k, 'char') and k.char:
            parts.append(k.char.upper())
        else:
            parts.append(str(k).replace('Key.', '').upper())
        return "+".join(parts)

    def on_release(self, key):
        self._held_mods.discard(_normalize(key))

    def on_press(self, key):
        canon = _normalize(key)
        if canon in _CANONICAL_MODS:
            self._held_mods.add(canon)

        if self.listening_for_bind:
            if canon in _CANONICAL_MODS:
                return
            self.current_mods = frozenset(self._held_mods)
            self.current_key  = key
            self.listening_for_bind = False
            self.bound_updated.emit(self.format_hotkey_string())
            return

        if _normalize(key) == _normalize(self.current_key):
            if frozenset(self._held_mods) == self.current_mods:
                self.toggle_requested.emit()

    def start_binding(self):
        self.listening_for_bind = True
