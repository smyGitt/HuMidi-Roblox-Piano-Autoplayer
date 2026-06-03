from PyQt6.QtWidgets import QPushButton
from PyQt6.QtCore import Qt


class HuMidiButton(QPushButton):
    """Standard action button for the HuMidi UI.

    Enforces PointingHandCursor and WA_StyledBackground on every instance.
    Accepts an optional tooltip string and an optional scheme dict for
    inline color overrides (see _build_scheme_qss for accepted keys).
    """

    _SCHEME_KEYS = {
        "bg":               ("background-color", ""),
        "bg_hover":         ("background-color", ":hover"),
        "bg_pressed":       ("background-color", ":pressed"),
        "bg_disabled":      ("background-color", ":disabled"),
        "border":           ("border-color",     ""),
        "border_hover":     ("border-color",     ":hover"),
        "border_disabled":  ("border-color",     ":disabled"),
        "color":            ("color",             ""),
        "color_hover":      ("color",             ":hover"),
        "color_disabled":   ("color",             ":disabled"),
        "border_radius":    ("border-radius",     ""),
        "padding":          ("padding",           ""),
        "min_height":       ("min-height",        ""),
        "font_weight":      ("font-weight",       ""),
    }

    def __init__(self, text: str = "", *, tooltip: str = "",
                 scheme: dict | None = None, parent=None):
        super().__init__(text, parent)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        if tooltip:
            self.setToolTip(tooltip)
        if scheme:
            self.setStyleSheet(self._build_scheme_qss(scheme))

    def set_icon_mode(self, enabled: bool) -> None:
        """Toggle the icon_mode QSS property and force a style refresh."""
        self.setProperty("icon_mode", "true" if enabled else "false")
        self.style().unpolish(self)
        self.style().polish(self)
        self.update()

    @staticmethod
    def _build_scheme_qss(scheme: dict) -> str:
        base: dict[str, list[str]] = {}
        for key, value in scheme.items():
            if key not in HuMidiButton._SCHEME_KEYS:
                continue
            prop, pseudo = HuMidiButton._SCHEME_KEYS[key]
            base.setdefault(pseudo, []).append(f"{prop}: {value};")

        parts: list[str] = []
        selector_base = "QPushButton"
        for pseudo, rules in base.items():
            selector = f"{selector_base}{pseudo}"
            body = " ".join(rules)
            parts.append(f"{selector} {{ {body} }}")
        return " ".join(parts)
