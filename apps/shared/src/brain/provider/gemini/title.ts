/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  generateGeminiChatTitle,
} from "./commands";
import {
  GEMINI_TITLE_MODEL_PRIMARY,
  GEMINI_TITLE_MODEL_RETRY,
} from "./models";

export async function generateProviderTitle(
  apiKey: string,
  text: string,
): Promise<string> {
  if (!apiKey || !text) return "New thread";

  try {
    const title = await generateGeminiChatTitle(
      apiKey,
      GEMINI_TITLE_MODEL_PRIMARY,
      text,
    );

    console.log(`[BrainTitle] Selected Model: ${GEMINI_TITLE_MODEL_PRIMARY}`);
    console.log(`[BrainTitle] Generated Chat Title: "${title || "New thread"}"`);
    return title || "New thread";
  } catch (primaryError) {
    console.warn(
      "Failed to generate text title with primary model, retrying fallback:",
      primaryError,
    );

    try {
      const fallbackTitle = await generateGeminiChatTitle(
        apiKey,
        GEMINI_TITLE_MODEL_RETRY,
        text,
      );

      console.log(
        `[BrainTitle] Selected Model: ${GEMINI_TITLE_MODEL_RETRY} (Retry)`,
      );
      console.log(
        `[BrainTitle] Generated Chat Title: "${fallbackTitle || "New thread"}"`,
      );
      return fallbackTitle || "New thread";
    } catch (fallbackError) {
      console.error(
        "Failed to generate text title with fallback model:",
        fallbackError,
      );
      return "New thread";
    }
  }
}
