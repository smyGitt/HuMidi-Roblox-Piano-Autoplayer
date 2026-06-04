import random
import numpy as np
from typing import List, Set, Dict, Optional

from core.models import Note, MusicalSection
from core.core import get_time_groups

# Independent per-group noise injected into hand drift (std dev in seconds).
# This gives Hand Drift a meaningful effect even when Vary Timing is off.
_DRIFT_NOISE_SIGMA = 0.004


class Humanizer:
    def __init__(self, config: Dict, debug_log: Optional[List[str]] = None):
        self.config = config
        self.debug_log = debug_log
        self.left_hand_drift = 0.0
        self.right_hand_drift = 0.0

    def _log(self, msg: str):
        if self.debug_log is not None:
            self.debug_log.append(msg)

    def apply_to_hand(self, notes: List[Note], hand: str, resync_points: Set[float]):
        vary_timing       = self.config.get('vary_timing', False)
        vary_articulation = self.config.get('vary_articulation', False)
        enable_drift      = self.config.get('enable_drift_correction', False)
        enable_chord_roll = self.config.get('enable_chord_roll', False)

        active_features = [name for name, on in [
            ('Vary Timing', vary_timing), ('Vary Articulation', vary_articulation),
            ('Drift Correction', enable_drift), ('Chord Roll', enable_chord_roll)
        ] if on]

        if not active_features:
            self._log(f"[HUMANIZE] {hand.upper()} hand: No features active, skipping {len(notes)} notes")
            return

        time_groups = get_time_groups(notes)
        self._log(
            f"[HUMANIZE] {hand.upper()} hand: {len(notes)} notes in {len(time_groups)} groups | "
            f"Active: {', '.join(active_features)}"
        )

        timing_sigma = self.config.get('timing_variance', 0.01) if vary_timing else 0
        chord_roll_count = 0
        resync_count = 0
        max_timing_offset = 0.0
        max_drift = 0.0

        for group in time_groups:
            is_resync = round(group[0].start_time, 2) in resync_points

            # At resync points (simultaneous notes in both hands), decay accumulated drift.
            if enable_drift and is_resync:
                decay = self.config.get('drift_decay_factor', 0.25)
                if hand == 'left':
                    self.left_hand_drift *= decay
                else:
                    self.right_hand_drift *= decay
                resync_count += 1

            # Per-group timing offset: Gaussian noise clamped to ±3σ.
            timing_offset = 0.0
            if vary_timing:
                timing_offset = random.gauss(0, timing_sigma)
                timing_offset = max(-3 * timing_sigma, min(3 * timing_sigma, timing_offset))
                max_timing_offset = max(max_timing_offset, abs(timing_offset))

            # Chord roll: stagger simultaneous notes with random speed and direction.
            if enable_chord_roll and len(group) > 1:
                ascending = random.random() > 0.2   # mostly low-to-high, occasionally reversed
                stagger   = random.uniform(0.004, 0.010)  # 4–10 ms per step
                group.sort(key=lambda n: n.pitch, reverse=not ascending)
                for i, note in enumerate(group):
                    note.start_time += i * stagger
                chord_roll_count += 1

            # Articulation: scale note duration only when explicitly enabled.
            articulation_scale = None
            if vary_articulation:
                base = self.config.get('articulation', 0.95)
                articulation_scale = base - random.random() * 0.1

            # Apply timing offset and current drift to every note in the group.
            # Drift is read *before* updating so this group uses the previously
            # accumulated value; the new noise feeds into the next group.
            current_drift = self.left_hand_drift if hand == 'left' else self.right_hand_drift
            max_drift = max(max_drift, abs(current_drift))
            for note in group:
                note.start_time += timing_offset
                if enable_drift:
                    note.start_time += current_drift
                if articulation_scale is not None:
                    note.duration = max(0.03, note.duration * articulation_scale)

            # Accumulate independent drift noise for the next group.
            if enable_drift:
                drift_noise = random.gauss(0, _DRIFT_NOISE_SIGMA)
                if hand == 'left':
                    self.left_hand_drift += drift_noise
                else:
                    self.right_hand_drift += drift_noise

        # Summary stats
        summary_parts = []
        if vary_timing:
            summary_parts.append(f"max_timing_offset={max_timing_offset*1000:.1f}ms (sigma={timing_sigma*1000:.1f}ms)")
        if enable_drift:
            final_drift = self.left_hand_drift if hand == 'left' else self.right_hand_drift
            summary_parts.append(f"final_drift={final_drift*1000:.2f}ms, peak_drift={max_drift*1000:.2f}ms, resyncs={resync_count}")
        if enable_chord_roll:
            summary_parts.append(f"chord_rolls={chord_roll_count}")
        if vary_articulation:
            summary_parts.append(f"articulation_base={self.config.get('articulation', 0.95):.2f}")
        self._log(f"[HUMANIZE] {hand.upper()} hand result: {' | '.join(summary_parts)}")

    def apply_tempo_rubato(self, all_notes: List[Note], sections: List[MusicalSection]):
        if not self.config.get('enable_tempo_sway'):
            self._log("[RUBATO] Tempo sway disabled, skipping")
            return

        base_intensity = self.config.get('tempo_sway_intensity', 0.0)
        invert_sway    = self.config.get('invert_tempo_sway', False)
        note_map       = {note.id: note for note in all_notes}

        self._log(f"[RUBATO] Applying tempo sway: intensity={base_intensity:.4f}, inverted={invert_sway}, sections={len(sections)}")

        total_shifted = 0
        max_shift = 0.0

        for section in sections:
            section_duration = section.end_time - section.start_time
            if section_duration < 1.0:
                continue

            # Fast sections sway less (they're already expressive); slow sections sway more.
            # Invert reverses this relationship.
            if section.pace_label == 'fast':
                pace_multiplier = 1.5 if invert_sway else 0.25
            elif section.pace_label == 'slow':
                pace_multiplier = 0.25 if invert_sway else 1.5
            else:
                pace_multiplier = 1.0

            intensity = base_intensity * pace_multiplier
            section_shifted = 0
            section_max_shift = 0.0
            for note in section.notes:
                if note.id in note_map:
                    rel_pos = (note.start_time - section.start_time) / section_duration
                    # sin(0..π) creates a bow: no shift at boundaries, max shift at midpoint.
                    # Subtracting pulls the midpoint earlier, producing a subtle acceleration arc.
                    shift = np.sin(rel_pos * np.pi) * intensity
                    note_map[note.id].start_time -= shift
                    section_shifted += 1
                    section_max_shift = max(section_max_shift, abs(shift))

            total_shifted += section_shifted
            max_shift = max(max_shift, section_max_shift)

        self._log(f"[RUBATO] Done: {total_shifted} notes shifted, max_shift={max_shift*1000:.2f}ms")
