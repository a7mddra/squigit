/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useEffect,
  useState,
  useCallback,
  createContext,
  useContext,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { loadPreferences, savePreferences } from "@/core";

export interface ThemeContextType {
  theme: "light" | "dark" | "system";
  resolvedTheme: "light" | "dark";
  setTheme: (theme: "light" | "dark" | "system") => void;
  ready: boolean;
}

export const ThemeContext = createContext<ThemeContextType>({
  theme: "system",
  resolvedTheme: "dark",
  setTheme: () => {},
  ready: false,
});

export const useTheme = () => useContext(ThemeContext);

const THEME_STORAGE_KEY = "theme";

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [ready, setReady] = useState(false);
  const [theme, setThemeState] = useState<"light" | "dark" | "system">(() => {
    if (typeof window !== "undefined") {
      const cached = localStorage.getItem(THEME_STORAGE_KEY);
      if (cached === "light" || cached === "dark" || cached === "system") {
        return cached;
      }
    }
    return "system";
  });

  const getSystemThemeFallback = useCallback((): "light" | "dark" => {
    if (
      typeof window !== "undefined" &&
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
    ) {
      return "dark";
    }
    return "light";
  }, []);

  const [resolvedTheme, setResolvedTheme] = useState<"light" | "dark">(() => {
    if (theme === "system") return getSystemThemeFallback();
    return theme;
  });

  const applyDomTheme = useCallback((targetTheme: "light" | "dark") => {
    document.documentElement.classList.toggle("dark", targetTheme === "dark");
    document.documentElement.classList.toggle("light", targetTheme === "light");
    document.documentElement.style.colorScheme = targetTheme;
    if (document.body) {
      document.body.classList.toggle("light-mode", targetTheme === "light");
    }

    // Sync native window background color with CSS --c-raw-013
    const bgColor = targetTheme === "light" ? "#ffffff" : "#0f0f0f";
    invoke("set_background_color", { color: bgColor }).catch(() => {});
  }, []);

  const setTheme = useCallback((newTheme: "light" | "dark" | "system") => {
    setThemeState(newTheme);
    localStorage.setItem(THEME_STORAGE_KEY, newTheme);

    loadPreferences().then((prefs) => {
      if (prefs.theme !== newTheme) {
        savePreferences({ ...prefs, theme: newTheme }).catch(console.error);
      }
    });
  }, []);

  useEffect(() => {
    let abortController = new AbortController();

    const setupTheme = async () => {
      if (theme === "system") {
        try {
          const sysTheme = await invoke<string>("get_system_theme");
          if (abortController.signal.aborted) return;
          const newResolved = sysTheme === "dark" ? "dark" : "light";
          setResolvedTheme(newResolved);
          applyDomTheme(newResolved);
        } catch (e) {
          console.warn(
            "Failed to get system theme from Rust, using fallback",
            e,
          );
          const fallback = getSystemThemeFallback();
          setResolvedTheme(fallback);
          applyDomTheme(fallback);
        }
      } else {
        setResolvedTheme(theme);
        applyDomTheme(theme);
      }
    };

    setupTheme();

    return () => {
      abortController.abort();
    };
  }, [theme, applyDomTheme, getSystemThemeFallback]);

  useEffect(() => {
    let mounted = true;
    loadPreferences()
      .then((prefs) => {
        if (!mounted) return;
        if (prefs.theme && prefs.theme !== theme) {
          setTheme(prefs.theme as "light" | "dark" | "system");
        }
      })
      .finally(() => {
        if (mounted) setReady(true);
      });

    return () => {
      mounted = false;
    };
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme, ready }}>
      {children}
    </ThemeContext.Provider>
  );
};
