/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { geminiStore } from "./store";
import { GeminiEvent } from "./gemini.types";
import { cancelCurrentRequest } from "./cancel";
import {
  resetBrainContext,
  setImageDescription,
  addToHistory,
} from "./context";

export const startNewChatStream = async (
  modelId: string,
  imagePath: string,
  onToken: (token: string) => void,
): Promise<string> => {
  if (!geminiStore.storedApiKey) throw new Error("Gemini API Key not set");

  cancelCurrentRequest();
  geminiStore.currentAbortController = new AbortController();
  geminiStore.currentModelId = modelId;
  const myGenId = geminiStore.generationId;

  resetBrainContext();
  geminiStore.storedImagePath = imagePath;

  const channelId = `gemini-stream-${Date.now()}`;
  geminiStore.currentChannelId = channelId;
  let fullResponse = "";

  const unlisten = await listen<GeminiEvent>(channelId, (event) => {
    if (geminiStore.generationId !== myGenId) return;
    fullResponse += event.payload.token;
    onToken(event.payload.token);
  });
  geminiStore.currentUnlisten = unlisten;

  try {
    console.log(`[GeminiClient] Starting New Stream`);
    console.log(`[GeminiClient] Target Model: ${modelId}`);

    await invoke("stream_gemini_chat_v2", {
      apiKey: geminiStore.storedApiKey,
      model: modelId,
      isInitialTurn: true,
      imageBase64: null,
      imageMimeType: null,
      imagePath,
      imageDescription: null,
      userFirstMsg: null,
      historyLog: null,
      userMessage: "",
      channelId: channelId,
    });

    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    geminiStore.currentAbortController = null;

    if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

    setImageDescription(fullResponse);
    addToHistory("Assistant", fullResponse);

    console.log(
      `[GeminiClient] Stream Completed successfully. Length: ${fullResponse.length} chars.`,
    );
    return fullResponse;
  } catch (error) {
    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    geminiStore.currentAbortController = null;
    if (error instanceof Error && error.message === "CANCELLED") throw error;
    console.error("Backend stream error:", error);
    throw error;
  }
};

export const startNewChat = async (
  modelId: string,
  imagePath: string,
): Promise<string> => {
  let fullText = "";
  await startNewChatStream(modelId, imagePath, (token) => {
    fullText += token;
  });
  return fullText;
};
