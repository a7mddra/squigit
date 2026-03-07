/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useEffect } from "react";
import { github } from "@/lib";
import packageJson from "../../../package.json";

const RELEASE_NOTES_URL = github.rawChangelog;

export interface ReleaseInfo {
  version: string;
  notes: string;
  hasUpdate: boolean;
  sections?: Record<string, string[]>;
  size?: string;
}

const STORAGE_KEYS = {
  VERSION: "pending_update_version",
  NOTES: "pending_update_notes",
  SECTIONS: "pending_update_sections",
  SIZE: "pending_update_size",
  AVAILABLE: "pending_update_available",
};

function compareVersions(v1: string, v2: string): number {
  const parts1 = v1.split(".").map(Number);
  const parts2 = v2.split(".").map(Number);

  for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
    const p1 = parts1[i] || 0;
    const p2 = parts2[i] || 0;
    if (p1 > p2) return 1;
    if (p1 < p2) return -1;
  }
  return 0;
}

export const fetchReleaseNotes = async (): Promise<ReleaseInfo> => {
  try {
    const response = await fetch(RELEASE_NOTES_URL);
    if (!response.ok) {
      throw new Error(`Failed to fetch release notes: ${response.statusText}`);
    }
    const text = await response.text();

    const headerRegex = /^##\s+\[?(\d+\.\d+\.\d+)\]?.*$/gm;
    const matches = Array.from(text.matchAll(headerRegex));

    if (matches.length === 0) {
      return { version: packageJson.version, notes: "", hasUpdate: false };
    }

    const latestMatch = matches[0];
    const latestVersion = latestMatch[1];

    if (compareVersions(latestVersion, packageJson.version) <= 0) {
      return { version: latestVersion, notes: "", hasUpdate: false };
    }

    const startIdx = latestMatch.index! + latestMatch[0].length;
    const endIdx = matches.length > 1 ? matches[1].index! : text.length;
    const rawBody = text.slice(startIdx, endIdx);

    const versionInfoMatch = rawBody.match(
      /###\s+Version Info\n([\s\S]*?)(?=\n###\s+|$)/,
    );
    let size = undefined;
    if (versionInfoMatch) {
      const versionInfoContent = versionInfoMatch[1].trim();
      const sizeMatch = versionInfoContent.match(/\*\*Size\*\*:\s*(.+)/i);
      size = sizeMatch ? sizeMatch[1].trim() : undefined;
    }

    const sections: Record<string, string[]> = {
      "New Features": [],
      "Bug Fixes": [],
      "UI Improvements": [],
    };

    const sectionRegex = /###\s+(.*?)\n([\s\S]*?)(?=\n###\s+|$)/g;
    let sectionMatch;

    while ((sectionMatch = sectionRegex.exec(rawBody)) !== null) {
      const title = sectionMatch[1].trim();

      if (title === "Version Info") continue;

      const content = sectionMatch[2].trim();

      const items = content
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.startsWith("-") || line.startsWith("*"))
        .map((line) => line.replace(/^[-*]\s+/, ""))
        .map((line) =>
          line
            .replace(/\*\*(.*?)\*\*/g, "$1")
            .replace(/__(.*?)__/g, "$1")
            .replace(/\*(.*?)\*/g, "$1")
            .replace(/_(.*?)_/g, "$1")
            .replace(/`([^`]+)`/g, "$1")
            .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1"),
        );

      if (sections[title]) {
        sections[title] = items;
      }
    }

    let notes = rawBody.trim();
    if (Object.values(sections).every((arr) => arr.length === 0)) {
      notes = rawBody
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.startsWith("-") || line.startsWith("*"))
        .join("\n");
    }

    return {
      version: latestVersion,
      notes: notes,
      hasUpdate: true,
      sections,
      size,
    };
  } catch (error) {
    console.error("Error fetching release notes:", error);
    return { version: packageJson.version, notes: "", hasUpdate: false };
  }
};

export function useUpdateCheck() {
  useEffect(() => {
    const checkUpdate = async () => {
      try {
        const { hasUpdate, version, notes, sections, size } =
          await fetchReleaseNotes();

        if (hasUpdate) {
          localStorage.setItem(STORAGE_KEYS.AVAILABLE, "true");
          localStorage.setItem(STORAGE_KEYS.VERSION, version);
          localStorage.setItem(STORAGE_KEYS.NOTES, notes);
          if (sections) {
            localStorage.setItem(
              STORAGE_KEYS.SECTIONS,
              JSON.stringify(sections),
            );
          }
          if (size) {
            localStorage.setItem(STORAGE_KEYS.SIZE, size);
          }
        } else {
          clearPendingUpdate();
        }
      } catch (error) {
        console.error("Failed to check for updates", error);
      }
    };

    checkUpdate();
  }, []);
}

export function getPendingUpdate() {
  const available = localStorage.getItem(STORAGE_KEYS.AVAILABLE) === "true";
  if (!available) return null;

  const storedVersion = localStorage.getItem(STORAGE_KEYS.VERSION) || "";
  const currentVersion = packageJson.version;

  if (compareVersions(storedVersion, currentVersion) <= 0) {
    console.log(
      `Clearing stale update: stored ${storedVersion} <= current ${currentVersion}`,
    );
    clearPendingUpdate();
    return null;
  }

  const sectionsStr = localStorage.getItem(STORAGE_KEYS.SECTIONS);
  let sections;
  try {
    sections = sectionsStr ? JSON.parse(sectionsStr) : undefined;
  } catch (e) {
    console.warn("Failed to parse update sections", e);
  }

  return {
    version: storedVersion,
    notes: localStorage.getItem(STORAGE_KEYS.NOTES) || "",
    sections,
    size: localStorage.getItem(STORAGE_KEYS.SIZE) || undefined,
  };
}

export function clearPendingUpdate() {
  localStorage.removeItem(STORAGE_KEYS.AVAILABLE);
  localStorage.removeItem(STORAGE_KEYS.VERSION);
  localStorage.removeItem(STORAGE_KEYS.NOTES);
  localStorage.removeItem(STORAGE_KEYS.SECTIONS);
  localStorage.removeItem(STORAGE_KEYS.SIZE);
}
