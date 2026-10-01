import type { VizHand } from "../../lib/noteTimeline";

export interface VizStyle {
  noteColors: Record<VizHand, string>;
  progressLineColor: string;
  measureLineColor: string;
  pedalColor: string;
  lineWidth: number;
  noteGap: number;
  secondLength: number;
  playheadFraction: number;
}

export function readVizStyle(element: Element): VizStyle {
  const computed = getComputedStyle(element);
  const text = (name: string) => computed.getPropertyValue(name).trim();
  const number = (name: string) => parseFloat(text(name));
  return {
    noteColors: {
      left: text("--viz_note_left"),
      right: text("--viz_note_right"),
      other: text("--viz_note_other"),
    },
    progressLineColor: text("--viz_progress_line"),
    measureLineColor: text("--viz_measure_line"),
    pedalColor: text("--pedal_color"),
    lineWidth: number("--border-width"),
    noteGap: number("--viz-note-gap"),
    secondLength: number("--viz-second-length"),
    playheadFraction: number("--viz-playhead-fraction"),
  };
}
