/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { github } from "@/core/services";
import packageJson from "@/../package.json";

const TAURI_CHANGELOG_URL = github.rawChangelog;

export type UpdateComponent = "tauri" | "ocr" | "stt";

export interface ReleaseInfo {
  component: UpdateComponent;
  version: string;
  notes: string;
  sections?: Record<string, string[]>;
  size?: string;
  hasUpdate: boolean;
}

const STORAGE_KEY = "pending_updates_queue";

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

async function getSidecarVersion(cmd: string): Promise<string | null> {
  try {
    const output = await invoke<string>("run_sidecar_version", {
      command: cmd,
    });
    const match = output.match(/(\d+\.\d+\.\d+)/);
    return match ? match[1] : null;
  } catch {
    return null; // not installed or error
  }
}

async function fetchReleaseNotes(
  url: string,
  currentVersion: string,
  component: UpdateComponent,
): Promise<ReleaseInfo | null> {
  try {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch release notes: ${response.statusText}`);
    }
    const text = await response.text();

    const headerRegex = /^##\s+\[?(\d+\.\d+\.\d+)\]?.*$/gm;
    const matches = Array.from(text.matchAll(headerRegex));

    if (matches.length === 0) {
      return null;
    }

    const latestMatch = matches[0];
    const latestVersion = latestMatch[1];

    if (compareVersions(latestVersion, currentVersion) <= 0) {
      return null;
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
      component,
      version: latestVersion,
      notes: notes,
      hasUpdate: true,
      sections,
      size,
    };
  } catch (error) {
    console.error(`Error fetching release notes for ${component}:`, error);
    return null;
  }
}

export function useUpdateCheck() {
  useEffect(() => {
    const checkAll = async () => {
      const queue: ReleaseInfo[] = [];

      // 1. Tauri
      const tauriUpdate = await fetchReleaseNotes(
        TAURI_CHANGELOG_URL,
        packageJson.version,
        "tauri",
      );
      if (tauriUpdate?.hasUpdate) queue.push(tauriUpdate);

      // 2. OCR
      const installedOcrVersion = await getSidecarVersion(
        "squigit-ocr --version",
      );
      if (installedOcrVersion) {
        const ocrUpdate = await fetchReleaseNotes(
          "https://raw.githubusercontent.com/a7mddra/squigit/main/sidecars/paddle-ocr/CHANGELOG.md",
          installedOcrVersion,
          "ocr",
        );
        if (ocrUpdate?.hasUpdate) queue.push(ocrUpdate);
      }

      // 3. STT (Future)
      const installedSttVersion = await getSidecarVersion(
        "squigit-stt --version",
      );
      if (installedSttVersion) {
        const sttUpdate = await fetchReleaseNotes(
          "https://raw.githubusercontent.com/a7mddra/squigit/main/sidecars/whisper-stt/CHANGELOG.md",
          installedSttVersion,
          "stt",
        );
        if (sttUpdate?.hasUpdate) queue.push(sttUpdate);
      }

      if (queue.length > 0) {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(queue));
      } else {
        localStorage.removeItem(STORAGE_KEY);
      }
    };

    checkAll();
  }, []);
}

export function getPendingUpdate(): ReleaseInfo | null {
  const queueJson = localStorage.getItem(STORAGE_KEY);
  if (!queueJson) return null;

  try {
    const queue: ReleaseInfo[] = JSON.parse(queueJson);
    return queue[0] || null;
  } catch {
    return null;
  }
}

export function clearPendingUpdate() {
  localStorage.removeItem(STORAGE_KEY);
}

export function markUpdateDone(component: UpdateComponent) {
  const queueJson = localStorage.getItem(STORAGE_KEY);
  if (!queueJson) return;

  try {
    let queue: ReleaseInfo[] = JSON.parse(queueJson);
    queue = queue.filter((u) => u.component !== component);

    if (queue.length === 0) {
      localStorage.removeItem(STORAGE_KEY);
    } else {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(queue));
    }
  } catch {
    localStorage.removeItem(STORAGE_KEY);
  }
}
