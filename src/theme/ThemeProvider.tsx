import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { isTauri, getActiveThemeName, setActiveThemeName, getCustomThemes } from "../lib/tauri";
import { THEMES, DEFAULT_THEME_NAME, applyThemeToRoot, type Theme } from "./tokens";

interface ThemeContextValue {
  theme: Theme;
  themeName: string;
  setThemeName: (name: string) => void;
  themeNames: string[];
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [themeName, setThemeNameState] = useState(DEFAULT_THEME_NAME);
  const theme = useMemo(
    () => THEMES.find((t) => t.name === themeName) ?? THEMES[0],
    [themeName],
  );

  useEffect(() => {
    applyThemeToRoot(theme);
  }, [theme]);

  useEffect(() => {
    if (!isTauri()) return;
    Promise.all([getActiveThemeName(), getCustomThemes()])
      .then(([stored, customThemes]) => {
        if (!stored) return;
        const isValid = THEMES.some((t) => t.name === stored) || customThemes.some((t) => t.name === stored);
        if (isValid) setThemeNameState(stored);
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function setThemeName(name: string) {
    setThemeNameState(name);
    if (isTauri()) void setActiveThemeName(name);
  }

  const value = useMemo(
    () => ({ theme, themeName, setThemeName, themeNames: THEMES.map((t) => t.name) }),
    [theme, themeName],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
