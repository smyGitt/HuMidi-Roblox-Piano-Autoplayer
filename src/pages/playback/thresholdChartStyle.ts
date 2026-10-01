export interface ThresholdChartStyle {
  curveColor: string;
  onColor: string;
  offColor: string;
  shadeColor: string;
  shadeOpacity: number;
  lineWidth: number;
}

export function readThresholdChartStyle(element: Element): ThresholdChartStyle {
  const computed = getComputedStyle(element);
  const text = (name: string) => computed.getPropertyValue(name).trim();
  return {
    curveColor: text("--accent"),
    onColor: text("--accent_play"),
    offColor: text("--accent_stop"),
    shadeColor: text("--pedal_color"),
    shadeOpacity: parseFloat(text("--pedal-chart-shade-opacity")),
    lineWidth: parseFloat(text("--border-width")),
  };
}
