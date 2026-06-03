from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QFrame,
    QLabel, QTextEdit, QListWidget, QAbstractItemView
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
        layout = QVBoxLayout(self)
        layout.setContentsMargins(16, 0, 16, 16)
        layout.setSpacing(0)

        # -- Page header -------------------------------------------------------
        header = QFrame()
        header.setObjectName("page_header")
        hl = QHBoxLayout(header)
        hl.setContentsMargins(0, 14, 0, 8)
        hl.setSpacing(8)
        title_lbl = QLabel("Licenses & Credits")
        title_lbl.setProperty("role", "title")
        hl.addWidget(title_lbl)
        hl.addStretch()
        # TODO: meta chips (e.g. component count)
        layout.addWidget(header)

        # -- Master-detail grid (~244px : rest) --------------------------------
        body_row = QHBoxLayout()
        body_row.setSpacing(10)

        # Left card: Components list (~244px fixed)
        comp_card, comp_body = make_card("Components")
        comp_card.setFixedWidth(244)
        self._list = QListWidget()
        self._list.setSelectionMode(QAbstractItemView.SelectionMode.SingleSelection)
        for name in _LICENSE_TEXTS:
            self._list.addItem(name)
        self._list.setCurrentRow(0)
        self._list.currentItemChanged.connect(self._on_list_changed)
        # TODO: per-row license badge (MIT / CC BY 4.0 / GPL v3)
        comp_body.addWidget(self._list)
        body_row.addWidget(comp_card)

        # Right card: License Text (document header + full text)
        detail_card, detail_body = make_card("License Text")

        doc_header = QHBoxLayout()
        doc_header.setSpacing(8)
        first_name = self._list.item(0).text() if self._list.count() else ""
        self._doc_title_lbl = QLabel(first_name)
        self._doc_title_lbl.setProperty("role", "section")
        doc_header.addWidget(self._doc_title_lbl)
        doc_header.addStretch()
        # TODO: license badge label and URL link
        detail_body.addLayout(doc_header)

        sep = QFrame()
        sep.setObjectName("h_sep")
        sep.setFrameShape(QFrame.Shape.HLine)
        detail_body.addWidget(sep)

        self._text = QTextEdit()
        self._text.setReadOnly(True)
        self._text.setFont(QFont("Courier New", 9))
        self._text.setPlainText(_LICENSE_TEXTS.get(first_name, ""))
        detail_body.addWidget(self._text)

        body_row.addWidget(detail_card, 1)
        layout.addLayout(body_row, 1)

    # ── Slots ─────────────────────────────────────────────────────────────────

    def _on_list_changed(self, current, previous) -> None:
        if current is None:
            return
        self._on_changed(current.text())

    def _on_changed(self, name: str) -> None:
        self._doc_title_lbl.setText(name)
        self._text.setPlainText(_LICENSE_TEXTS.get(name, ""))
