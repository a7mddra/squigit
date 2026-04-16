/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { PendingAssistantRequestKind, ToolStep } from "./types";
import { API_STATUS_TEXT } from "../../helpers/api-status";

const DEFAULT_THREAD_TITLE_NORMALIZED = "new thread";

export function normalizeThreadTitle(title: string | null | undefined): string {
  if (!title) return "";

  return title.trim().toLowerCase().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
}

export function isUntitledThreadTitle(
  title: string | null | undefined,
): boolean {
  const normalized = normalizeThreadTitle(title);
  return (
    normalized.length === 0 || normalized === DEFAULT_THREAD_TITLE_NORMALIZED
  );
}

export function getThoughtSecondsFromToolSteps(
  steps: ToolStep[],
): number | undefined {
  const seconds = steps.reduce((sum, step) => {
    if (
      typeof step.startedAtMs === "number" &&
      typeof step.endedAtMs === "number" &&
      step.endedAtMs >= step.startedAtMs
    ) {
      return (
        sum + Math.max(1, Math.round((step.endedAtMs - step.startedAtMs) / 1000))
      );
    }

    const match = step.message?.trim().match(/^Thought for (\d+)s$/i);
    if (!match) return sum;

    const parsed = Number.parseInt(match[1], 10);
    if (!Number.isFinite(parsed) || parsed <= 0) return sum;

    return sum + parsed;
  }, 0);

  return seconds > 0 ? seconds : undefined;
}

export function getElapsedThoughtSeconds(startedAtMs: number): number {
  return Math.max(1, Math.round((Date.now() - startedAtMs) / 1000));
}

export function getDefaultProgressText(
  requestKind: PendingAssistantRequestKind,
): string {
  if (requestKind === "initial" || requestKind === "edit") {
    return API_STATUS_TEXT.ANALYZING_IMAGE;
  }

  return "";
}
