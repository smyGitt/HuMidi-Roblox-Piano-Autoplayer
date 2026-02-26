from PyQt6.QtCore import QObject, pyqtSignal as Signal
from pynput import keyboard
from pynput.keyboard import Key

class HotkeyManager(QObject):
    toggle_requested = Signal()
    bound_updated = Signal(str)

    def __init__(self):
        super().__init__()
        self.current_key = Key.f6
        self.listener = None
        self.listening_for_bind = False
        self._start_listener()

    def _start_listener(self):
        self.listener = keyboard.Listener(on_press=self.on_press)
        self.listener.start()

    def _format_key_string(self, key):
        if hasattr(key, 'char') and key.char:
            return key.char
        return str(key).replace('Key.', '')

    def on_press(self, key):
        if self.listening_for_bind:
            self.current_key = key
            self.listening_for_bind = False
            self.bound_updated.emit(self._format_key_string(key))
            return

        if key == self.current_key:
            self.toggle_requested.emit()

    def start_binding(self):
        self.listening_for_bind = True