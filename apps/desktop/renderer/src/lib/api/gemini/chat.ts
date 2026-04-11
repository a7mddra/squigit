/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { geminiStore } from "./store";
import { GeminiStreamEvent } from "./gemini.types";
import { cancelCurrentRequest } from "./cancel";
import { createStreamWatchdog } from "./streamWatchdog";
import {
  resetBrainContext,
  setUserInfo,
  setImageDescription,
  setImageBrief,
  addToHistory,
} from "./context";

export const startNewThreadStream = async (
  modelId: string,
  imagePath: string,
  onToken: (token: string) => void,
  chatId?: string | null,
  userName?: string,
  userEmail?: string,
  userInstruction?: string,
  onBriefReady?: (brief: string) => void,
  onEvent?: (event: GeminiStreamEvent) => void,
): Promise<string> => {
  if (!geminiStore.storedApiKey) throw new Error("Gemini API Key not set");

  cancelCurrentRequest();
  geminiStore.currentAbortController = new AbortController();
  geminiStore.currentModelId = modelId;
  const myGenId = geminiStore.generationId;

  resetBrainContext();
  setUserInfo(userName, userEmail, userInstruction);
  geminiStore.storedImagePath = imagePath;

  const channelId = `gemini-stream-${Date.now()}`;
  geminiStore.currentChannelId = channelId;
  let fullResponse = "";
  const streamWatchdog = createStreamWatchdog(() => cancelCurrentRequest());

  const unlisten = await listen<GeminiStreamEvent>(channelId, (event) => {
    if (geminiStore.generationId !== myGenId) return;
    streamWatchdog.touch();
    const payload: any = event.payload;
    if (!payload?.type || payload.type === "token") {
      const token = payload.token || "";
      fullResponse += token;
      onToken(token);
      onEvent?.({ type: "token", token });
      return;
    }
    if (payload.type === "reset") {
      fullResponse = "";
      onEvent?.(payload as GeminiStreamEvent);
      return;
    }
    onEvent?.(payload as GeminiStreamEvent);
  });
  geminiStore.currentUnlisten = unlisten;

  try {
    console.log(`[GeminiClient] Starting New Stream`);
    console.log(`[GeminiClient] Target Model: ${modelId}`);

    // Fire image brief in parallel
    const briefPromise = invoke<string>("generate_image_brief", {
      apiKey: geminiStore.storedApiKey,
      imagePath,
    })
      .then((brief) => {
        if (brief && onBriefReady) {
          onBriefReady(brief);
        }
        return brief;
      })
      .catch((e) => {
        console.warn("[GeminiClient] Image brief failed, continuing without:", e);
        return "";
      });

    void briefPromise.then((brief) => {
      if (geminiStore.generationId !== myGenId) return;
      if (brief) {
        setImageBrief(brief);
      }
    });

    streamWatchdog.touch();
    await Promise.race([
      invoke("stream_gemini_chat_v2", {
        apiKey: geminiStore.storedApiKey,
        model: modelId,
        isInitialTurn: true,
        imagePath,
        imageDescription: null,
        userFirstMsg: null,
        historyLog: null,
        rollingSummary: null,
        userMessage: "",
        channelId: channelId,
        chatId: chatId ?? null,
        userName,
        userEmail,
        userInstruction,
        imageBrief: "", // Empty on initial turn
      }),
      streamWatchdog.stallPromise,
    ]);
    streamWatchdog.stop();

    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    if (geminiStore.currentChannelId === channelId)
      geminiStore.currentChannelId = null;
    geminiStore.currentAbortController = null;

    if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

    setImageDescription(fullResponse);
    addToHistory("Assistant", fullResponse);

    console.log(
      `[GeminiClient] Stream Completed successfully. Length: ${fullResponse.length} chars.`,
    );
    return fullResponse.trim();
  } catch (error) {
    streamWatchdog.stop();
    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    if (geminiStore.currentChannelId === channelId)
      geminiStore.currentChannelId = null;
    geminiStore.currentAbortController = null;
    if (error instanceof Error && error.message === "CANCELLED") throw error;
    console.error("Backend stream error:", error);
    throw error;
  }
};

export const startNewThread = async (
  modelId: string,
  imagePath: string,
  chatId?: string | null,
): Promise<string> => {
  let fullText = "";
  await startNewThreadStream(modelId, imagePath, (token) => {
    fullText += token;
  }, chatId);
  return fullText;
};
