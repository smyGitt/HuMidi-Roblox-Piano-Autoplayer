import { Card } from "../../components/Card";
import { ArrowClockwiseIcon, ArrowCounterClockwiseIcon } from "@phosphor-icons/react";
import { Button } from "../../components/Button";
import type { PedalConfidence } from "../../lib/pedalPreview";
import { ThresholdChart } from "./ThresholdChart";
import { ThresholdLegend } from "./ThresholdLegend";
import { ThresholdStepper } from "./ThresholdStepper";

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
  confidence?: PedalConfidence | null;
  onGenerate: () => void;
  onThresholdChange: (on: number, off: number) => void;
  onReset: () => void;
}

const CHATTER_MAX_PRESSES_PER_MIN = 45;
const CHATTER_MIN_AVG_HOLD = 0.25;
const CHATTER_MIN_HOLD = 0.08;
const SPARSE_MIN_PRESSES_PER_MIN = 3;
const SPARSE_MAX_HOLD = 12;
const SPARSE_MAX_HOLD_RATIO = 8;

function diagnose(stats: PedalAiStats): string | null {
  const isChattering =
    stats.pressesPerMin > CHATTER_MAX_PRESSES_PER_MIN ||
    stats.avgDur < CHATTER_MIN_AVG_HOLD ||
    stats.minDur < CHATTER_MIN_HOLD;
  if (isChattering) return "⚠ Pedal chattering detected";
  const isSparse =
    stats.pressesPerMin < SPARSE_MIN_PRESSES_PER_MIN ||
    stats.maxDur > SPARSE_MAX_HOLD ||
    (stats.avgDur > 0 && stats.maxDur / stats.avgDur > SPARSE_MAX_HOLD_RATIO);
  return isSparse ? "⚠ Pedal holds too long or too sparse" : null;
}

export function PedalAiCard({
  hasThresholds,
  generateEnabled,
  thresholdOn,
  thresholdOff,
  stats,
  confidence = null,
  onGenerate,
  onThresholdChange,
  onReset,
}: PedalAiCardProps) {
  const warning = stats ? diagnose(stats) : null;

  return (
    <Card
      title="PEDAL AI THRESHOLDS"
      titleButtons={
        hasThresholds && (
          <>
            {warning && <span className="pedal-ai-card__warning">{warning}</span>}
            <Button variant="icon" subtle danger onClick={onReset} title="Reset">
              <ArrowCounterClockwiseIcon weight="duotone" />
            </Button>
          </>
        )
      }
      className="pedal-ai-card"
    >
      {!hasThresholds ? (
        <div className="pedal-ai-card__pre">
          <span className="pedal-ai-card__hint">
            A trained model reads through your notes, forwards and backwards, and works out when a real player would
            press and lift the pedal. Afterwards, the On and Off thresholds set the confidence the model needs before the pedal
            goes down and before it lifts.
          </span>
          <Button variant="accent" disabled={!generateEnabled} onClick={onGenerate}>
            Generate pedal events
          </Button>
        </div>
      ) : (
        <div className="pedal-ai-card__post">
          <div className="pedal-ai-card__controls">
            <div className="pedal-ai-card__thresholds">
              <ThresholdStepper
                label="On threshold"
                value={thresholdOn}
                onChange={(value) => onThresholdChange(value, thresholdOff)}
              />
              <ThresholdStepper
                label="Off threshold"
                value={thresholdOff}
                onChange={(value) => onThresholdChange(thresholdOn, value)}
              />
            </div>
            <div className="pedal-ai-card__side">
              {stats && (
                <div className="pedal-ai-card__stats">
                  <span>Avg hold: {stats.avgDur.toFixed(2)}s</span>
                  <span>Min hold: {stats.minDur.toFixed(2)}s</span>
                  <span>Max hold: {stats.maxDur.toFixed(2)}s</span>
                  <span>Presses/min: {stats.pressesPerMin.toFixed(1)}</span>
                </div>
              )}
              <Button className="pedal-ai-card__regen" onClick={onGenerate}>
                <ArrowClockwiseIcon weight="duotone" />
                Regenerate
              </Button>
            </div>
          </div>
          {confidence && (
            <>
              <ThresholdChart confidence={confidence} thresholdOn={thresholdOn} thresholdOff={thresholdOff} />
              <ThresholdLegend />
            </>
          )}
        </div>
      )}
    </Card>
  );
}
