export type Parser<T> = (raw: unknown) => T | undefined;

export const parseBoolean: Parser<boolean> = (raw) => (typeof raw === "boolean" ? raw : undefined);

export const parseString: Parser<string> = (raw) => (typeof raw === "string" ? raw : undefined);

export function parseNumberInRange(min: number, max: number): Parser<number> {
  return (raw) => (typeof raw === "number" && Number.isFinite(raw) && raw >= min && raw <= max ? raw : undefined);
}

export function parseOneOf<T extends string | number>(options: readonly T[]): Parser<T> {
  return (raw) => options.find((option) => option === raw);
}
