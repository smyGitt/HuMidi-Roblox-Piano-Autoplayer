from PyQt6.QtWidgets import QPushButton
from PyQt6.QtCore import Qt

# Variation selectors used in _normalize() to control text-vs-emoji rendering.
_VS15 = chr(0xFE0E)  # U+FE0E: text presentation -- forces glyph, not color emoji
_VS16 = chr(0xFE0F)  # U+FE0F: emoji presentation -- stripped before re-normalizing


class HuMidiButton(QPushButton):
    """Standard action button for the HuMidi UI.

    Enforces PointingHandCursor and WA_StyledBackground on every instance.
    Accepts an optional tooltip string and an optional scheme dict for
    inline color overrides (see _build_scheme_qss for accepted keys).

    Single-char text (icon buttons) is automatically:
      - Normalized with VS15 (U+FE0E) to force text rendering over emoji.
      - Rendered via _ICON_FONT_FAMILY at _ICON_PT via an inline widget stylesheet,
        which takes precedence over the inherited application stylesheet.
    """

    _ICON_FONT_FAMILY = "Segoe UI Symbol"  # monochrome text glyphs for ▶ ⏸ ⏹ on Win10/11
    _ICON_PT = 13  # point size for single-char (icon) labels in expanded state

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
        super().__init__("", parent)   # empty init -- setText below handles normalization
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        if tooltip:
            self.setToolTip(tooltip)
        # Build inline widget stylesheet. Inline stylesheets win over the inherited
        # application stylesheet, so font-family/font-size set here are not overridden.
        parts: list[str] = []
        if self._is_icon_text(text):
            parts.append(
                f"QPushButton {{ font-family: '{self._ICON_FONT_FAMILY}';"
                f" font-size: {self._ICON_PT}pt; }}"
            )
        if scheme:
            parts.append(self._build_scheme_qss(scheme))
        if parts:
            self.setStyleSheet(" ".join(parts))
        self.setText(text)

    def setText(self, text: str) -> None:
        """Normalize single-char text with VS15 before passing to Qt."""
        super().setText(self._normalize(text))

    def set_icon_mode(self, enabled: bool) -> None:
        """Toggle the icon_mode QSS property and force a style refresh."""
        self.setProperty("icon_mode", "true" if enabled else "false")
        self.style().unpolish(self)
        self.style().polish(self)
        self.update()

    @staticmethod
    def _normalize(text: str) -> str:
        """Return text with VS15 appended if it is a single Unicode code point.

        Strips any trailing VS15/VS16 first so normalization is idempotent.
        Multi-char strings are returned unchanged.
        """
        core = text[:-1] if text and text[-1] in (_VS15, _VS16) else text
        return (core + _VS15) if len(core) == 1 else text

    @staticmethod
    def _is_icon_text(text: str) -> bool:
        """True when text (after stripping VS) is a single Unicode code point."""
        core = text[:-1] if text and text[-1] in (_VS15, _VS16) else text
        return len(core) == 1

    @staticmethod
    def _build_scheme_qss(scheme: dict) -> str:
        base: dict[str, list[str]] = {}   # pseudo -> list of "prop: value"
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
