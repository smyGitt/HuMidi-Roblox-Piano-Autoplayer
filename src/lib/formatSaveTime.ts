const SAVE_TIME_LOCALE = "en-US";

const SAVE_TIME_FORMAT: Intl.DateTimeFormatOptions = {
  year: "numeric",
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
  hour12: true,
};

export function formatSaveTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString(SAVE_TIME_LOCALE, SAVE_TIME_FORMAT);
}
