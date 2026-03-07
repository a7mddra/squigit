/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { AppRouter } from "@/router/AppRouter";

function App() {
  useEffect(() => {
    const handleAnchorClick = (e: MouseEvent) => {
      const target = (e.target as HTMLElement).closest("a");
      if (target && target.href && target.href.startsWith("http")) {
        e.preventDefault();
        invoke("open_external_url", { url: target.href });
      }
    };

    document.addEventListener("click", handleAnchorClick);
    return () => document.removeEventListener("click", handleAnchorClick);
  }, []);

  return (
    <ThemeProvider>
      <AppRouter />
    </ThemeProvider>
  );
}

export default App;
