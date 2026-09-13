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
mod tests {
    use super::*;

    fn note(id: i32, pitch: i32, start_time: f64, duration: f64, hand: &str) -> Note {
        Note {
            id,
            pitch,
            velocity: 64,
            start_time,
            duration,
            hand: hand.to_string(),
            original_track_index: -1,
            channel: -1,
        }
    }

    fn default_tm() -> TempoMap {
        TempoMap::new(vec![], vec![])
    }

    mod test_assign_hands {
        use super::*;

        #[test]
        fn test_all_below_60_become_left() {
            let mut notes: Vec<Note> = (0..5)
                .map(|i| note(i, 40 + i, i as f64 * 10.0, 0.5, "unknown"))
                .collect();
            assign_hands(&mut notes);
            assert!(notes.iter().all(|n| n.hand == "left"));
        }

        #[test]
        fn test_all_above_60_become_right() {
            let mut notes: Vec<Note> = (0..5)
                .map(|i| note(i, 65 + i, i as f64 * 10.0, 0.5, "unknown"))
                .collect();
            assign_hands(&mut notes);
            assert!(notes.iter().all(|n| n.hand == "right"));
        }

        #[test]
        fn test_pitch_60_assigned_right() {
            let mut notes = vec![note(0, 60, 0.0, 0.5, "unknown")];
            assign_hands(&mut notes);
            assert_eq!(notes[0].hand, "right");
        }

        #[test]
        fn test_pitch_59_assigned_left() {
            let mut notes = vec![note(0, 59, 0.0, 0.5, "unknown")];
            assign_hands(&mut notes);
            assert_eq!(notes[0].hand, "left");
        }

        #[test]
        fn test_already_assigned_note_unchanged() {
            let mut notes = vec![note(0, 40, 0.0, 0.5, "right")];
            assign_hands(&mut notes);
            assert_eq!(notes[0].hand, "right");
        }

        #[test]
        fn test_simultaneous_group_uses_average() {
            let mut notes = vec![
                note(0, 50, 0.0, 0.5, "unknown"),
                note(1, 70, 0.0, 0.5, "unknown"),
            ];
            assign_hands(&mut notes);
            assert_eq!(notes[0].hand, "right");
            assert_eq!(notes[1].hand, "right");
        }

        #[test]
        fn test_simultaneous_group_below_avg() {
            let mut notes = vec![
                note(0, 40, 0.0, 0.5, "unknown"),
                note(1, 50, 0.0, 0.5, "unknown"),
            ];
            assign_hands(&mut notes);
            assert_eq!(notes[0].hand, "left");
            assert_eq!(notes[1].hand, "left");
        }

        #[test]
        fn test_empty_notes_no_error() {
            let mut notes: Vec<Note> = vec![];
            assign_hands(&mut notes);
        }

        #[test]
        fn test_mixed_group_uses_unassigned_only_for_average() {
            let mut notes = vec![
                note(0, 80, 0.0, 0.5, "left"),
                note(1, 40, 0.0, 0.5, "unknown"),
            ];
            assign_hands(&mut notes);
            assert_eq!(notes[0].hand, "left");
            assert_eq!(notes[1].hand, "left");
        }
    }

    mod test_analyze {
        use super::*;

        #[test]
        fn test_empty_notes_returns_empty() {
            let tm = default_tm();
            assert!(SectionAnalyzer::new(vec![], &tm).analyze().is_empty());
        }

        #[test]
        fn test_single_note_produces_one_section() {
            let tm = default_tm();
            let notes = vec![note(0, 60, 0.0, 1.0, "left")];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(sections.len(), 1);
        }

        #[test]
        fn test_section_spans_note_range() {
            let tm = default_tm();
            let notes: Vec<Note> = (0..4)
                .map(|i| note(i, 60, i as f64 * 0.5, 0.5, "left"))
                .collect();
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            let total: usize = sections.iter().map(|s| s.notes.len()).sum();
            assert_eq!(total, 4);
        }

        #[test]
        fn test_grand_pause_produces_two_sections() {
            let tm = default_tm();
            let notes = vec![
                note(0, 60, 0.0, 0.2, "left"),
                note(1, 60, 0.3, 0.2, "left"),
                note(2, 60, 3.0, 0.2, "left"),
                note(3, 60, 3.3, 0.2, "left"),
            ];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(sections.len(), 2);
            assert_eq!(sections[0].notes.len(), 2);
            assert_eq!(sections[1].notes.len(), 2);
        }

        #[test]
        fn test_analyze_dispatches_to_measures_when_explicit_time_signature() {
            let tm = TempoMap::new(vec![], vec![(0.0, 4, 4), (100.0, 3, 4)]);
            assert!(tm.has_explicit_time_signatures);
            let notes = vec![
                note(0, 60, 0.0, 0.2, "left"),
                note(1, 60, 0.3, 0.2, "left"),
                note(2, 60, 3.0, 0.2, "left"),
                note(3, 60, 3.3, 0.2, "left"),
            ];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(
                sections.len(),
                1,
                "measures dispatch ignores silence gaps and should merge same-style measures"
            );
        }
    }

    mod test_classify_pace_beats {
        use super::*;

        #[test]
        fn test_fast() {
            let notes: Vec<Note> = (0..8).map(|i| note(i, 60, 0.0, 0.5, "left")).collect();
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 0.0, 1.4),
                "fast"
            );
        }

        #[test]
        fn test_slow() {
            let notes: Vec<Note> = (0..3).map(|i| note(i, 60, 0.0, 0.5, "left")).collect();
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 0.0, 10.0),
                "slow"
            );
        }

        #[test]
        fn test_normal() {
            let notes: Vec<Note> = (0..6).map(|i| note(i, 60, 0.0, 0.5, "left")).collect();
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 0.0, 4.0),
                "normal"
            );
        }

        #[test]
        fn test_zero_duration_returns_normal() {
            let notes = vec![note(0, 60, 0.0, 0.5, "left")];
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 1.0, 1.0),
                "normal"
            );
        }

        #[test]
        fn test_negative_duration_returns_normal() {
            let notes = vec![note(0, 60, 0.0, 0.5, "left")];
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 2.0, 1.0),
                "normal"
            );
        }

        #[test]
        fn test_boundary_exactly_3_5_is_not_fast() {
            let notes: Vec<Note> = (0..7).map(|i| note(i, 60, 0.0, 0.5, "left")).collect();
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 0.0, 2.0),
                "normal"
            );
        }

        #[test]
        fn test_boundary_exactly_1_0_is_not_slow() {
            let notes: Vec<Note> = (0..4).map(|i| note(i, 60, 0.0, 0.5, "left")).collect();
            assert_eq!(
                SectionAnalyzer::classify_pace_beats(&notes, 0.0, 4.0),
                "normal"
            );
        }
    }

    mod test_classify_bass_articulation {
        use super::*;

        #[test]
        fn test_no_left_hand_notes_returns_legato() {
            let tm = default_tm();
            let notes: Vec<Note> = (0..3).map(|i| note(i, 60, 0.0, 0.5, "right")).collect();
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm).classify_bass_articulation(&notes),
                "legato"
            );
        }

        #[test]
        fn test_single_left_hand_note_returns_legato() {
            let tm = default_tm();
            let notes = vec![note(0, 60, 0.0, 0.5, "left")];
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm).classify_bass_articulation(&notes),
                "legato"
            );
        }

        #[test]
        fn test_high_overlap_returns_legato() {
            let tm = default_tm();
            let n1 = note(0, 60, 0.0, 0.5, "left");
            let n2 = note(1, 60, 0.5, 0.5, "left");
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm)
                    .classify_bass_articulation(&[n1.clone(), n2.clone()]),
                "legato"
            );
        }

        #[test]
        fn test_low_overlap_returns_staccato() {
            let tm = default_tm();
            let n1 = note(0, 60, 0.0, 0.1, "left");
            let n2 = note(1, 60, 1.0, 0.1, "left");
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm)
                    .classify_bass_articulation(&[n1.clone(), n2.clone()]),
                "staccato"
            );
        }

        #[test]
        fn test_mid_overlap_returns_hybrid() {
            let tm = default_tm();
            let n1 = note(0, 60, 0.0, 0.75, "left");
            let n2 = note(1, 60, 1.0, 0.75, "left");
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm)
                    .classify_bass_articulation(&[n1.clone(), n2.clone()]),
                "hybrid"
            );
        }

        #[test]
        fn test_non_positive_ioi_pair_is_skipped() {
            let tm = default_tm();
            let n1 = note(0, 60, 0.0, 0.5, "left");
            let n2 = note(1, 60, 0.0, 0.5, "left");
            let n3 = note(2, 60, 0.5, 0.5, "left");
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm).classify_bass_articulation(&[n1, n2, n3]),
                "legato"
            );
        }

        #[test]
        fn test_right_hand_notes_ignored_in_mixed_list() {
            let tm = default_tm();
            let n1 = note(0, 60, 0.0, 0.1, "left");
            let n2 = note(1, 72, 0.2, 0.5, "right");
            let n3 = note(2, 60, 1.0, 0.1, "left");
            assert_eq!(
                SectionAnalyzer::new(vec![], &tm).classify_bass_articulation(&[n1, n2, n3]),
                "staccato"
            );
        }
    }

    mod test_detect_grand_pauses {
        use super::*;

        #[test]
        fn test_no_gap_returns_two_boundaries() {
            let tm = default_tm();
            let notes: Vec<Note> = (0..4)
                .map(|i| note(i, 60, i as f64 * 0.4, 0.4, "left"))
                .collect();
            let len = notes.len();
            let sa = SectionAnalyzer::new(notes, &tm);
            assert_eq!(sa.detect_grand_pauses(), vec![0, len]);
        }

        #[test]
        fn test_large_gap_inserts_boundary() {
            let tm = default_tm();
            let notes = vec![
                note(0, 60, 0.0, 0.4, "left"),
                note(1, 60, 3.5, 0.4, "left"),
            ];
            let sa = SectionAnalyzer::new(notes, &tm);
            let boundaries = sa.detect_grand_pauses();
            assert_eq!(boundaries.len(), 3);
            assert_eq!(boundaries[1], 1);
        }

        #[test]
        fn test_empty_notes_returns_single_boundary() {
            let tm = default_tm();
            let sa = SectionAnalyzer::new(vec![], &tm);
            assert_eq!(sa.detect_grand_pauses(), vec![0]);
        }

        #[test]
        fn test_boundary_at_exactly_two_beats_not_a_gap() {
            let tm = default_tm();
            let notes = vec![
                note(0, 60, 0.0, 0.0, "left"),
                note(1, 60, 1.0, 0.4, "left"),
            ];
            let sa = SectionAnalyzer::new(notes, &tm);
            assert_eq!(sa.detect_grand_pauses(), vec![0, 2]);
        }
    }

    mod test_analyze_by_measures {
        use super::*;

        fn three_four_tm() -> TempoMap {
            TempoMap::new(vec![], vec![(0.0, 3, 4)])
        }

        #[test]
        fn test_single_measure_one_section() {
            let tm = three_four_tm();
            let notes = vec![
                note(0, 60, 0.1, 0.1, "left"),
                note(1, 64, 0.5, 0.1, "left"),
            ];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(sections.len(), 1);
        }

        #[test]
        fn test_same_style_measures_merge_into_one_section() {
            let tm = three_four_tm();
            let boundaries = tm.get_measure_boundaries(4.0);
            assert!(boundaries.len() >= 2, "test setup needs >=2 measures");
            let m0_mid = (boundaries[0].0 + boundaries[0].1) / 2.0;
            let m1_mid = (boundaries[1].0 + boundaries[1].1) / 2.0;
            let notes = vec![
                note(0, 60, m0_mid, 0.05, "left"),
                note(1, 60, m1_mid, 0.05, "left"),
            ];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(
                sections.len(),
                1,
                "single sparse notes in different measures both fall back to the same \
                 legato/normal default and should merge, not split"
            );
        }

        #[test]
        fn test_style_change_splits_section() {
            let tm = three_four_tm();
            let boundaries = tm.get_measure_boundaries(6.0);
            assert!(boundaries.len() >= 2, "test setup needs >=2 measures");
            let (m0_start, m0_end) = boundaries[0];
            let (m1_start, m1_end) = boundaries[1];

            let staccato_pair = |base: f64| {
                vec![
                    note(0, 60, base + 0.01, 0.02, "left"),
                    note(1, 60, base + (m0_end - m0_start) * 0.4, 0.02, "left"),
                ]
            };
            let legato_pair = |base: f64, span: f64| {
                vec![
                    note(2, 60, base + 0.01, span * 0.45, "left"),
                    note(3, 60, base + span * 0.45, span * 0.45, "left"),
                ]
            };

            let mut notes = staccato_pair(m0_start);
            notes.extend(legato_pair(m1_start, m1_end - m1_start));
            let sa = SectionAnalyzer::new(notes, &tm);
            let sections = sa.analyze();
            assert_eq!(sections.len(), 2, "differing styles must split into two sections");
            assert_eq!(sections[0].articulation_label, "staccato");
            assert_eq!(sections[1].articulation_label, "legato");
        }

        #[test]
        fn test_empty_measure_inherits_previous_style() {
            let tm = TempoMap::new(vec![], vec![(0.0, 3, 4)]);
            let boundaries = tm.get_measure_boundaries(6.0);
            assert!(boundaries.len() >= 3, "test setup needs >=3 measures");
            let (m0_start, m0_end) = boundaries[0];
            let notes = vec![
                note(0, 60, m0_start + 0.01, (m0_end - m0_start) * 0.45, "left"),
                note(
                    1,
                    60,
                    m0_start + (m0_end - m0_start) * 0.45,
                    (m0_end - m0_start) * 0.45,
                    "left",
                ),
                note(2, 60, boundaries[2].0 + 0.01, 0.05, "left"),
            ];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(
                sections.len(),
                1,
                "the silent middle measure should inherit measure 0's style, not split"
            );
        }

        #[test]
        fn test_all_measures_empty_but_one_note_present() {
            let tm = three_four_tm();
            let notes = vec![note(0, 60, 0.01, 0.02, "left")];
            let sections = SectionAnalyzer::new(notes, &tm).analyze();
            assert_eq!(sections.len(), 1);
            assert_eq!(sections[0].articulation_label, "legato");
            assert_eq!(
                sections[0].pace_label, "slow",
                "one note over a 3-beat measure is 1/3 notes-per-beat, below the 1.0 slow threshold"
            );
        }
    }
}
