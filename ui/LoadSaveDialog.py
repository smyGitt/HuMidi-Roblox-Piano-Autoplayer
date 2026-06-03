import os
import json
from datetime import datetime
from PyQt6.QtWidgets import (QDialog, QVBoxLayout, QHBoxLayout, QSplitter,
                             QTreeWidget, QTreeWidgetItem, QWidget,
                             QScrollArea, QPushButton,
                             QLabel, QFrame, QGridLayout, QMessageBox, QInputDialog)
from PyQt6.QtCore import Qt

from ui.theme import ThemeManager, generate_stylesheet


class LoadSaveDialog(QDialog):
    def __init__(self, save_dir, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Load Saved Playback")
        self.resize(820, 520)
        self.setStyleSheet(generate_stylesheet(ThemeManager.get_active()))
        self.save_dir = save_dir
        self.selected_file = None
        self._setup_ui()
        self._load_files()

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        splitter = QSplitter(Qt.Orientation.Horizontal)

        # ── Left: tree ────────────────────────────────────────────────
        self.tree = QTreeWidget()
        self.tree.setHeaderHidden(True)
        self.tree.setAlternatingRowColors(True)
        self.tree.itemSelectionChanged.connect(self._on_selection_changed)
        splitter.addWidget(self.tree)

        # ── Right: details pane ───────────────────────────────────────
        self.details_widget = QWidget()
        self.details_layout = QVBoxLayout(self.details_widget)
        self.details_layout.setAlignment(Qt.AlignmentFlag.AlignTop)
        self.details_layout.setContentsMargins(12, 8, 8, 8)
        self.details_layout.setSpacing(6)

        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True)
        self.scroll_area.setWidget(self.details_widget)

        splitter.addWidget(self.scroll_area)
        splitter.setSizes([270, 550])

        layout.addWidget(splitter)

        # ── Buttons ───────────────────────────────────────────────────
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(6)

        self.rename_btn = QPushButton("Rename")
        self.delete_btn = QPushButton("Delete")
        self.delete_btn.setObjectName("stop_button")   # reuse red styling
        self.cancel_btn = QPushButton("Cancel")
        self.load_btn = QPushButton("Load")
        self.load_btn.setObjectName("save_button")     # reuse accent styling

        self.rename_btn.setEnabled(False)
        self.delete_btn.setEnabled(False)
        self.load_btn.setEnabled(False)

        self.rename_btn.clicked.connect(self._rename_save)
        self.delete_btn.clicked.connect(self._delete_save)
        self.cancel_btn.clicked.connect(self.reject)
        self.load_btn.clicked.connect(self.accept)

        btn_layout.addWidget(self.rename_btn)
        btn_layout.addWidget(self.delete_btn)
        btn_layout.addStretch()
        btn_layout.addWidget(self.cancel_btn)
        btn_layout.addWidget(self.load_btn)
        layout.addLayout(btn_layout)

    def _load_files(self):
        self.tree.clear()
        if not os.path.exists(self.save_dir):
            return

        grouped_files = {}
        for filename in os.listdir(self.save_dir):
            if filename.endswith('.json'):
                filepath = os.path.join(self.save_dir, filename)
                try:
                    with open(filepath, 'r') as file:
                        data = json.load(file)
                        metadata = data.get('metadata', {})
                        midi_name = metadata.get('source_midi_filename', 'Unknown MIDI')
                        if midi_name not in grouped_files:
                            grouped_files[midi_name] = []
                        grouped_files[midi_name].append((filename, filepath, metadata))
                except Exception:
                    pass

        for midi_name, files in grouped_files.items():
            parent_item = QTreeWidgetItem(self.tree, [midi_name])
            parent_item.setFlags(parent_item.flags() & ~Qt.ItemFlag.ItemIsSelectable)
            font = parent_item.font(0)
            font.setBold(True)
            parent_item.setFont(0, font)
            files.sort(key=lambda x: x[2].get('creation_timestamp', ''), reverse=True)

            for _f, filepath, metadata in files:
                timestamp = metadata.get('creation_timestamp', 'Unknown Time')
                try:
                    dt = datetime.fromisoformat(timestamp)
                    timestamp_str = dt.strftime("%Y-%m-%d  %H:%M")
                except ValueError:
                    timestamp_str = timestamp

                display_name = metadata.get('custom_name', timestamp_str)
                child_item = QTreeWidgetItem(parent_item, [display_name])
                child_item.setData(0, Qt.ItemDataRole.UserRole, filepath)

        self.tree.expandAll()

    def _on_selection_changed(self):
        selected = self.tree.selectedItems()
        if not selected:
            self._disable_actions()
            return

        filepath = selected[0].data(0, Qt.ItemDataRole.UserRole)
        if filepath:
            self.selected_file = filepath
            self.load_btn.setEnabled(True)
            self.rename_btn.setEnabled(True)
            self.delete_btn.setEnabled(True)
            try:
                with open(filepath, 'r') as f:
                    data = json.load(f)
                    self._display_metadata(data.get('metadata', {}))
            except Exception:
                self._clear_details()
        else:
            self._disable_actions()

    def _disable_actions(self):
        self.load_btn.setEnabled(False)
        self.rename_btn.setEnabled(False)
        self.delete_btn.setEnabled(False)
        self._clear_details()
        self.selected_file = None

    def _rename_save(self):
        if not self.selected_file:
            return
        current_custom = ""
        try:
            with open(self.selected_file, 'r') as f:
                data = json.load(f)
                current_custom = data.get('metadata', {}).get('custom_name', '')
        except Exception:
            pass

        new_name, ok = QInputDialog.getText(
            self, "Rename Save",
            "Enter custom name (leave blank to revert to timestamp):",
            text=current_custom
        )
        if ok:
            try:
                with open(self.selected_file, 'r') as f:
                    data = json.load(f)
                if new_name.strip():
                    data['metadata']['custom_name'] = new_name.strip()
                else:
                    data['metadata'].pop('custom_name', None)
                with open(self.selected_file, 'w') as f:
                    json.dump(data, f, indent=4)
                self._load_files()
                self._disable_actions()
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Could not rename file:\n{e}")

    def _delete_save(self):
        if not self.selected_file:
            return
        reply = QMessageBox.question(
            self, 'Delete Save',
            'Are you sure you want to permanently delete this save?',
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No
        )
        if reply == QMessageBox.StandardButton.Yes:
            try:
                os.remove(self.selected_file)
                self._load_files()
                self._disable_actions()
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Could not delete file:\n{e}")

    def _clear_details(self):
        new_widget = QWidget()
        self.details_layout = QVBoxLayout(new_widget)
        self.details_layout.setAlignment(Qt.AlignmentFlag.AlignTop)
        self.details_layout.setContentsMargins(12, 8, 8, 8)
        self.details_layout.setSpacing(6)
        self.scroll_area.setWidget(new_widget)
        self.details_widget = new_widget

    def _display_metadata(self, metadata):
        self._clear_details()

        # Title
        title = QLabel(metadata.get('source_midi_filename', 'Unknown MIDI'))
        title.setProperty("role", "title")
        title.setWordWrap(True)
        self.details_layout.addWidget(title)

        # Timestamp
        ts = metadata.get('creation_timestamp', 'Unknown')
        try:
            dt = datetime.fromisoformat(ts)
            date_str = dt.strftime('%B %d, %Y  ·  %I:%M %p')
        except Exception:
            date_str = ts
        date_label = QLabel(date_str)
        date_label.setProperty("role", "muted")
        self.details_layout.addWidget(date_label)

        # Separator
        sep = QFrame()
        sep.setObjectName("h_sep")
        sep.setFrameShape(QFrame.Shape.HLine)
        self.details_layout.addSpacing(4)
        self.details_layout.addWidget(sep)
        self.details_layout.addSpacing(4)

        settings = metadata.get('playback_settings', {})

        # Playback settings
        pb_label = QLabel("Playback Settings")
        pb_label.setProperty("role", "section")
        self.details_layout.addWidget(pb_label)

        pb_grid = QGridLayout()
        pb_grid.setSpacing(4)
        pb_grid.setColumnMinimumWidth(1, 12)

        def add_row(grid, row, key_str, val_str):
            k = QLabel(key_str)
            k.setProperty("role", "muted")
            v = QLabel(str(val_str))
            v.setProperty("role", "value")
            grid.addWidget(k, row, 0)
            grid.addWidget(v, row, 2)

        add_row(pb_grid, 0, "Tempo", f"{settings.get('tempo', 100)}%")
        add_row(pb_grid, 1, "Pedal Style", settings.get('pedal_style', 'hybrid').title())
        add_row(pb_grid, 2, "88-Key Layout", "Yes" if settings.get('use_88_key_layout') else "No")
        self.details_layout.addLayout(pb_grid)
        self.details_layout.addSpacing(8)

        # Humanization
        hum_label = QLabel("Humanization")
        hum_label.setProperty("role", "section")
        self.details_layout.addWidget(hum_label)

        hum_grid = QGridLayout()
        hum_grid.setSpacing(4)
        hum_grid.setColumnMinimumWidth(1, 12)
        h_row = 0

        def add_h_row(key_str, val_str):
            nonlocal h_row
            k = QLabel(key_str)
            k.setProperty("role", "muted")
            v = QLabel(str(val_str))
            v.setProperty("role", "value")
            hum_grid.addWidget(k, h_row, 0)
            hum_grid.addWidget(v, h_row, 2)
            h_row += 1

        if settings.get('simulate_hands'):
            add_h_row("Simulate Hands", "Yes")
        if settings.get('enable_chord_roll'):
            add_h_row("Chord Rolling", "Yes")
        if settings.get('vary_timing'):
            add_h_row("Vary Timing", f"{settings.get('timing_variance', 0.0)}s")
        if settings.get('vary_articulation'):
            add_h_row("Vary Articulation",
                      f"{int(settings.get('articulation', 1.0) * 100)}%")
        if settings.get('enable_drift_correction'):
            add_h_row("Hand Drift",
                      f"{int(settings.get('drift_decay_factor', 1.0) * 100)}%")
        if settings.get('enable_mistakes'):
            add_h_row("Mistake Chance", f"{settings.get('mistake_chance', 0.0)}%")
        if settings.get('enable_tempo_sway'):
            inv = " (Inverted)" if settings.get('invert_tempo_sway') else ""
            add_h_row("Tempo Sway",
                      f"{settings.get('tempo_sway_intensity', 0.0)}s{inv}")

        if h_row == 0:
            none_lbl = QLabel("None selected")
            none_lbl.setProperty("role", "placeholder")
            hum_grid.addWidget(none_lbl, 0, 0)

        self.details_layout.addLayout(hum_grid)
        self.details_layout.addStretch()

    def get_selected_data(self):
        if not self.selected_file:
            return None, None
        try:
            with open(self.selected_file, 'r') as f:
                data = json.load(f)
            return self.selected_file, data
        except Exception:
            return None, None
