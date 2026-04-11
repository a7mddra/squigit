/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ModelType } from "@/core";

const TITLE_MODEL_PRIMARY = ModelType.GEMINI_3_1_FLASH;
const TITLE_MODEL_RETRY = ModelType.GEMINI_3_1_FLASH;

interface UseChatTitleProps {
  apiKey: string;
}

export const useChatTitle = ({ apiKey }: UseChatTitleProps) => {
  const [isGenerating, setIsGenerating] = useState(false);

  const generateTitleForText = useCallback(
    async (text: string): Promise<string> => {
      if (!apiKey || !text) return "New thread";

      setIsGenerating(true);

      try {
        const title = await invoke<string>("generate_chat_title", {
          apiKey,
          model: TITLE_MODEL_PRIMARY,
          promptContext: text,
        });

        console.log(
          `[ChatTitleTracker] Selected Model: ${TITLE_MODEL_PRIMARY}`,
        );
        console.log(
          `[ChatTitleTracker] Generated Chat Title: "${title || "New thread"}"`,
        );
        return title || "New thread";
      } catch (liteError) {
        console.warn(
          "Failed to generate text title with primary model, retrying flash:",
          liteError,
        );

        try {
          const fallbackTitle = await invoke<string>("generate_chat_title", {
            apiKey,
            model: TITLE_MODEL_RETRY,
            promptContext: text,
          });

          console.log(
            `[ChatTitleTracker] Selected Model: ${TITLE_MODEL_RETRY} (Retry)`,
          );
          console.log(
            `[ChatTitleTracker] Generated Chat Title: "${fallbackTitle || "New thread"}"`,
          );
          return fallbackTitle || "New thread";
        } catch (flashError) {
          console.error(
            "Failed to generate text title with fallback model:",
            flashError,
          );
          return "New thread";
        }
      } finally {
        setIsGenerating(false);
      }
    },
    [apiKey],
  );

  return {
    isGeneratingTitle: isGenerating,
    generateTitleForText,
  };
};
