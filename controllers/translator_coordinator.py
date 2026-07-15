from PySide6.QtWidgets import QMessageBox

from core.core import KeyMapper, TempoMap
from core.translator import FormatRegistry


class TranslatorCoordinator:
    """Bridges TranslatorTab/PlaybackController for Virtual Piano sheet import/export."""

    def __init__(self, window, ui, playback_controller, state, playback_coordinator):
        self.window = window
        self.ui = ui
        self.playback_controller = playback_controller
        self.state = state
        self.playback_coordinator = playback_coordinator

    def bind_signals(self) -> None:
        self.ui.translator_tab.play_sheet_requested.connect(self._on_play_sheet)
        self.ui.translator_tab.export_requested.connect(self._on_export_sheet)

    def _on_play_sheet(self, text: str, format_name: str, bpm: int, humanize: bool) -> None:
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return

        fmt = FormatRegistry.get(format_name)
        if not fmt:
            QMessageBox.critical(self.window, "Unknown Format", f"No handler found for format: {format_name}")
            return

        use_88 = self.ui.playback_tab.use_88_key_check.isChecked()
        key_mapper = KeyMapper(use_88_key_layout=use_88)

        try:
            notes = fmt.parse(text, float(bpm), key_mapper)
        except Exception as e:
            QMessageBox.critical(self.window, "Parse Error", f"Failed to parse sheet:\n{e}")
            return

        if not notes:
            QMessageBox.warning(self.window, "No Notes", "No playable notes were found in the pasted sheet.")
            return

        tempo_us = int(60_000_000 / bpm)
        tempo_map = TempoMap([(0, tempo_us)], [])

        if humanize:
            config = self.ui.gather_playback_config()
        else:
            config = {
                'use_88_key_layout': use_88, 'debug_mode': False, 'countdown': False,
                'pedal_style': 'none', 'simulate_hands': False, 'vary_velocity': False,
                'enable_chord_roll': False, 'vary_timing': False, 'timing_variance': 0.01,
                'vary_articulation': False, 'articulation': 0.95,
                'enable_drift_correction': False, 'drift_decay_factor': 0.25,
                'enable_mistakes': False, 'mistake_chance': 0.0,
                'enable_tempo_sway': False, 'tempo_sway_intensity': 0.0,
                'invert_tempo_sway': False, 'use_ai_pedal': False,
            }

        self.ui.debug_tab.append_log(f"Importing sheet: {len(notes)} notes at {bpm} BPM ({format_name})")
        self.ui.debug_tab.clear_snapshot()
        self.ui.debug_tab.update_snapshot({
            'file': '(pasted sheet)',
            'source': f"{format_name} import",
            'tempo': f"{bpm} BPM",
        })
        self.playback_controller.play_from_notes(config, notes, tempo_map)
        self.ui.set_controls_enabled(False)
        self.ui.play_button.setEnabled(True)
        self.ui.stop_button.setEnabled(True)
        self.ui.scrubber_slider.setEnabled(True)
        self.playback_coordinator.sync_play_button()
        if self.ui._nav_btns[1].isEnabled():
            self.ui.tabs.setCurrentIndex(1)  # Switch to Visualizer

    def _on_export_sheet(self, format_name: str) -> None:
        if not self.state.current_notes:
            QMessageBox.warning(self.window, "No MIDI Loaded",
                                "Load and prepare a MIDI file on the Playback tab first.")
            return

        fmt = FormatRegistry.get(format_name)
        if not fmt:
            QMessageBox.critical(self.window, "Unknown Format", f"No handler found for format: {format_name}")
            return

        use_88 = self.ui.playback_tab.use_88_key_check.isChecked()
        key_mapper = KeyMapper(use_88_key_layout=use_88)
        tempo_map = self.state.parsed_tempo_map or TempoMap([(0, 500000)], [])

        try:
            text = fmt.serialize(self.state.current_notes, key_mapper, tempo_map)
        except Exception as e:
            QMessageBox.critical(self.window, "Export Error", f"Failed to generate sheet:\n{e}")
            return

        self.ui.translator_tab.set_export_text(text)
        self.ui.debug_tab.append_log(f"Sheet exported: {format_name} ({len(text.splitlines())} lines)")
