export interface WindowSize {
  width: number;
  height: number;
}

export const FULL_SIZE_KEY = "window_size_full";
export const MINI_SIZE_KEY = "window_size_mini";

function isPositiveNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value > 0;
}

export function readSavedSize(config: Record<string, unknown>, key: string): WindowSize | null {
  const value = config[key];
  if (typeof value !== "object" || value === null) return null;
  const { width, height } = value as Record<string, unknown>;
  return isPositiveNumber(width) && isPositiveNumber(height) ? { width, height } : null;
}

export function sameSize(a: WindowSize | null, b: WindowSize | null): boolean {
  if (!a || !b) return false;
  return Math.round(a.width) === Math.round(b.width) && Math.round(a.height) === Math.round(b.height);
}
