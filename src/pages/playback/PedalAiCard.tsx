import { Card } from "../../components/Card";
import { Icon } from "../../components/Icon";

export interface PedalAiStats {
  avgDur: number;
  minDur: number;
  maxDur: number;
  pressesPerMin: number;
}

interface PedalAiCardProps {
  hasThresholds: boolean;
  generateEnabled: boolean;
  thresholdOn: number;
  thresholdOff: number;
  stats: PedalAiStats | null;
  onGenerate: () => void;
  onThresholdChange: (on: number, off: number) => void;
  onReset: () => void;
}

interface DiagRow {
  label: string;
  onClick: () => void;
}

interface Diagnosis {
  title: string;
  rows: DiagRow[];
}

function round3(n: number): number {
  return Math.round(n * 1000) / 1000;
}

function diagnose(
  stats: PedalAiStats,
  thresholdOn: number,
  thresholdOff: number,
  onThresholdChange: (on: number, off: number) => void,
  onGenerate: () => void,
): Diagnosis | null {
  const isChattering = stats.pressesPerMin > 45 || stats.avgDur < 0.25 || stats.minDur < 0.08;
  const isSparse =
    !isChattering &&
    (stats.pressesPerMin < 3 || stats.maxDur > 12 || (stats.avgDur > 0 && stats.maxDur / stats.avgDur > 8));

  if (!isChattering && !isSparse) return null;

  const rows: DiagRow[] = [];
  if (isChattering) {
    const suggestedOn = Math.min(0.99, round3(thresholdOn + 0.08));
    const suggestedOff = Math.max(0.01, round3(thresholdOff - 0.05));
    rows.push({
      label: `Raise On Threshold to ${suggestedOn.toFixed(3)} (fewer false triggers)`,
      onClick: () => onThresholdChange(suggestedOn, thresholdOff),
    });
    rows.push({
      label: `Lower Off Threshold to ${suggestedOff.toFixed(3)} (widen hysteresis gap)`,
      onClick: () => onThresholdChange(thresholdOn, suggestedOff),
    });
  } else {
    const suggestedOff = Math.max(0.01, Math.min(round3(thresholdOn - 0.02), round3(thresholdOff + 0.05)));
    rows.push({
      label: `Raise Off Threshold to ${suggestedOff.toFixed(3)} (releases more easily)`,
      onClick: () => onThresholdChange(thresholdOn, suggestedOff),
    });
  }
  rows.push({ label: "Regenerate with fresh auto-thresholds", onClick: onGenerate });

  return {
    title: isChattering ? "⚠ Pedal chattering detected" : "⚠ Pedal holds too long or too sparse",
    rows,
  };
}

export function PedalAiCard({
  hasThresholds,
  generateEnabled,
  thresholdOn,
  thresholdOff,
  stats,
  onGenerate,
  onThresholdChange,
  onReset,
}: PedalAiCardProps) {
  const diagnosis = stats ? diagnose(stats, thresholdOn, thresholdOff, onThresholdChange, onGenerate) : null;

  return (
    <Card
      title="PEDAL AI THRESHOLDS"
      titleButtons={
        hasThresholds && (
          <button className="icon-btn icon-btn--danger" onClick={onReset} title="Reset">
            <Icon name="arrow-counter-clockwise" size={16} />
          </button>
        )
      }
      className="pedal-ai-card"
    >
      {!hasThresholds ? (
        <div className="pedal-ai-card__pre">
          <span className="pedal-ai-card__hint">
            Generate AI-driven sustain pedal events from the loaded MIDI's dynamics.
          </span>
          <button className="pedal-ai-card__generate-btn" disabled={!generateEnabled} onClick={onGenerate}>
            Generate AI Pedal Events
          </button>
        </div>
      ) : (
        <div className="pedal-ai-card__post">
          <div className="pedal-ai-card__thresholds">
            <label>
              On threshold
              <input
                type="number"
                min={0}
                max={1}
                step={0.001}
                value={thresholdOn}
                onChange={(e) => onThresholdChange(Number(e.target.value), thresholdOff)}
              />
            </label>
            <label>
              Off threshold
              <input
                type="number"
                min={0}
                max={1}
                step={0.001}
                value={thresholdOff}
                onChange={(e) => onThresholdChange(thresholdOn, Number(e.target.value))}
              />
            </label>
          </div>
          {stats && (
            <div className="pedal-ai-card__stats">
              <span>Avg hold: {stats.avgDur.toFixed(2)}s</span>
              <span>Min hold: {stats.minDur.toFixed(2)}s</span>
              <span>Max hold: {stats.maxDur.toFixed(2)}s</span>
              <span>Presses/min: {stats.pressesPerMin.toFixed(1)}</span>
            </div>
          )}
          {diagnosis && (
            <div className="pedal-ai-card__diag">
              <div className="pedal-ai-card__diag-title">{diagnosis.title}</div>
              {diagnosis.rows.map((row) => (
                <button key={row.label} className="pedal-ai-card__diag-row" onClick={row.onClick}>
                  {"→ "}
                  {row.label}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </Card>
  );
}
