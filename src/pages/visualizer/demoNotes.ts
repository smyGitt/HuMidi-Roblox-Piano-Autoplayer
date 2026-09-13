export interface VizNote {
  pitch: number;
  start: number;
  duration: number;
  hand: "left" | "right" | "unknown";
}

export function buildDemoNotes(): { notes: VizNote[]; totalDuration: number; pedalIntervals: [number, number][] } {
  const notes: VizNote[] = [];
  const leftPattern = [48, 52, 55, 48, 53, 57, 48, 55, 59];
  const rightPattern = [72, 74, 76, 77, 76, 74, 72, 71, 69, 71, 72, 74];

  let t = 0;
  for (let bar = 0; bar < 6; bar++) {
    for (const pitch of leftPattern) {
      notes.push({ pitch, start: t, duration: 0.45, hand: "left" });
      t += 0.5;
    }
  }

  t = 0;
  for (let bar = 0; bar < 4; bar++) {
    for (const pitch of rightPattern) {
      notes.push({ pitch, start: t, duration: 0.3, hand: "right" });
      t += 0.375;
    }
  }

  const totalDuration = Math.max(...notes.map((n) => n.start + n.duration)) + 1;
  const pedalIntervals: [number, number][] = [];
  for (let p = 0; p < totalDuration; p += 3) {
    pedalIntervals.push([p, Math.min(p + 1.8, totalDuration)]);
  }

  return { notes, totalDuration, pedalIntervals };
}
