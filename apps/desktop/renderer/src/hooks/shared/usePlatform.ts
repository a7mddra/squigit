/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useMemo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  pkgMgrName: string;
  getPkgInstallCmd: (pkg: string) => string;
  getPkgUpgradeCmd: (pkg: string) => string;
}

export function usePlatform(): PlatformInfo {
  const [linuxDistro, setLinuxDistro] = useState<
    "debian" | "rpm" | "unknown"
  >("unknown");
  const linuxPackagesBaseUrl = "https://squigit-org.github.io/squigit-packages";
  const linuxAptRepoBaseUrl =
    "https://github.com/squigit-org/squigit-packages/raw/main/apt";

  const base = useMemo(() => {
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

    return { os, isMac, isWin, isLinux };
  }, []);

  useEffect(() => {
    if (base.isLinux) {
      invoke<"debian" | "rpm" | "unknown">("get_linux_package_manager")
        .then(setLinuxDistro)
        .catch(console.error);
    }
  }, [base.isLinux]);

  return useMemo(() => {
    const { isMac, isWin, isLinux, os } = base;

    let pkgMgrName = "APT / DNF";
    if (isMac) pkgMgrName = "Homebrew";
    else if (isWin) pkgMgrName = "Winget";
    else if (linuxDistro === "debian") pkgMgrName = "APT";
    else if (linuxDistro === "rpm") pkgMgrName = "DNF";

    const getPkgInstallCmd = (pkg: string) => {
      if (isMac) return `brew install ${pkg}`;
      if (isWin) return `winget install ${pkg}`;
      if (linuxDistro === "debian") {
        return [
          "# 1) add repo",
          "sudo mkdir -p /etc/apt/keyrings",
          `curl -fsSL ${linuxPackagesBaseUrl}/keys/squigit-packages.asc | gpg --dearmor | sudo tee /etc/apt/keyrings/squigit-packages.gpg >/dev/null`,
          `echo "deb [signed-by=/etc/apt/keyrings/squigit-packages.gpg] ${linuxAptRepoBaseUrl} stable ocr stt" | sudo tee /etc/apt/sources.list.d/squigit-packages.list >/dev/null`,
          "",
          "# 2) update",
          "sudo apt update",
          "",
          "# 3) install",
          `sudo apt install ${pkg}`,
        ].join("\n");
      }
      if (linuxDistro === "rpm") {
        return [
          "# 1) add repo",
          `sudo curl -fsSL ${linuxPackagesBaseUrl}/rpm/squigit.repo -o /etc/yum.repos.d/squigit.repo`,
          "",
          "# 2) update",
          "sudo dnf makecache",
          "",
          "# 3) install",
          `sudo dnf install ${pkg}`,
        ].join("\n");
      }
      return [
        "# Debian/Ubuntu",
        "sudo mkdir -p /etc/apt/keyrings",
        `curl -fsSL ${linuxPackagesBaseUrl}/keys/squigit-packages.asc | gpg --dearmor | sudo tee /etc/apt/keyrings/squigit-packages.gpg >/dev/null`,
        `echo "deb [signed-by=/etc/apt/keyrings/squigit-packages.gpg] ${linuxAptRepoBaseUrl} stable ocr stt" | sudo tee /etc/apt/sources.list.d/squigit-packages.list >/dev/null`,
        "sudo apt update",
        `sudo apt install ${pkg}`,
        "",
        "# Fedora/RHEL",
        `sudo curl -fsSL ${linuxPackagesBaseUrl}/rpm/squigit.repo -o /etc/yum.repos.d/squigit.repo`,
        "sudo dnf makecache",
        `sudo dnf install ${pkg}`,
      ].join("\n");
    };

    const getPkgUpgradeCmd = (pkg: string) => {
      if (isMac) return `brew upgrade ${pkg}`;
      if (isWin) return `winget upgrade ${pkg}`;
      if (linuxDistro === "debian")
        return `sudo apt update && sudo apt install --only-upgrade ${pkg}`;
      if (linuxDistro === "rpm") return `sudo dnf upgrade ${pkg}`;
      return `# Ubuntu/Debian\nsudo apt update && sudo apt install --only-upgrade ${pkg}\n\n# Fedora/RHEL\nsudo dnf upgrade ${pkg}`;
    };

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
      pkgMgrName,
      getPkgInstallCmd,
      getPkgUpgradeCmd,
    };
  }, [base, linuxDistro]);
}
