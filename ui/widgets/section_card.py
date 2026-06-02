from PyQt6.QtWidgets import QFrame, QVBoxLayout, QHBoxLayout, QLabel


def make_card(
    title: str,
    title_buttons=None,
    header_widgets=None,
    footer_widgets=None,
    outer_margins=None,
    row_h_pad: int = 0,
) -> tuple:
    """Return a styled section card (QFrame) and its content QVBoxLayout.

    title_buttons   -- widgets placed inline-right of the title label
    header_widgets  -- widgets in a row below the title, above the body
    footer_widgets  -- widgets in a row below the body
    outer_margins   -- (l, t, r, b) override for the card's outer layout;
                       defaults to (14, 6, 14, 7)
    row_h_pad       -- extra horizontal padding applied to the title, header,
                       and footer rows (useful when outer_margins strips the
                       card's default horizontal padding for edge-to-edge body
                       content, but title/footer still need inset)
    """
    card = QFrame()
    card.setObjectName("section_card")
    outer = QVBoxLayout(card)
    outer.setContentsMargins(*(outer_margins if outer_margins is not None else (14, 6, 14, 7)))
    outer.setSpacing(4)

    if title:
        lbl = QLabel(title)
        lbl.setProperty("role", "section")
        if title_buttons:
            title_row = QHBoxLayout()
            title_row.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
            title_row.setSpacing(6)
            title_row.addWidget(lbl)
            title_row.addStretch()
            for btn in title_buttons:
                title_row.addWidget(btn)
            outer.addLayout(title_row)
        else:
            lbl.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
            outer.addWidget(lbl)

    if header_widgets:
        header_row = QHBoxLayout()
        header_row.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
        header_row.setSpacing(6)
        for w in header_widgets:
            header_row.addWidget(w)
        outer.addLayout(header_row)

    body = QVBoxLayout()
    body.setContentsMargins(0, 0, 0, 0)
    body.setSpacing(4)
    outer.addLayout(body)

    if footer_widgets:
        footer_row = QHBoxLayout()
        footer_row.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
        footer_row.setSpacing(6)
        for w in footer_widgets:
            footer_row.addWidget(w)
        outer.addLayout(footer_row)

    return card, body
