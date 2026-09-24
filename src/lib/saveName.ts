import { stripMidiExtension } from "./midiName";

const TWO_DIGITS = 2;

function pad(value: number): string {
  return String(value).padStart(TWO_DIGITS, "0");
}

export function saveTimestamp(now: Date): string {
  const date = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`;
  const time = `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `${date}_${time}`;
}

export function defaultSaveName(fileName: string, now: Date = new Date()): string {
  return `${stripMidiExtension(fileName)}_${saveTimestamp(now)}`;
}
