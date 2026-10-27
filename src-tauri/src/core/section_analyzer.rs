use crate::core::midi::{get_time_group_indices, TempoMap};
use crate::core::models::{MusicalSection, Note};

pub fn assign_hands(notes: &mut [Note]) {
    let groups = get_time_group_indices(notes);
    for group in groups {
        let unassigned: Vec<usize> = group
            .iter()
            .copied()
            .filter(|&i| notes[i].hand == "unknown")
            .collect();
        if unassigned.is_empty() {
            continue;
        }
        let avg_pitch: f64 =
            unassigned.iter().map(|&i| notes[i].pitch as f64).sum::<f64>() / unassigned.len() as f64;
        let hand = if avg_pitch < 60.0 { "left" } else { "right" };
        for &i in &unassigned {
            notes[i].hand = hand.to_string();
        }
    }
}

pub struct SectionAnalyzer<'a> {
    notes: Vec<Note>,
    note_times: Vec<f64>,
    tempo_map: &'a TempoMap,
}

impl<'a> SectionAnalyzer<'a> {
    pub fn new(mut notes: Vec<Note>, tempo_map: &'a TempoMap) -> Self {
        notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        let note_times = notes.iter().map(|n| n.start_time).collect();
        SectionAnalyzer {
            notes,
            note_times,
            tempo_map,
        }
    }

    pub fn analyze(&self) -> Vec<MusicalSection> {
        if self.notes.is_empty() {
            return Vec::new();
        }
        if self.tempo_map.has_explicit_time_signatures {
            self.analyze_by_measures()
        } else {
            self.analyze_by_silence()
        }
    }

    fn analyze_by_silence(&self) -> Vec<MusicalSection> {
        let boundaries = self.detect_grand_pauses();
        let mut sections = Vec::new();
        if boundaries.len() < 2 {
            return sections;
        }
        for i in 0..boundaries.len() - 1 {
            let start_idx = boundaries[i];
            if boundaries[i + 1] == 0 {
                continue;
            }
            let end_idx = boundaries[i + 1] - 1;
            if start_idx > end_idx {
                continue;
            }
            let sec_notes: Vec<Note> = self.notes[start_idx..=end_idx].to_vec();
            if sec_notes.is_empty() {
                continue;
            }
            let start_time = sec_notes[0].start_time;
            let end_time = sec_notes
                .iter()
                .map(|n| n.end_time())
                .fold(f64::MIN, f64::max);
            let start_beat = self.tempo_map.time_to_beat(start_time);
            let end_beat = self.tempo_map.time_to_beat(end_time);
            let articulation_label = self.classify_bass_articulation(&sec_notes);
            let pace_label = Self::classify_pace_beats(&sec_notes, start_beat, end_beat);
            sections.push(MusicalSection {
                start_time,
                end_time,
                notes: sec_notes,
                articulation_label,
                pace_label,
                start_beat,
                end_beat,
            });
        }
        sections
    }

    fn analyze_by_measures(&self) -> Vec<MusicalSection> {
        let total_dur = self
            .notes
            .iter()
            .map(|n| n.end_time())
            .fold(f64::MIN, f64::max);
        let measures = self.tempo_map.get_measure_boundaries(total_dur);
        let mut sections = Vec::new();
        let mut current_section_start = measures.first().map(|m| m.0).unwrap_or(0.0);
        let mut current_notes_in_section: Vec<Note> = Vec::new();
        let mut prev_style: Option<String> = None;
        let mut prev_pace: Option<String> = None;

        for &(m_start, m_end) in &measures {
            let lo = self.note_times.partition_point(|&t| t < m_start);
            let hi = self.note_times.partition_point(|&t| t < m_end);
            let notes_in_measure: Vec<Note> = self.notes[lo..hi].to_vec();

            let (style, pace) = if notes_in_measure.is_empty() {
                (
                    prev_style.clone().unwrap_or_else(|| "legato".to_string()),
                    prev_pace.clone().unwrap_or_else(|| "normal".to_string()),
                )
            } else {
                let s_beat = self.tempo_map.time_to_beat(m_start);
                let e_beat = self.tempo_map.time_to_beat(m_end);
                let art = self.classify_bass_articulation(&notes_in_measure);
                let pc = Self::classify_pace_beats(&notes_in_measure, s_beat, e_beat);
                (art, pc)
            };

            if prev_style.is_none() {
                prev_style = Some(style);
                prev_pace = Some(pace);
                current_notes_in_section.extend(notes_in_measure);
                continue;
            }

            if Some(&style) != prev_style.as_ref() {
                if !current_notes_in_section.is_empty() {
                    let sec_end = m_start;
                    let s_beat = self.tempo_map.time_to_beat(current_section_start);
                    let e_beat = self.tempo_map.time_to_beat(sec_end);
                    sections.push(MusicalSection {
                        start_time: current_section_start,
                        end_time: sec_end,
                        notes: std::mem::take(&mut current_notes_in_section),
                        articulation_label: prev_style.clone().unwrap(),
                        pace_label: prev_pace.clone().unwrap(),
                        start_beat: s_beat,
                        end_beat: e_beat,
                    });
                }
                current_section_start = m_start;
                prev_style = Some(style);
                prev_pace = Some(pace);
            }
            current_notes_in_section.extend(notes_in_measure);
        }

        if !current_notes_in_section.is_empty() {
            let sec_end = measures.last().unwrap().1;
            let s_beat = self.tempo_map.time_to_beat(current_section_start);
            let e_beat = self.tempo_map.time_to_beat(sec_end);
            sections.push(MusicalSection {
                start_time: current_section_start,
                end_time: sec_end,
                notes: current_notes_in_section,
                articulation_label: prev_style.unwrap_or_else(|| "legato".to_string()),
                pace_label: prev_pace.unwrap_or_else(|| "normal".to_string()),
                start_beat: s_beat,
                end_beat: e_beat,
            });
        }
        sections
    }

    fn detect_grand_pauses(&self) -> Vec<usize> {
        let mut indices = vec![0];
        if self.notes.is_empty() {
            return indices;
        }
        let mut last_end_time = self.notes[0].end_time();
        for i in 1..self.notes.len() {
            let current_start = self.notes[i].start_time;
            let gap_sec = current_start - last_end_time;
            let tempo = self.tempo_map.get_tempo_at(last_end_time);
            let sec_per_beat = tempo as f64 / 1_000_000.0;
            let gap_beats = gap_sec / sec_per_beat;
            if gap_beats > 2.0 {
                indices.push(i);
            }
            last_end_time = last_end_time.max(self.notes[i].end_time());
        }
        indices.push(self.notes.len());
        indices
    }

    fn classify_bass_articulation(&self, notes: &[Note]) -> String {
        let mut lh_notes: Vec<&Note> = notes.iter().filter(|n| n.hand == "left").collect();
        if lh_notes.len() < 2 {
            return "legato".to_string();
        }
        lh_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        let mut total_overlap = 0.0;
        let mut total_possible = 0.0;
        for i in 0..lh_notes.len() - 1 {
            let curr = lh_notes[i];
            let next_n = lh_notes[i + 1];
            let curr_beat = self.tempo_map.time_to_beat(curr.start_time);
            let next_beat = self.tempo_map.time_to_beat(next_n.start_time);
            let ioi_beats = next_beat - curr_beat;
            if ioi_beats <= 0.0 {
                continue;
            }
            let dur_beats = self.tempo_map.time_to_beat(curr.end_time()) - curr_beat;
            let ratio = dur_beats / ioi_beats;
            total_overlap += ratio.min(1.2);
            total_possible += 1.0;
        }
        if total_possible == 0.0 {
            return "legato".to_string();
        }
        let avg_ratio = total_overlap / total_possible;
        if avg_ratio >= 0.95 {
            "legato".to_string()
        } else if avg_ratio <= 0.60 {
            "staccato".to_string()
        } else {
            "hybrid".to_string()
        }
    }

    fn classify_pace_beats(notes: &[Note], start_beat: f64, end_beat: f64) -> String {
        let duration_beats = end_beat - start_beat;
        if duration_beats <= 0.0 {
            return "normal".to_string();
        }
        let npb = notes.len() as f64 / duration_beats;
        if npb > 3.5 {
            "fast".to_string()
        } else if npb < 1.0 {
            "slow".to_string()
        } else {
            "normal".to_string()
        }
    }
}

#[cfg(test)]
mod tests;
