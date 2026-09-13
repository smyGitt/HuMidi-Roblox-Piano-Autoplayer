import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PedalAiCard, type PedalAiStats } from "./PedalAiCard";

function baseProps(overrides: Partial<Parameters<typeof PedalAiCard>[0]> = {}) {
  return {
    hasThresholds: true,
    generateEnabled: true,
    thresholdOn: 0.6,
    thresholdOff: 0.4,
    stats: null as PedalAiStats | null,
    onGenerate: vi.fn(),
    onThresholdChange: vi.fn(),
    onReset: vi.fn(),
    ...overrides,
  };
}

describe("PedalAiCard", () => {
  it("shows the Generate button and hides thresholds/reset before any generation", () => {
    render(<PedalAiCard {...baseProps({ hasThresholds: false })} />);
    expect(screen.getByText("Generate AI Pedal Events")).toBeInTheDocument();
    expect(screen.queryByTitle("Reset")).not.toBeInTheDocument();
  });

  it("shows no diagnostics when stats are within normal range", () => {
    const stats: PedalAiStats = { avgDur: 0.5, minDur: 0.3, maxDur: 1.0, pressesPerMin: 20 };
    render(<PedalAiCard {...baseProps({ stats })} />);
    expect(screen.queryByText(/chattering/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/holds too long/i)).not.toBeInTheDocument();
  });

  it("detects chattering (high presses/min) and offers Raise On / Lower Off / Regenerate rows", () => {
    const onThresholdChange = vi.fn();
    const onGenerate = vi.fn();
    const stats: PedalAiStats = { avgDur: 0.5, minDur: 0.3, maxDur: 1.0, pressesPerMin: 50 };
    render(
      <PedalAiCard
        {...baseProps({ stats, thresholdOn: 0.6, thresholdOff: 0.4, onThresholdChange, onGenerate })}
      />,
    );
    expect(screen.getByText(/Pedal chattering detected/)).toBeInTheDocument();

    fireEvent.click(screen.getByText(/Raise On Threshold to 0.680/));
    expect(onThresholdChange).toHaveBeenCalledWith(0.68, 0.4);

    fireEvent.click(screen.getByText(/Lower Off Threshold to 0.350/));
    expect(onThresholdChange).toHaveBeenCalledWith(0.6, 0.35);

    fireEvent.click(screen.getByText(/Regenerate with fresh auto-thresholds/));
    expect(onGenerate).toHaveBeenCalledTimes(1);
  });

  it("detects chattering via short average hold even at a low press rate", () => {
    const stats: PedalAiStats = { avgDur: 0.1, minDur: 0.05, maxDur: 0.2, pressesPerMin: 10 };
    render(<PedalAiCard {...baseProps({ stats })} />);
    expect(screen.getByText(/Pedal chattering detected/)).toBeInTheDocument();
  });

  it("clamps the chattering suggestions at 0.99 / 0.01", () => {
    const onThresholdChange = vi.fn();
    const stats: PedalAiStats = { avgDur: 0.1, minDur: 0.05, maxDur: 0.2, pressesPerMin: 50 };
    render(
      <PedalAiCard
        {...baseProps({ stats, thresholdOn: 0.95, thresholdOff: 0.03, onThresholdChange })}
      />,
    );
    fireEvent.click(screen.getByText(/Raise On Threshold to 0.990/));
    expect(onThresholdChange).toHaveBeenCalledWith(0.99, 0.03);
    fireEvent.click(screen.getByText(/Lower Off Threshold to 0.010/));
    expect(onThresholdChange).toHaveBeenCalledWith(0.95, 0.01);
  });

  it("detects sparse/long holds and offers a single Raise Off / Regenerate pair, not the chattering rows", () => {
    const onThresholdChange = vi.fn();
    const onGenerate = vi.fn();
    const stats: PedalAiStats = { avgDur: 2, minDur: 1, maxDur: 20, pressesPerMin: 10 };
    render(
      <PedalAiCard
        {...baseProps({ stats, thresholdOn: 0.6, thresholdOff: 0.4, onThresholdChange, onGenerate })}
      />,
    );
    expect(screen.getByText(/Pedal holds too long or too sparse/)).toBeInTheDocument();
    expect(screen.queryByText(/Raise On Threshold/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByText(/Raise Off Threshold to 0.450/));
    expect(onThresholdChange).toHaveBeenCalledWith(0.6, 0.45);

    fireEvent.click(screen.getByText(/Regenerate with fresh auto-thresholds/));
    expect(onGenerate).toHaveBeenCalledTimes(1);
  });

  it("detects sparse via a very low press rate even with an otherwise unremarkable max duration", () => {
    const stats: PedalAiStats = { avgDur: 1, minDur: 0.5, maxDur: 2, pressesPerMin: 1 };
    render(<PedalAiCard {...baseProps({ stats })} />);
    expect(screen.getByText(/Pedal holds too long or too sparse/)).toBeInTheDocument();
  });

  it("Reset button calls onReset when thresholds exist", () => {
    const onReset = vi.fn();
    render(<PedalAiCard {...baseProps({ onReset })} />);
    fireEvent.click(screen.getByTitle("Reset"));
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it("editing the On/Off inputs preserves the other threshold's current value", () => {
    const onThresholdChange = vi.fn();
    render(<PedalAiCard {...baseProps({ thresholdOn: 0.6, thresholdOff: 0.4, onThresholdChange })} />);
    fireEvent.change(screen.getByLabelText(/On threshold/), { target: { value: "0.7" } });
    expect(onThresholdChange).toHaveBeenCalledWith(0.7, 0.4);
    fireEvent.change(screen.getByLabelText(/Off threshold/), { target: { value: "0.3" } });
    expect(onThresholdChange).toHaveBeenCalledWith(0.6, 0.3);
  });
});
