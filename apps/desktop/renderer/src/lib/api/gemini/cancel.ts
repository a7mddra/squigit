/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { geminiStore } from "./store";

export const cancelCurrentRequest = () => {
  geminiStore.generationId++;
  if (geminiStore.currentAbortController) {
    geminiStore.currentAbortController.abort();
    geminiStore.currentAbortController = null;
  }
  if (geminiStore.currentUnlisten) {
    geminiStore.currentUnlisten();
    geminiStore.currentUnlisten = null;
  }
  if (geminiStore.currentChannelId) {
    invoke("cancel_gemini_request", {
      channelId: null,
    }).catch(console.error);
    geminiStore.currentChannelId = null;
  } else {
    invoke("cancel_gemini_request", {
      channelId: null,
    }).catch(console.error);
  }
};

export const quickAnswerCurrentRequest = async () => {
  const channelId = geminiStore.currentChannelId;
  if (!channelId) return;

  try {
    await invoke("quick_answer_gemini_request", { channelId });
  } catch (error) {
    console.error("Failed to request answer-now:", error);
  }
};
