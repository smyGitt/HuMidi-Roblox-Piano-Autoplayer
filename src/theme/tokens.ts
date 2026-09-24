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
}

export interface Theme {
  name: string;
  tokens: ThemeTokens;
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
    },
  },
  {
    name: "Hatsune Miku",
    tokens: {
      bg_primary: "#111111",
      bg_surface: "#2c2c2c",
      bg_input: "#1a1611",
      text_primary: "#00bbcc",
      text_muted: "#2a7fa3",
      border: "#2e5963",
      accent: "#00ffff",
      accent_play: "#4affff",
      accent_stop: "#ff0000",
      pedal_color: "#51a4cb",
      accent_loaded: "#d4bd0d",
      knob_color: "#dbdbdb",
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
];

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
