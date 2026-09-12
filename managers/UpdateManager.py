import json
import sys
import urllib.request

from PySide6.QtCore import QThread, Signal

RELEASES_API = "https://api.github.com/repos/smyGitt/HuMidi-Roblox-Piano-Autoplayer/releases/latest"
RELEASES_PAGE = "https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer/releases/latest"
REQUEST_TIMEOUT_SECONDS = 8


def parse_version(tag: str) -> tuple:
    try:
        return tuple(int(x) for x in tag.lstrip("v").strip().split("."))
    except (ValueError, AttributeError):
        return ()


class UpdateChecker(QThread):
    update_available = Signal(str, str)
    no_update        = Signal()
    check_failed     = Signal()

    def __init__(self, current_version: str, force: bool = False):
        super().__init__()
        self._current_version = current_version
        self._force = force

    def run(self):
        if not self._force and not getattr(sys, "frozen", False):
            return
        try:
            req = urllib.request.Request(
                RELEASES_API, headers={"User-Agent": "HuMidi-updater"}
            )
            with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT_SECONDS) as resp:
                data = json.loads(resp.read().decode())

            latest_tag = data.get("tag_name", "")
            if not latest_tag:
                return

            latest_tuple = parse_version(latest_tag)
            current_tuple = parse_version(self._current_version)
            if not latest_tuple or not current_tuple:
                return

            if latest_tuple <= current_tuple:
                self.no_update.emit()
                return

            self.update_available.emit(latest_tag, RELEASES_PAGE)
        except Exception:
            self.check_failed.emit()
