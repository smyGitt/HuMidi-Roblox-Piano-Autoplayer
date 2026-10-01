export function ThresholdLegend() {
  return (
    <div className="pedal-ai-card__legend">
      <span className="pedal-ai-card__legend-item">
        <span className="pedal-ai-card__legend-swatch pedal-ai-card__legend-swatch--on" />
        On threshold
      </span>
      <span className="pedal-ai-card__legend-item">
        <span className="pedal-ai-card__legend-swatch pedal-ai-card__legend-swatch--off" />
        Off threshold
      </span>
    </div>
  );
}
