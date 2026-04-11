/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { DEFAULT_MODEL_ID } from "@/core/config/models";

export const GEMINI_FALLBACK_MODEL_ID = DEFAULT_MODEL_ID;
export const GEMINI_TITLE_MODEL_PRIMARY = DEFAULT_MODEL_ID;
export const GEMINI_TITLE_MODEL_RETRY = DEFAULT_MODEL_ID;

export function shouldFallbackToGeminiDefaultModel(
  currentModel: string,
  error: unknown,
): boolean {
  if (currentModel === GEMINI_FALLBACK_MODEL_ID) {
    return false;
  }

  const message =
    typeof error === "string"
      ? error
      : error && typeof error === "object" && "message" in error
        ? String((error as { message?: unknown }).message ?? "")
        : String(error ?? "");

  return message.includes("429") || message.includes("503");
}
