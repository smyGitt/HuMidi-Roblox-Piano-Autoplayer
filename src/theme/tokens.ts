export interface ThemeTokens {
  bg_primary: string;
  bg_surface: string;
  bg_input: string;
  text_primary: string;
  text_muted: string;
  border: string;
  accent: string;
  accent_play: string;
  accent_stop: string;
  pedal_color: string;
  accent_loaded: string;
  knob_color: string;
  viz_note_left: string;
  viz_note_right: string;
  viz_note_other: string;
  viz_progress_line: string;
  viz_measure_line: string;
  viz_key_white: string;
  viz_key_black: string;
  viz_key_pressed: string;
}

export const MIN_IMAGE_ZOOM = 0.1;
export const IMAGE_ZOOM_STEP = 0.1;
export const DEFAULT_IMAGE_ZOOM = 1;
export const DEFAULT_IMAGE_OFFSET = 0;
const IMAGE_ZOOM_DECIMALS = 1;

export function clampImageZoom(value: number): number {
  const rounded = Number(value.toFixed(IMAGE_ZOOM_DECIMALS));
  return Math.max(MIN_IMAGE_ZOOM, rounded);
}

export function imageFields(theme: Theme) {
  return {
    image: theme.image ?? "",
    image_scale: theme.imageZoom ?? DEFAULT_IMAGE_ZOOM,
    image_x: theme.imageX ?? DEFAULT_IMAGE_OFFSET,
    image_y: theme.imageY ?? DEFAULT_IMAGE_OFFSET,
  };
}

export interface Theme {
  name: string;
  tokens: ThemeTokens;
  image?: string;
  imageZoom?: number;
  imageX?: number;
  imageY?: number;
}

export const THEMES: Theme[] = [
  {
    name: "Midnight",
    tokens: {
      bg_primary: "#0d1117",
      bg_surface: "#161b22",
      bg_input: "#1c2230",
      text_primary: "#e6edf3",
      text_muted: "#8b949e",
      border: "#30363d",
      accent: "#58a6ff",
      accent_play: "#3fb950",
      accent_stop: "#f85149",
      pedal_color: "#f0a030",
      accent_loaded: "#d4a020",
      knob_color: "#dadbdc",
      viz_note_left: "#58a6ff",
      viz_note_right: "#3fb950",
      viz_note_other: "#8b949e",
      viz_progress_line: "#f85149",
      viz_measure_line: "#30363d",
      viz_key_white: "#dadbdc",
      viz_key_black: "#161b22",
      viz_key_pressed: "#3fb950",
    },
  },
  {
    name: "Light",
    tokens: {
      bg_primary: "#f0f0f8",
      bg_surface: "#ffffff",
      bg_input: "#fafafa",
      text_primary: "#1a1a2e",
      text_muted: "#6868a0",
      border: "#d0d0e8",
      accent: "#4a7adb",
      accent_play: "#2a9a60",
      accent_stop: "#cc3333",
      pedal_color: "#d08010",
      accent_loaded: "#b87010",
      knob_color: "#fcfcfd",
      viz_note_left: "#4a7adb",
      viz_note_right: "#2a9a60",
      viz_note_other: "#6868a0",
      viz_progress_line: "#cc3333",
      viz_measure_line: "#d0d0e8",
      viz_key_white: "#fcfcfd",
      viz_key_black: "#1a1a2e",
      viz_key_pressed: "#2a9a60",
    },
  },
  {
    name: "Hatsune Miku",
    tokens: {
      bg_primary: "#000000",
      bg_surface: "#1f1d1e",
      bg_input: "#121112",
      text_primary: "#ffffff",
      text_muted: "#92898b",
      border: "#3a3637",
      accent: "#00d0ea",
      accent_play: "#00d0ea",
      accent_stop: "#ff0003",
      pedal_color: "#009db2",
      accent_loaded: "#ffd800",
      knob_color: "#ffffff",
      viz_note_left: "#00d0ea",
      viz_note_right: "#009db2",
      viz_note_other: "#92898b",
      viz_progress_line: "#ff0003",
      viz_measure_line: "#3a3637",
      viz_key_white: "#ffffff",
      viz_key_black: "#000000",
      viz_key_pressed: "#00d0ea",
    },
  },
  {
    name: "Kasane Teto",
    tokens: {
      bg_primary: "#111111",
      bg_surface: "#2a2a2a",
      bg_input: "#1a1a1a",
      text_primary: "#f0f0f0",
      text_muted: "#a08a8e",
      border: "#4a3a3d",
      accent: "#d4203c",
      accent_play: "#ff5c77",
      accent_stop: "#ff9f1c",
      pedal_color: "#ff8fa3",
      accent_loaded: "#f4c20d",
      knob_color: "#f0f0f0",
      viz_note_left: "#d4203c",
      viz_note_right: "#ff8fa3",
      viz_note_other: "#a08a8e",
      viz_progress_line: "#ffd166",
      viz_measure_line: "#4a3a3d",
      viz_key_white: "#f0f0f0",
      viz_key_black: "#2a2a2a",
      viz_key_pressed: "#ff5c77",
    },
  },
];

export const DEFAULT_THEME_NAME = "Midnight";

export const COLOR_FIELD_GROUPS: { label: string; fields: { key: keyof ThemeTokens; label: string }[] }[] = [
  {
    label: "Surfaces",
    fields: [
      { key: "bg_primary", label: "Background" },
      { key: "bg_surface", label: "Surface" },
      { key: "bg_input", label: "Input Fields" },
    ],
  },
  {
    label: "Text & Borders",
    fields: [
      { key: "text_primary", label: "Text" },
      { key: "text_muted", label: "Muted Text" },
      { key: "border", label: "Borders" },
    ],
  },
  {
    label: "Accents",
    fields: [
      { key: "accent", label: "Accent" },
      { key: "knob_color", label: "Knob" },
    ],
  },
  {
    label: "Status",
    fields: [
      { key: "accent_play", label: "Play Color" },
      { key: "accent_stop", label: "Stop / Danger" },
      { key: "accent_loaded", label: "File Loaded" },
      { key: "pedal_color", label: "Pedal Color" },
    ],
  },
  {
    label: "Visualizer",
    fields: [
      { key: "viz_note_left", label: "Left Hand Notes" },
      { key: "viz_note_right", label: "Right Hand Notes" },
      { key: "viz_note_other", label: "Unassigned Notes" },
      { key: "viz_progress_line", label: "Progress Line" },
      { key: "viz_measure_line", label: "Measure Lines" },
      { key: "viz_key_white", label: "White Keys" },
      { key: "viz_key_black", label: "Black Keys" },
      { key: "viz_key_pressed", label: "Pressed Keys" },
    ],
  },
];

export function completeTokens(tokens: Partial<ThemeTokens>): ThemeTokens {
  const fallback = THEMES.find((t) => t.name === DEFAULT_THEME_NAME)!.tokens;
  const complete: ThemeTokens = { ...fallback };
  for (const key of Object.keys(fallback) as (keyof ThemeTokens)[]) {
    const value = tokens[key];
    if (typeof value === "string" && value !== "") complete[key] = value;
  }
  return complete;
}

export function uniqueCopyName(baseName: string, existing: Set<string>): string {
  let candidate = `${baseName} Copy`;
  let n = 2;
  while (existing.has(candidate)) {
    candidate = `${baseName} Copy ${n}`;
    n += 1;
  }
  return candidate;
}

export function applyThemeToRoot(theme: Theme, root: HTMLElement = document.documentElement) {
  for (const [key, value] of Object.entries(theme.tokens)) {
    root.style.setProperty(`--${key}`, value);
  }
  root.style.setProperty("--btn_hover", `color-mix(in srgb, var(--accent) 16%, var(--bg_surface))`);
  root.style.setProperty("--accent_tint", `color-mix(in srgb, var(--accent) 12%, transparent)`);
  root.style.setProperty("--save_card_hover", `color-mix(in srgb, var(--accent) 10%, var(--bg_surface))`);
}
