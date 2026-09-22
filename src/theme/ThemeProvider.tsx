import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { isTauri, getActiveThemeName, setActiveThemeName, getCustomThemes, type CustomThemeColors } from "../lib/tauri";
import { THEMES, DEFAULT_THEME_NAME, applyThemeToRoot, type Theme, type ThemeTokens } from "./tokens";

function toTheme(custom: CustomThemeColors): Theme {
  const { name, builtin: _builtin, ...tokens } = custom;
  return { name, tokens: tokens as ThemeTokens };
}

interface ThemeContextValue {
  theme: Theme;
  themeName: string;
  setThemeName: (name: string) => void;
  themeNames: string[];
  customThemes: Theme[];
  refreshCustomThemes: () => Promise<void>;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [themeName, setThemeNameState] = useState(DEFAULT_THEME_NAME);
  const [customThemes, setCustomThemes] = useState<Theme[]>([]);

  const allThemes = useMemo(() => [...THEMES, ...customThemes], [customThemes]);
  const theme = useMemo(
    () => allThemes.find((t) => t.name === themeName) ?? THEMES[0],
    [allThemes, themeName],
  );

  useEffect(() => {
    applyThemeToRoot(theme);
  }, [theme]);

  const refreshCustomThemes = useCallback(async () => {
    if (!isTauri()) return;
    try {
      const fetched = await getCustomThemes();
      setCustomThemes(fetched.map(toTheme));
    } catch {
      // themes file missing/corrupted -- keep whatever was already loaded
    }
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    Promise.all([getActiveThemeName(), getCustomThemes()])
      .then(([stored, fetched]) => {
        const custom = fetched.map(toTheme);
        setCustomThemes(custom);
        if (!stored) return;
        const isValid = THEMES.some((t) => t.name === stored) || custom.some((t) => t.name === stored);
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
    () => ({
      theme,
      themeName,
      setThemeName,
      themeNames: allThemes.map((t) => t.name),
      customThemes,
      refreshCustomThemes,
    }),
    [theme, themeName, allThemes, customThemes, refreshCustomThemes],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
