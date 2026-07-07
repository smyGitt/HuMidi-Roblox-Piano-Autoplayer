import threading

from PySide6.QtCore import QObject, Signal
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
    save_requested = Signal()
    bound_save_updated = Signal(str)

    def __init__(self):
        super().__init__()
        # _lock guards every field below that the pynput listener thread writes
        # (current_mods, current_key, save_mods, save_key, _held_mods,
        # listening_for_bind, listening_for_save_bind) and that the GUI thread
        # reads (format_hotkey_string, format_save_hotkey_string).
        self._lock = threading.Lock()
        self.current_mods = frozenset()
        self.current_key  = Key.f6
        self.save_mods    = frozenset({Key.ctrl})
        self.save_key     = keyboard.KeyCode.from_char('s')
        self._held_mods   = set()
        self.listening_for_bind      = False
        self.listening_for_save_bind = False
        self.listener = None
        self._start_listener()

    def _start_listener(self):
        self.listener = keyboard.Listener(
            on_press=self.on_press,
            on_release=self.on_release,
        )
        self.listener.start()

    def _format_combo(self, mods, k) -> str:
        parts = []
        for mod, label in [(Key.ctrl, "Ctrl"), (Key.alt, "Alt"),
                           (Key.shift, "Shift"), (Key.cmd, "Cmd")]:
            if mod in mods:
                parts.append(label)
        if hasattr(k, 'char') and k.char:
            parts.append(k.char.upper())
        else:
            parts.append(str(k).replace('Key.', '').upper())
        return "+".join(parts)

    def format_hotkey_string(self):
        with self._lock:
            mods = self.current_mods
            k = self.current_key
        return self._format_combo(mods, k)

    def format_save_hotkey_string(self):
        with self._lock:
            mods = self.save_mods
            k = self.save_key
        return self._format_combo(mods, k)

    def on_release(self, key):
        with self._lock:
            self._held_mods.discard(_normalize(key))

    def on_press(self, key):
        # Runs on the pynput listener thread. All shared-state reads/writes are
        # done under the lock, then signal emission (and format_* helpers,
        # which lock independently) happens after release to avoid re-entrancy.
        canon = _normalize(key)
        emit_bound      = False
        emit_save_bound = False
        emit_toggle     = False
        emit_save       = False
        with self._lock:
            if canon in _CANONICAL_MODS:
                self._held_mods.add(canon)

            if self.listening_for_bind:
                if canon in _CANONICAL_MODS:
                    return
                self.current_mods = frozenset(self._held_mods)
                self.current_key  = key
                self.listening_for_bind = False
                emit_bound = True
            elif self.listening_for_save_bind:
                if canon in _CANONICAL_MODS:
                    return
                self.save_mods = frozenset(self._held_mods)
                self.save_key  = key
                self.listening_for_save_bind = False
                emit_save_bound = True
            else:
                if _normalize(key) == _normalize(self.current_key):
                    if frozenset(self._held_mods) == self.current_mods:
                        emit_toggle = True
                if _normalize(key) == _normalize(self.save_key):
                    if frozenset(self._held_mods) == self.save_mods:
                        emit_save = True

        if emit_bound:
            self.bound_updated.emit(self.format_hotkey_string())
        if emit_save_bound:
            self.bound_save_updated.emit(self.format_save_hotkey_string())
        if emit_toggle:
            self.toggle_requested.emit()
        if emit_save:
            self.save_requested.emit()

    def start_binding(self):
        with self._lock:
            self.listening_for_bind = True

    def start_save_binding(self):
        with self._lock:
            self.listening_for_save_bind = True
