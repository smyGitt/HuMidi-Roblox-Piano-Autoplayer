import { useRef } from "react";
import type { VizNote } from "./demoNotes";
import type { MeasureBoundary } from "../../lib/tauri";

const PX_PER_SECOND = 60;
const MIN_WIDTH = 800;
const MAX_WIDTH = 6000;
const HEIGHT = 260;
const TOP_MARGIN = 6;
const PEDAL_GAP = 3;

interface PianoRollProps {
  notes: VizNote[];
  totalDuration: number;
  currentTime: number;
  measureBoundaries: MeasureBoundary[];
  pedalIntervals: [number, number][];
  minPitch: number;
  maxPitch: number;
  showPedal: boolean;
  onScrub: (time: number) => void;
  onSeek: (time: number) => void;
}

const HAND_CLASS: Record<VizNote["hand"], string> = {
  left: "piano-roll__note--left",
  right: "piano-roll__note--right",
  unknown: "piano-roll__note--unknown",
};

export function PianoRoll({
  notes,
  totalDuration,
  currentTime,
  measureBoundaries,
  pedalIntervals,
  minPitch,
  maxPitch,
  showPedal,
  onScrub,
  onSeek,
}: PianoRollProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const isDraggingRef = useRef(false);

  const width = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, totalDuration * PX_PER_SECOND));
  const pitchRange = maxPitch - minPitch + 1;
  const rowH = showPedal ? (HEIGHT - TOP_MARGIN - PEDAL_GAP) / (pitchRange + 1) : (HEIGHT - TOP_MARGIN) / pitchRange;

  function timeFromClientX(clientX: number): number {
    const svg = svgRef.current;
    if (!svg) return 0;
    const rect = svg.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    return ratio * totalDuration;
  }

  function handlePointerDown(e: React.PointerEvent<SVGSVGElement>) {
    if (e.button !== 0) return;
    isDraggingRef.current = true;
    svgRef.current?.setPointerCapture(e.pointerId);
    onScrub(timeFromClientX(e.clientX));
  }

  function handlePointerMove(e: React.PointerEvent<SVGSVGElement>) {
    if (!isDraggingRef.current) return;
    onScrub(timeFromClientX(e.clientX));
  }

  function handlePointerUp(e: React.PointerEvent<SVGSVGElement>) {
    if (!isDraggingRef.current) return;
    isDraggingRef.current = false;
    svgRef.current?.releasePointerCapture(e.pointerId);
    onSeek(timeFromClientX(e.clientX));
  }

  return (
    <div className="piano-roll">
      <svg
        ref={svgRef}
        className="piano-roll__svg"
        width={width}
        height="100%"
        viewBox={`0 0 ${width} ${HEIGHT}`}
        preserveAspectRatio="none"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <rect className="piano-roll__bg" x={0} y={0} width={width} height={HEIGHT} />

        {measureBoundaries.map(([start]) => (
          <line
            key={start}
            className="piano-roll__measure-line"
            x1={(start / totalDuration) * width}
            x2={(start / totalDuration) * width}
            y1={0}
            y2={HEIGHT}
          />
        ))}

        {notes.map((n, i) => {
          const rowIndex = maxPitch - n.pitch;
          const y = TOP_MARGIN + rowIndex * rowH;
          const x = (n.start / totalDuration) * width;
          const w = Math.max(2, (n.duration / totalDuration) * width - 1);
          return (
            <rect
              key={i}
              className={`piano-roll__note ${HAND_CLASS[n.hand]}`}
              x={x}
              y={y}
              width={w}
              height={Math.max(2, rowH - 1)}
              rx={1.5}
            />
          );
        })}

        {showPedal &&
          pedalIntervals.map(([start, end], i) => (
            <rect
              key={i}
              className="piano-roll__pedal"
              x={(start / totalDuration) * width}
              y={HEIGHT - rowH}
              width={Math.max(2, ((end - start) / totalDuration) * width)}
              height={rowH}
              rx={1.5}
            />
          ))}

        <line
          className="piano-roll__cursor"
          x1={(currentTime / totalDuration) * width}
          x2={(currentTime / totalDuration) * width}
          y1={0}
          y2={HEIGHT}
        />
      </svg>
    </div>
  );
}
