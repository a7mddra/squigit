/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useMemo } from "react";

export type Platform = "macos" | "windows" | "linux";

interface PlatformInfo {
  os: Platform;
  isMac: boolean;
  isWin: boolean;
  isLinux: boolean;
  modKey: "Meta" | "Control";
  modSymbol: string;
  altSymbol: string;
  shiftSymbol: string;
  enterSymbol: string;
}

export function usePlatform(): PlatformInfo {
  return useMemo(() => {
    const ua = navigator.userAgent.toLowerCase();
    let os: Platform = "linux";
    if (ua.includes("mac")) {
      os = "macos";
    } else if (ua.includes("win")) {
      os = "windows";
    }

    const isMac = os === "macos";
    const isWin = os === "windows";
    const isLinux = os === "linux";

    return {
      os,
      isMac,
      isWin,
      isLinux,
      modKey: isMac ? "Meta" : "Control",
      modSymbol: isMac ? "⌘" : "Ctrl",
      altSymbol: isMac ? "⌥" : "Alt",
      shiftSymbol: "⇧",
      enterSymbol: "↵",
    };
  }, []);
}
