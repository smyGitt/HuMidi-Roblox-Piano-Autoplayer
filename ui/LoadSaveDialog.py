import os
import json
from datetime import datetime
from PyQt6.QtWidgets import (QDialog, QVBoxLayout, QHBoxLayout, QSplitter, 
                             QTreeWidget, QTreeWidgetItem, QWidget, 
                             QScrollArea, QPushButton, QTextEdit, 
                             QLabel, QFrame, QGridLayout, QMessageBox, QInputDialog)
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont

class LoadSaveDialog(QDialog):
    def __init__(self, save_dir, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Load Saved Playback")
        self.resize(800, 500)
        self.save_dir = save_dir
        self.selected_file = None
        self._setup_ui()
        self._load_files()
        
    def _setup_ui(self):
        layout = QVBoxLayout(self)
        splitter = QSplitter(Qt.Orientation.Horizontal)
        
        self.tree = QTreeWidget()
        self.tree.setHeaderHidden(True)
        self.tree.itemSelectionChanged.connect(self._on_selection_changed)
        
        # Details container setup
        self.details_widget = QWidget()
        self.details_layout = QVBoxLayout(self.details_widget)
        self.details_layout.setAlignment(Qt.AlignmentFlag.AlignTop)
        
        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True)
        self.scroll_area.setWidget(self.details_widget)
        
        splitter.addWidget(self.tree)
        splitter.addWidget(self.scroll_area)
        splitter.setSizes([266, 534]) 
        
        layout.addWidget(splitter)
        
        btn_layout = QHBoxLayout()
        
        self.rename_btn = QPushButton("Rename")
        self.delete_btn = QPushButton("Delete")
        self.cancel_btn = QPushButton("Cancel")
        self.load_btn = QPushButton("Load")
        
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
        if not os.path.exists(self.save_dir): return
        
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
            files.sort(key=lambda x: x[2].get('creation_timestamp', ''), reverse=True)
            
            for f, filepath, metadata in files:
                timestamp = metadata.get('creation_timestamp', 'Unknown Time')
                try:
                    dt = datetime.fromisoformat(timestamp)
                    timestamp_str = dt.strftime("%Y-%m-%d %H:%M:%S")
                except ValueError:
                    timestamp_str = timestamp
                
                # Fetch custom name if user injected one via Rename mechanism
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
        if not self.selected_file: return
        
        current_custom = ""
        try:
            with open(self.selected_file, 'r') as f:
                data = json.load(f)
                current_custom = data.get('metadata', {}).get('custom_name', '')
        except: pass

        new_name, ok = QInputDialog.getText(self, "Rename Save", "Enter custom name (leave blank to revert to timestamp):", text=current_custom)
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
        if not self.selected_file: return
        
        reply = QMessageBox.question(self, 'Delete Save', 'Are you sure you want to permanently delete this sequence?', 
                                     QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
        if reply == QMessageBox.StandardButton.Yes:
            try:
                os.remove(self.selected_file)
                self._load_files()
                self._disable_actions()
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Could not delete file:\n{e}")

    def _clear_details(self):
        # Physically replace the entire nested container instead of individually destroying logic nodes
        new_widget = QWidget()
        self.details_layout = QVBoxLayout(new_widget)
        self.details_layout.setAlignment(Qt.AlignmentFlag.AlignTop)
        self.scroll_area.setWidget(new_widget)
        self.details_widget = new_widget

    def _display_metadata(self, metadata):
        self._clear_details()

        title = QLabel(metadata.get('source_midi_filename', 'Unknown MIDI'))
        title.setFont(QFont("Segoe UI", 16, QFont.Weight.Bold))
        self.details_layout.addWidget(title)

        ts = metadata.get('creation_timestamp', 'Unknown')
        try:
            dt = datetime.fromisoformat(ts)
            date_str = dt.strftime('%B %d, %Y at %I:%M:%S %p')
        except Exception:
            date_str = ts
            
        date_label = QLabel(f"Saved On: {date_str}")
        date_label.setStyleSheet("color: gray;")
        self.details_layout.addWidget(date_label)
        
        line = QFrame()
        line.setFrameShape(QFrame.Shape.HLine)
        line.setFrameShadow(QFrame.Shadow.Sunken)
        self.details_layout.addWidget(line)

        settings = metadata.get('playback_settings', {})
        
        # General Settings Grid
        grid = QGridLayout()
        row = 0
        def add_row(key_str, val_str):
            nonlocal row
            k_lbl = QLabel(key_str)
            k_lbl.setFont(QFont("Segoe UI", 10, QFont.Weight.Bold))
            v_lbl = QLabel(str(val_str))
            v_lbl.setFont(QFont("Segoe UI", 10))
            grid.addWidget(k_lbl, row, 0)
            grid.addWidget(v_lbl, row, 1)
            row += 1

        add_row("Tempo:", f"{settings.get('tempo', 100)}%")
        add_row("Pedal Style:", settings.get('pedal_style', 'hybrid').title())
        add_row("88-Key Layout:", "Yes" if settings.get('use_88_key_layout') else "No")
        
        self.details_layout.addSpacing(10)
        hum_title = QLabel("Enabled Humanization Parameters")
        hum_title.setFont(QFont("Segoe UI", 12, QFont.Weight.Bold))
        self.details_layout.addWidget(hum_title)
        self.details_layout.addLayout(grid)

        # Humanization Settings Grid
        hum_grid = QGridLayout()
        h_row = 0
        def add_h_row(key_str, val_str):
            nonlocal h_row
            k_lbl = QLabel(key_str)
            k_lbl.setFont(QFont("Segoe UI", 10, QFont.Weight.Bold))
            v_lbl = QLabel(str(val_str))
            v_lbl.setFont(QFont("Segoe UI", 10))
            hum_grid.addWidget(k_lbl, h_row, 0)
            hum_grid.addWidget(v_lbl, h_row, 1)
            h_row += 1

        if settings.get('simulate_hands'): add_h_row("Simulate Hands:", "Yes")
        if settings.get('enable_chord_roll'): add_h_row("Chord Rolling:", "Yes")
        if settings.get('vary_timing'): add_h_row("Vary Timing:", f"{settings.get('timing_variance', 0.0)}s")
        if settings.get('vary_articulation'): add_h_row("Vary Articulation:", f"{int(settings.get('articulation', 1.0) * 100)}%")
        if settings.get('enable_drift_correction'): add_h_row("Hand Drift:", f"{int(settings.get('drift_decay_factor', 1.0) * 100)}%")
        if settings.get('enable_mistakes'): add_h_row("Mistake Chance:", f"{settings.get('mistake_chance', 0.0)}%")
        if settings.get('enable_tempo_sway'): 
            inv = " (Inverted)" if settings.get('invert_tempo_sway') else ""
            add_h_row("Tempo Sway:", f"{settings.get('tempo_sway_intensity', 0.0)}s{inv}")

        if h_row == 0:
            none_lbl = QLabel("None selected")
            none_lbl.setStyleSheet("color: gray; font-style: italic;")
            hum_grid.addWidget(none_lbl, 0, 0)

        self.details_layout.addLayout(hum_grid)
        self.details_layout.addStretch()

    def get_selected_data(self):
        if not self.selected_file: return None, None
        try:
            with open(self.selected_file, 'r') as f:
                data = json.load(f)
            return self.selected_file, data
        except Exception:
            return None, None