import bisect
from typing import List

from core.models import Note, MusicalSection
from core.core import TempoMap, get_time_groups


def assign_hands(notes: List[Note]):
    """Assign left/right hand to unassigned notes based on pitch and timing groups.

    Replaces the FingeringEngine class. Groups simultaneous notes together and
    assigns a hand to each group based on average pitch relative to middle C (60).
    """
    time_groups = get_time_groups(notes)
    for group in time_groups:
        unassigned = [n for n in group if n.hand == 'unknown']
        if not unassigned:
            continue
        avg_pitch = sum(n.pitch for n in unassigned) / len(unassigned)
        hand = 'left' if avg_pitch < 60 else 'right'
        for n in unassigned:
            n.hand = hand


class SectionAnalyzer:
    def __init__(self, notes: List[Note], tempo_map: TempoMap, debug_log=None):
        self.notes = sorted(notes, key=lambda n: n.start_time)
        self._note_times = [n.start_time for n in self.notes]
        self.tempo_map = tempo_map
        self._log = debug_log

    def _debug(self, msg: str):
        if self._log is not None:
            self._log(msg)

    def analyze(self) -> List[MusicalSection]:
        if not self.notes:
            self._debug("[SECTIONS] No notes, returning empty section list")
            return []
        method = "measures (explicit time signatures)" if self.tempo_map.has_explicit_time_signatures else "silence gaps"
        self._debug(f"\n=== SECTION ANALYSIS ({method}) ===")
        self._debug(f"[SECTIONS] Input: {len(self.notes)} notes, time span: {self.notes[0].start_time:.2f}s – {self.notes[-1].end_time:.2f}s")
        if self.tempo_map.has_explicit_time_signatures:
            sections = self._analyze_by_measures()
        else:
            sections = self._analyze_by_silence()
        self._debug(f"[SECTIONS] Result: {len(sections)} sections")
        for i, sec in enumerate(sections):
            self._debug(
                f"  Section {i}: {sec.start_time:.2f}s–{sec.end_time:.2f}s | "
                f"{len(sec.notes)} notes | {sec.articulation_label} | pace={sec.pace_label} | "
                f"beats {sec.start_beat:.1f}–{sec.end_beat:.1f}"
            )
        return sections

    def _analyze_by_silence(self) -> List[MusicalSection]:
        boundaries = self._detect_grand_pauses()
        sections = []
        for i in range(len(boundaries) - 1):
            start_idx = boundaries[i]
            end_idx = boundaries[i+1] - 1
            if start_idx > end_idx: continue
            sec_notes = self.notes[start_idx : end_idx+1]
            if not sec_notes: continue
            start_time = sec_notes[0].start_time
            end_time = max(n.end_time for n in sec_notes)
            start_beat = self.tempo_map.time_to_beat(start_time)
            end_beat = self.tempo_map.time_to_beat(end_time)
            articulation = self._classify_bass_articulation(sec_notes)
            pace = self._classify_pace_beats(sec_notes, start_beat, end_beat)
            sections.append(MusicalSection(start_time, end_time, sec_notes, articulation, pace, start_beat, end_beat))
        return sections

    def _analyze_by_measures(self) -> List[MusicalSection]:
        total_dur = max(n.end_time for n in self.notes)
        measures = self.tempo_map.get_measure_boundaries(total_dur)
        sections = []
        current_section_start = measures[0][0] if measures else 0
        current_notes_in_section = []
        prev_style = None
        prev_pace = None

        def classify_chunk(chunk_notes, s_time, e_time):
            s_beat = self.tempo_map.time_to_beat(s_time)
            e_beat = self.tempo_map.time_to_beat(e_time)
            art = self._classify_bass_articulation(chunk_notes)
            pace = self._classify_pace_beats(chunk_notes, s_beat, e_beat)
            return art, pace

        for i, (m_start, m_end) in enumerate(measures):
            lo = bisect.bisect_left(self._note_times, m_start)
            hi = bisect.bisect_left(self._note_times, m_end)
            notes_in_measure = self.notes[lo:hi]
            if not notes_in_measure:
                style, pace = (prev_style or 'legato'), (prev_pace or 'normal')
            else:
                style, pace = classify_chunk(notes_in_measure, m_start, m_end)
            if prev_style is None:
                prev_style = style
                prev_pace = pace
                current_notes_in_section.extend(notes_in_measure)
                continue
            if style != prev_style:
                if current_notes_in_section:
                    sec_end = m_start
                    s_beat = self.tempo_map.time_to_beat(current_section_start)
                    e_beat = self.tempo_map.time_to_beat(sec_end)
                    sections.append(MusicalSection(current_section_start, sec_end, list(current_notes_in_section), prev_style, prev_pace, s_beat, e_beat))
                current_section_start = m_start
                current_notes_in_section = []
                prev_style = style
                prev_pace = pace
            current_notes_in_section.extend(notes_in_measure)

        if current_notes_in_section:
            sec_end = measures[-1][1]
            s_beat = self.tempo_map.time_to_beat(current_section_start)
            e_beat = self.tempo_map.time_to_beat(sec_end)
            sections.append(MusicalSection(current_section_start, sec_end, list(current_notes_in_section), prev_style, prev_pace, s_beat, e_beat))
        return sections

    def _detect_grand_pauses(self) -> List[int]:
        indices = [0]
        if not self.notes: return indices
        last_end_time = self.notes[0].end_time
        for i in range(1, len(self.notes)):
            current_start = self.notes[i].start_time
            gap_sec = current_start - last_end_time
            tempo = self.tempo_map.get_tempo_at(last_end_time)
            sec_per_beat = tempo / 1_000_000.0
            gap_beats = gap_sec / sec_per_beat
            if gap_beats > 2.0:
                indices.append(i)
            last_end_time = max(last_end_time, self.notes[i].end_time)
        indices.append(len(self.notes))
        return indices

    def _classify_bass_articulation(self, notes: List[Note]) -> str:
        lh_notes = [n for n in notes if n.hand == 'left']
        if len(lh_notes) < 2: return 'legato'
        total_overlap = 0.0
        total_possible = 0.0
        lh_notes.sort(key=lambda n: n.start_time)
        for i in range(len(lh_notes) - 1):
            curr = lh_notes[i]
            next_n = lh_notes[i+1]
            curr_beat = self.tempo_map.time_to_beat(curr.start_time)
            next_beat = self.tempo_map.time_to_beat(next_n.start_time)
            ioi_beats = next_beat - curr_beat
            if ioi_beats <= 0: continue
            dur_beats = self.tempo_map.time_to_beat(curr.end_time) - curr_beat
            ratio = dur_beats / ioi_beats
            total_overlap += min(ratio, 1.2)
            total_possible += 1.0
        if total_possible == 0: return 'legato'
        avg_ratio = total_overlap / total_possible
        if avg_ratio >= 0.95: return 'legato'
        if avg_ratio <= 0.60: return 'staccato'
        return 'hybrid'

    def _classify_pace_beats(self, notes: List[Note], start_beat: float, end_beat: float) -> str:
        duration_beats = end_beat - start_beat
        if duration_beats <= 0: return 'normal'
        npb = len(notes) / duration_beats
        if npb > 3.5: return 'fast'
        if npb < 1.0: return 'slow'
        return 'normal'
