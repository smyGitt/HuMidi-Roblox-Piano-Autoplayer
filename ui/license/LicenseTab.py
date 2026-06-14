from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QFrame,
    QLabel, QPushButton, QTextEdit, QStackedWidget
)
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont

from ui.widgets import make_card


_LICENSE_TEXTS: dict[str, str] = {
    "HuMidi": """\
MIT License

Copyright (c) 2026 smyGitt

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
""",

    "PedalAI Dataset": """\
The following datasets were used to train the BiLSTM AI pedal timing
model bundled with HuMidi.

────────────────
POP909
────────────────
A piano MIDI dataset of 909 popular songs with performance annotations.

Citation:
  Wang, Z., Chen, K., Jiang, J., Zhang, Y., Xu, M., Dai, S., Xia, G.,
  & Fazekas, G. (2020). POP909: A Pop-song Dataset for Music Arrangement
  Generation. Proceedings of ISMIR 2020.

License : MIT
URL     : https://github.com/music-x-lab/POP909-Dataset

────────────────
GiantMIDI-Piano
────────────────
A large-scale MIDI dataset of classical piano music transcribed from
audio recordings.

Citation:
  Kong, Q., Li, B., Chen, J., & Wang, Y. (2020). GiantMIDI-Piano: A
  large-scale MIDI dataset for classical piano music. arXiv:2010.07061.

License : Creative Commons Attribution 4.0 International (CC BY 4.0)

  You are free to share and adapt the material for any purpose, provided
  appropriate credit is given.

URL     : https://github.com/bytedance/GiantMIDI-Piano
""",

    "Third-Party Libraries": """\
PyQt6
  License : GPL v3 / Commercial (Riverbank Computing)
  URL     : https://riverbankcomputing.com/software/pyqt/

mido
  License : MIT
  URL     : https://github.com/mido/mido

numpy
  License : BSD 3-Clause
  URL     : https://numpy.org/

pynput
  License : LGPL v3
  URL     : https://github.com/moses-palmer/pynput

PyInstaller
  License : GPL v2 with a special exception for bundled apps
  URL     : https://pyinstaller.org
""",

    "Phosphor Icons": """\
Phosphor Icons
  A flexible icon family for interfaces, diagrams, presentations, and more.

  License : MIT
  URL     : https://phosphoricons.com
  GitHub  : https://github.com/phosphor-icons/homepage

Copyright (c) 2020 Phosphor Icons

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
""",
}


class LicenseTab(QWidget):

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        # Full-width page header bar
        header = QFrame()
        header.setObjectName("page_header")
        header.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        hl = QHBoxLayout(header)
        hl.setContentsMargins(14, 10, 14, 10)
        hl.setSpacing(8)
        title_lbl = QLabel("Licenses & Credits")
        title_lbl.setObjectName("page_header_title")
        hl.addWidget(title_lbl)
        hl.addStretch()
        outer.addWidget(header)

        # Body widget restores side margins
        body = QWidget()
        layout = QVBoxLayout(body)
        layout.setContentsMargins(16, 8, 16, 16)
        layout.setSpacing(0)
        outer.addWidget(body, 1)

        # -- Nav + stacked content inside a single card ------------------------
        split = QHBoxLayout()
        split.setContentsMargins(0, 0, 0, 0)
        split.setSpacing(0)

        # Left nav panel
        nav_panel = QWidget()
        nav_panel.setObjectName("settings_nav_panel")
        nav_panel.setFixedWidth(170)
        nav_layout = QVBoxLayout(nav_panel)
        nav_layout.setContentsMargins(0, 8, 0, 8)
        nav_layout.setSpacing(0)

        self._tab_btns = []
        for i, name in enumerate(_LICENSE_TEXTS):
            btn = QPushButton(name)
            btn.setCheckable(True)
            btn.setProperty("role", "settings_nav")
            btn.clicked.connect(lambda checked, idx=i: self._switch_tab(idx))
            nav_layout.addWidget(btn)
            self._tab_btns.append(btn)
        nav_layout.addStretch()

        # Vertical divider
        sep = QFrame()
        sep.setFrameShape(QFrame.Shape.VLine)
        sep.setObjectName("settings_nav_sep")

        # Stacked pages -- one QTextEdit per license entry
        self._stack = QStackedWidget()
        for text in _LICENSE_TEXTS.values():
            page = QWidget()
            page_layout = QVBoxLayout(page)
            page_layout.setContentsMargins(16, 12, 16, 12)
            page_layout.setSpacing(0)
            text_edit = QTextEdit()
            text_edit.setReadOnly(True)
            text_edit.setFont(QFont("Courier New", 9))
            text_edit.setPlainText(text)
            page_layout.addWidget(text_edit)
            self._stack.addWidget(page)

        split.addWidget(nav_panel)
        split.addWidget(sep)
        split.addWidget(self._stack, 1)

        card, card_body = make_card("", outer_margins=(0, 0, 0, 0))
        card_body.addLayout(split)
        layout.addWidget(card, 1)

        self._switch_tab(0)

    # ── Internal ──────────────────────────────────────────────────────────────

    def _switch_tab(self, idx: int) -> None:
        self._stack.setCurrentIndex(idx)
        for i, btn in enumerate(self._tab_btns):
            btn.setChecked(i == idx)
