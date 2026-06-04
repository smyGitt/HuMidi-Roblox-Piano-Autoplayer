from PyQt6.QtCore import QObject, pyqtSignal as Signal
from pynput.keyboard import Key, Controller
import time
import threading
import traceback
import heapq
import random
import bisect
import copy
from typing import List, Dict, Optional, Tuple

from core.models import Note, KeyEvent, MusicalSection, KeyState
from core.core import TempoMap, KeyMapper
from core.humanizer import Humanizer
import core.pedal_generator as pedal_generator

class Player(QObject):
    status_updated = Signal(str)
    progress_updated = Signal(float)
    playback_finished = Signal()
    visualizer_updated = Signal(list)
    pedal_updated = Signal(bool)
    auto_paused = Signal()
    error_occurred = Signal(str)

    def __init__(self, config: Dict, notes: List[Note], sections: List[MusicalSection], tempo_map: TempoMap):
        super().__init__()
        self.config = config
        self.notes = notes
        self.sections = sections
        self.tempo_map = tempo_map
        self.keyboard = Controller()
        self.mapper = KeyMapper(use_88_key_layout=self.config.get('use_88_key_layout', False))
        
        self.compiled_events: List[KeyEvent] = []
        self.event_index = 0

        self.stop_event = threading.Event()
        self.pause_event = threading.Event()
        self.key_states: Dict[str, KeyState] = {}
        self.active_pitches: set = set()
        self.pedal_is_down = False

        # Running key-net state for O(1) resume sync (maintained by _execute_chord_event)
        self._key_net: Dict[str, int] = {}
        self._key_last_press: Dict[str, KeyEvent] = {}
        self._pedal_net_down = False
        
        self.start_time = 0.0
        self.total_paused_time = 0.0
        self.last_pause_timestamp = 0.0
        self.total_duration = 0.0
        
        self.last_progress_emit_time = 0.0
        self.progress_update_interval = 1.0 / 60.0
        
        self.debug_log: Optional[List[str]] = [] if self.config.get('debug_mode') else None
        self.current_section_idx = -1
    
    def _log_debug(self, msg: str):
        if self.debug_log is not None: 
            self.debug_log.append(msg)
            self.status_updated.emit(msg)

    def _apply_humanization_and_compile(self):
        self._log_debug("\n=== HUMANIZATION PIPELINE ===")
        self.humanizer = Humanizer(self.config, self.debug_log)
        humanized_notes = copy.deepcopy(self.notes)
        left_hand_notes = [n for n in humanized_notes if n.hand == 'left']
        right_hand_notes = [n for n in humanized_notes if n.hand == 'right']
        unknown_notes = [n for n in humanized_notes if n.hand == 'unknown']
        self._log_debug(
            f"[PIPELINE] Input: {len(humanized_notes)} notes total | "
            f"L={len(left_hand_notes)} R={len(right_hand_notes)} Unknown={len(unknown_notes)}"
        )
        resync_points = {round(n.start_time, 2) for n in left_hand_notes}.intersection(
            {round(n.start_time, 2) for n in right_hand_notes}
        )
        self._log_debug(f"[PIPELINE] Resync points (both hands simultaneous): {len(resync_points)}")

        # Track list length so we can emit humanizer-only entries afterward
        pre_len = len(self.debug_log) if self.debug_log is not None else 0
        self.humanizer.apply_to_hand(left_hand_notes, 'left', resync_points)
        self.humanizer.apply_to_hand(right_hand_notes, 'right', resync_points)
        all_notes = sorted(left_hand_notes + right_hand_notes, key=lambda n: n.start_time)
        self.humanizer.apply_tempo_rubato(all_notes, self.sections)
        # Humanizer._log() appends to the shared list but doesn't emit — flush those entries now
        if self.debug_log is not None:
            for msg in self.debug_log[pre_len:]:
                self.status_updated.emit(msg)

        self._log_debug("\n=== COMPILATION ===")
        self._compile_event_list(all_notes, self.sections)

    def export_compiled_events(self) -> List[KeyEvent]:
        """
        Standalone compilation pipeline for generating serialization data
        without modifying or interrupting the hardware execution loop in play().
        """
        self.status_updated.emit("Compiling playback events for saving...")
        self._apply_humanization_and_compile()
        return self.compiled_events

    def load_compiled_events(self, events: List[KeyEvent], total_duration: float):
        """Load pre-compiled events for saved playback, bypassing the compilation pipeline.

        Populates key_states so the physical simulation loop can track key presses,
        and sets total_duration for the progress display.
        """
        self.compiled_events = events
        self.total_duration = total_duration
        self.key_states.clear()
        for ev in events:
            if ev.key_char not in self.key_states:
                self.key_states[ev.key_char] = KeyState(ev.key_char)

    def play_saved_events(self):
        """
        Dedicated execution branch for running pre-compiled JSON events,
        completely skipping the internal compilation pipeline.
        """
        self.status_updated.emit("Initiating saved playback sequence...")
        self.status_updated.emit(f"Successfully loaded {len(self.compiled_events)} physical execution instructions.")
        self.status_updated.emit("Playing from save!")
        self._execute_playback()

    def play(self):
        self.status_updated.emit("Initiating playback sequence...")
        self._log_debug("\n=== STARTING PLAYBACK PROCESS ===")
        self.status_updated.emit("Compiling playback events...")
        self._apply_humanization_and_compile()
        self.status_updated.emit("Playing!")
        self._execute_playback()

    def _execute_playback(self):
        """Shared playback execution: countdown → cursor loop → cleanup.

        Called by both play() (after compilation) and play_saved_events()
        (after load_compiled_events). Owns the try/except/finally so the
        pattern is defined exactly once.
        """
        try:
            if self.config.get('countdown'): self._run_countdown()
            if self.stop_event.is_set():
                self.playback_finished.emit()
                return

            self.start_time = time.perf_counter()
            self.total_paused_time = 0.0
            self.event_index = 0
            self.last_progress_emit_time = self.start_time

            self._run_cursor_loop()

        except Exception as e:
            error_msg = f"Critical Execution Error:\n{str(e)}\n\nTraceback:\n{traceback.format_exc()}"
            self.error_occurred.emit(error_msg)
            self.stop_event.set()
        finally:
            if self.stop_event.is_set():
                self.shutdown()
                self.playback_finished.emit()

    def stop(self):
        if not self.stop_event.is_set():
            self.status_updated.emit("Stopping playback...")
            self.stop_event.set()
            self.pause_event.clear()
            # shutdown() is called by _execute_playback()'s finally block once the loop exits.

    def toggle_pause(self):
        if self.pause_event.is_set():
            try:
                self.keyboard.release(Key.space)
            except Exception:
                pass

            pause_duration = time.perf_counter() - self.last_pause_timestamp
            self.total_paused_time += pause_duration
            self.pause_event.clear()
            self._log_debug(f"[PAUSE] Resuming | paused_for={pause_duration:.3f}s | total_paused={self.total_paused_time:.3f}s | event_index={self.event_index}/{len(self.compiled_events)}")
            self.status_updated.emit("Resuming...")
        else:
            self.last_pause_timestamp = time.perf_counter()
            playback_time = (self.last_pause_timestamp - self.start_time) - self.total_paused_time
            self.pause_event.set()
            self._log_debug(f"[PAUSE] Paused at playback_time={playback_time:.3f}s | event_index={self.event_index}/{len(self.compiled_events)}")
            self.status_updated.emit("Paused.")

    def seek(self, target_time: float):
        old_idx = self.event_index
        self.shutdown()
        times = [e.time for e in self.compiled_events]
        new_idx = bisect.bisect_left(times, target_time)
        self.event_index = new_idx
        self._key_net.clear()
        self._key_last_press.clear()
        self._pedal_net_down = False

        now = time.perf_counter()
        if self.pause_event.is_set():
            self.total_paused_time = 0.0
            self.start_time = now - target_time
            self.last_pause_timestamp = now
        else:
            self.start_time = now - target_time - self.total_paused_time

        self.last_progress_emit_time = now
        self.progress_updated.emit(target_time)
        remaining = len(self.compiled_events) - new_idx
        self._log_debug(
            f"[SEEK] target={target_time:.3f}s | event_index: {old_idx}->{new_idx} | "
            f"remaining={remaining} events | paused={self.pause_event.is_set()}"
        )

    def _run_countdown(self):
        self.status_updated.emit("Get ready...")
        for i in range(3, 0, -1):
            if self.stop_event.is_set(): return
            self.status_updated.emit(f"{i}...")
            time.sleep(1)

    def _compile_event_list(self, notes_to_play: List[Note], sections: List[MusicalSection]):
        self.key_states.clear()
        use_mistakes   = self.config.get('enable_mistakes', False)
        mistake_chance = self.config.get('mistake_chance', 0) / 100.0
        temp_heap      = []
        mistakes_injected = 0
        notes_unmapped = 0

        self._log_debug(f"[COMPILE] Notes to compile: {len(notes_to_play)} | Mistakes: {'ON' if use_mistakes else 'OFF'}"
                        + (f" ({mistake_chance*100:.1f}%)" if use_mistakes else ""))

        for note in notes_to_play:
            scheduled = False
            if use_mistakes and random.random() < mistake_chance:
                mistake_pitch = self._get_mistake_pitch(note.pitch)
                if mistake_pitch:
                    key_data = self.mapper.get_key_data(mistake_pitch)
                    if key_data:
                        mk_char = key_data['key']
                        heapq.heappush(temp_heap, KeyEvent(note.start_time, 2, 'press', mk_char, pitch=mistake_pitch))
                        heapq.heappush(temp_heap, KeyEvent(note.start_time + note.duration, 4, 'release', mk_char, pitch=mistake_pitch))
                        scheduled = True
                        mistakes_injected += 1

            if not scheduled:
                key_data = self.mapper.get_key_data(note.pitch)
                if key_data:
                    key_char = key_data['key']
                    heapq.heappush(temp_heap, KeyEvent(note.start_time, 2, 'press', key_char, pitch=note.pitch))
                    heapq.heappush(temp_heap, KeyEvent(note.end_time, 4, 'release', key_char, pitch=note.pitch))
                    if key_char not in self.key_states:
                        self.key_states[key_char] = KeyState(key_char)
                else:
                    notes_unmapped += 1

        self._log_debug(f"[COMPILE] Pedal style: {self.config.get('pedal_style', 'none')}")
        for event in pedal_generator.generate_events(self.config, notes_to_play, sections, self._log_debug):
            heapq.heappush(temp_heap, event)
        self.compiled_events = []
        while temp_heap:
            self.compiled_events.append(heapq.heappop(temp_heap))

        self.total_duration = self.compiled_events[-1].time if self.compiled_events else 0.0

        # Compilation summary
        press_count = sum(1 for e in self.compiled_events if e.action == 'press')
        release_count = sum(1 for e in self.compiled_events if e.action == 'release')
        pedal_down = sum(1 for e in self.compiled_events if e.action == 'pedal' and e.key_char == 'down')
        pedal_up = sum(1 for e in self.compiled_events if e.action == 'pedal' and e.key_char == 'up')

        pitches = [e.pitch for e in self.compiled_events if e.pitch is not None]
        pitch_range_str = f"{KeyMapper.pitch_to_name(min(pitches))}–{KeyMapper.pitch_to_name(max(pitches))}" if pitches else "none"

        self._log_debug(
            f"[COMPILE] Result: {len(self.compiled_events)} events | "
            f"press={press_count} release={release_count} pedal_down={pedal_down} pedal_up={pedal_up}"
        )
        self._log_debug(
            f"[COMPILE] Duration: {self.total_duration:.2f}s | Pitch range: {pitch_range_str} | "
            f"Unique keys: {len(self.key_states)} | Mistakes: {mistakes_injected} | Unmapped: {notes_unmapped}"
        )
        if self.total_duration > 0:
            self._log_debug(
                f"[COMPILE] Density: {press_count / self.total_duration:.1f} presses/sec | "
                f"{pedal_down / self.total_duration:.2f} pedal-downs/sec"
            )
            
    def _get_mistake_pitch(self, original_pitch: int) -> Optional[int]:
        candidates = [original_pitch + d for d in (-2, -1, 1, 2)]
        if KeyMapper.is_black_key(original_pitch):
            black_pool = [p for p in candidates if KeyMapper.is_black_key(p)]
            white_pool = [p for p in candidates if not KeyMapper.is_black_key(p)]
            pool = (black_pool if random.random() < 0.5 else white_pool) or black_pool or white_pool
            return random.choice(pool) if pool else None
        valid = [p for p in candidates if not KeyMapper.is_black_key(p)]
        return random.choice(valid) if valid else None

    def _run_cursor_loop(self):
        self._log_debug("\n=== ENTERING CURSOR LOOP ===")
        self.current_section_idx = -1
        _was_paused = False
        self._key_net.clear()
        self._key_last_press.clear()
        self._pedal_net_down = False

        while not self.stop_event.is_set():
            if self.pause_event.is_set():
                if not _was_paused:
                    # First pause iteration: safe to release now — cursor loop owns all key presses
                    self.shutdown()
                    _was_paused = True
                time.sleep(0.05)
                continue

            if _was_paused:
                # Just unpaused — re-press any notes that were mid-play at the pause point
                self._sync_active_keys_at_resume()
                _was_paused = False

            now = time.perf_counter()
            playback_time = (now - self.start_time) - self.total_paused_time
            
            next_sec_idx = self.current_section_idx + 1
            if self.sections and next_sec_idx < len(self.sections):
                if playback_time >= self.sections[next_sec_idx].start_time:
                    self.current_section_idx = next_sec_idx
                    sec = self.sections[next_sec_idx]
                    self._log_debug(f"\n--- SECTION {next_sec_idx} | Time: {sec.start_time:.2f}s | Style: {sec.articulation_label.upper()} ---")

            if self.event_index >= len(self.compiled_events):
                if playback_time > self.total_duration + 0.1:
                    if not self.pause_event.is_set():
                        self.last_pause_timestamp = now
                        self.pause_event.set()
                        self._log_debug(
                            f"[AUTO-PAUSE] End of timeline reached at {playback_time:.3f}s | "
                            f"duration={self.total_duration:.3f}s | "
                            f"events processed={self.event_index}/{len(self.compiled_events)}"
                        )
                        self.shutdown()
                        self.auto_paused.emit()
                        self.status_updated.emit("Playback finished. Paused.")
                    time.sleep(0.1)
                    continue
                else:
                    time.sleep(0.001)
                    continue

            next_event = self.compiled_events[self.event_index]
            
            if next_event.time <= playback_time:
                batch = []
                while self.event_index < len(self.compiled_events):
                    e = self.compiled_events[self.event_index]
                    if e.time <= playback_time:
                        batch.append(e)
                        self.event_index += 1
                    else:
                        break

                batch.sort(key=lambda x: x.priority)
                self._execute_chord_event(batch, playback_time)
            else:
                sleep_time = min(next_event.time - playback_time - 0.001, self.progress_update_interval)
                time.sleep(max(0.0005, sleep_time))
                # Refresh after sleep so the cursor emits current time, not pre-sleep time
                now = time.perf_counter()
                playback_time = (now - self.start_time) - self.total_paused_time

            if now - self.last_progress_emit_time >= self.progress_update_interval:
                self.progress_updated.emit(playback_time)
                self.last_progress_emit_time = now

    def _get_press_info_from_event(self, event: KeyEvent) -> Tuple[List[Key], str]:
        if event.pitch is None: return [], event.key_char
        key_data = self.mapper.get_key_data(event.pitch)
        if not key_data: return [], event.key_char
        return key_data['modifiers'], key_data['key']
        
    def _execute_chord_event(self, events: List[KeyEvent], playback_time: float):
        if self.stop_event.is_set(): return
        press_events = [e for e in events if e.action == 'press']
        release_events = [e for e in events if e.action == 'release']
        pedal_events = [e for e in events if e.action == 'pedal']

        state_changed = False 

        for event in pedal_events:
            physical = self._handle_pedal_event(event)
            self._log_debug(
                f"[ACT] {playback_time:.4f}s | PEDAL {event.key_char.upper():3s} | "
                f"[PHYSICAL] {physical} (Delta: {playback_time - event.time:+.4f}s)"
            )
            new_pedal_state = (event.key_char == 'down')
            if new_pedal_state != self._pedal_net_down:
                self._pedal_net_down = new_pedal_state
                self.pedal_updated.emit(self._pedal_net_down)
            else:
                self._pedal_net_down = new_pedal_state

        for event in release_events:
            self._key_net[event.key_char] = self._key_net.get(event.key_char, 0) - 1
            net = self._key_net.get(event.key_char, 0)
            if event.pitch is not None:
                self.active_pitches.discard(event.pitch)
                state_changed = True

            key_char = event.key_char
            state = self.key_states.get(key_char)
            if not state: continue

            # Only physically release when no other notes still need this key
            if net <= 0:
                base_key = key_char
                if key_char in self.mapper.SYMBOL_MAP: base_key = self.mapper.SYMBOL_MAP[key_char]
                state.release()
                try:
                    self.keyboard.release(base_key)
                    self._log_debug(f"[ACT] {playback_time:.4f}s | RELEASE | {event.key_char} | [PHYSICAL] Released '{base_key}'")
                except Exception as e:
                    self._log_debug(f"[ACT] {playback_time:.4f}s | RELEASE | {event.key_char} | [PHYSICAL FAILURE] {e}")
            else:
                self._log_debug(f"[ACT] {playback_time:.4f}s | RELEASE | {event.key_char} | Held (net={net})")

        for event in press_events:
            self._key_net[event.key_char] = self._key_net.get(event.key_char, 0) + 1
            self._key_last_press[event.key_char] = event
            if event.pitch is not None:
                self.active_pitches.add(event.pitch)
                state_changed = True

            state = self.key_states.get(event.key_char)
            if not state or event.pitch is None: continue

            modifiers, base_key = self._get_press_info_from_event(event)

            was_physically_down = state.is_physically_down
            state.press()

            try:
                with self.keyboard.pressed(*modifiers):
                    if was_physically_down:
                        # Key already held by an overlapping note — re-strike for new attack
                        self.keyboard.release(base_key)
                        time.sleep(0.001)
                        self.keyboard.press(base_key)
                        self._log_debug(f"[ACT] {playback_time:.4f}s | PRESS   | {event.key_char} | [PHYSICAL] Re-struck '{base_key}' (overlap)")
                    else:
                        self.keyboard.press(base_key)
                        self._log_debug(f"[ACT] {playback_time:.4f}s | PRESS   | {event.key_char} | [PHYSICAL] Pressed '{base_key}' {modifiers}")
            except Exception as e:
                self._log_debug(f"[ACT] {playback_time:.4f}s | PRESS   | {event.key_char} | [PHYSICAL FAILURE] {e}")

        if state_changed:
            self.visualizer_updated.emit(list(self.active_pitches))

    def _handle_pedal_event(self, event: KeyEvent) -> str:
        """Drive Space for the pedal and return a short physical-status string
        for the caller's [ACT] log line. Returns one of: 'Pressed Space',
        'Released Space', 'Already down (no-op)', 'Already up (no-op)',
        'Stopped (no-op)', or 'FAILED: <err>'."""
        if self.stop_event.is_set():
            return "Stopped (no-op)"
        if event.key_char == 'down':
            if self.pedal_is_down:
                return "Already down (no-op)"
            self.pedal_is_down = True
            try:
                self.keyboard.press(Key.space)
                return "Pressed Space"
            except Exception as e:
                return f"FAILED: {e}"
        elif event.key_char == 'up':
            if not self.pedal_is_down:
                return "Already up (no-op)"
            self.pedal_is_down = False
            try:
                self.keyboard.release(Key.space)
                return "Released Space"
            except Exception as e:
                return f"FAILED: {e}"
        return "Unknown action"

    def _sync_active_keys_at_resume(self):
        """Re-press any notes/pedal that were physically held at the moment of pause.

        Uses the running _key_net / _key_last_press counters maintained by
        _execute_chord_event — O(currently-held keys) instead of O(all events).
        """
        pitch_net: Dict[int, int] = {}
        keys_repressed = []
        for key_char, count in self._key_net.items():
            if count > 0 and key_char in self.key_states:
                press_event = self._key_last_press.get(key_char)
                if press_event is None:
                    continue
                if press_event.pitch is not None:
                    pitch_net[press_event.pitch] = pitch_net.get(press_event.pitch, 0) + 1
                modifiers, base_key = self._get_press_info_from_event(press_event)
                self.key_states[key_char].press()
                try:
                    with self.keyboard.pressed(*modifiers):
                        self.keyboard.press(base_key)
                    keys_repressed.append(f"{base_key}(p={press_event.pitch})")
                except Exception:
                    pass

        self.active_pitches = {p for p, c in pitch_net.items() if c > 0}
        self.visualizer_updated.emit(list(self.active_pitches))

        pedal_restored = False
        if self._pedal_net_down and not self.pedal_is_down:
            self.pedal_is_down = True
            pedal_restored = True
            try:
                self.keyboard.press(Key.space)
            except Exception:
                pass

        self._log_debug(
            f"[RESUME] Re-pressed {len(keys_repressed)} keys: [{', '.join(keys_repressed)}] | "
            f"pedal_restored={pedal_restored} | active_pitches={len(self.active_pitches)}"
        )

    def shutdown(self):
        active_keys = [k for k, s in self.key_states.items() if s.is_active]
        self._log_debug(f"[SHUTDOWN] Releasing {len(active_keys)} active keys, pedal_down={self.pedal_is_down}")
        self.status_updated.emit("Releasing all keys...")
        for key_char, state in self.key_states.items():
            try:
                base_key = key_char
                if key_char in self.mapper.SYMBOL_MAP: base_key = self.mapper.SYMBOL_MAP[key_char]
                if state.is_active:
                    self.keyboard.release(base_key)
                state.release()
            except Exception: pass

        if self.pedal_is_down:
            try: self.keyboard.release(Key.space)
            except Exception: pass
            self.pedal_is_down = False
        for key in [Key.shift, Key.ctrl, Key.alt]:
            try: self.keyboard.release(key)
            except Exception: pass
        self.status_updated.emit("Shutdown complete.")