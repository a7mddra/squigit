/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { brainSessionStore } from "../../../session/store";
import type { ProviderStreamEvent } from "../../../engine/types";
import {
  resetBrainContext,
  setUserInfo,
  setImageDescription,
  setImageBrief,
  addToHistory,
} from "../../../session/context";
import {
  cancelCurrentRequest,
  clearActiveProviderTransport,
  createProviderChannelId,
  createStreamWatchdog,
} from "../transport";
import {
  generateGeminiImageBrief,
  listenGeminiStream,
  streamGeminiChat,
} from "../commands";
import { saveImageBrief } from "../../../../config/chat-storage";
import { MODEL_IDS } from "../../../../config/models-config";

export const startNewThreadStream = async (
  modelId: string,
  imagePath: string,
  onToken: (token: string) => void,
  chatId?: string | null,
  userName?: string,
  userEmail?: string,
  userInstruction?: string,
  onBriefReady?: (brief: string) => void,
  onEvent?: (event: ProviderStreamEvent) => void,
): Promise<string> => {
  if (!brainSessionStore.storedApiKey)
    throw new Error("Gemini API Key not set");

  cancelCurrentRequest();
  brainSessionStore.currentAbortController = new AbortController();
  brainSessionStore.currentModelId = modelId;
  const myGenId = brainSessionStore.generationId;

  resetBrainContext();
  setUserInfo(userName, userEmail, userInstruction);
  brainSessionStore.storedImagePath = imagePath;

  const channelId = createProviderChannelId();
  brainSessionStore.currentChannelId = channelId;
  let fullResponse = "";
  const streamWatchdog = createStreamWatchdog(() => cancelCurrentRequest());

  const unlisten = await listenGeminiStream(channelId, (payload) => {
    if (brainSessionStore.generationId !== myGenId) return;
    streamWatchdog.touch();
    const normalizedPayload = payload as ProviderStreamEvent & {
      type?: ProviderStreamEvent["type"];
      token?: string;
    };
    if (!normalizedPayload?.type || normalizedPayload.type === "token") {
      const token = normalizedPayload.token || "";
      fullResponse += token;
      onToken(token);
      onEvent?.({ type: "token", token });
      return;
    }
    if (normalizedPayload.type === "reset") {
      fullResponse = "";
      onEvent?.(normalizedPayload as ProviderStreamEvent);
      return;
    }
    onEvent?.(normalizedPayload as ProviderStreamEvent);
  });
  brainSessionStore.currentUnlisten = unlisten;

  try {
    console.log(`[GeminiClient] Starting New Stream`);
    console.log(`[GeminiClient] Target Model: ${modelId}`);

    const generateAndSaveBrief = async () => {
      // Delay brief generation by 5000ms so it doesn't fire concurrently with the main stream
      // during high-demand/cold-start situations, which prevents simultaneous 503s.
      await new Promise((r) => setTimeout(r, 5000));

      let attempt = 0;
      let lastError = null;
      const delays = [1000, 2000];
      while (attempt < 3) {
        try {
          const providerApiKey = brainSessionStore.storedApiKey;
          if (!providerApiKey) return "";
          const modelToUse = MODEL_IDS.MICRO_TASKS;
          const brief = await generateGeminiImageBrief(
            providerApiKey,
            imagePath,
            modelToUse,
          );
          if (brief) {
            if (chatId) {
              saveImageBrief(chatId, brief).catch(console.error);
            }
            if (brainSessionStore.generationId === myGenId) {
              setImageBrief(brief);
              if (onBriefReady) onBriefReady(brief);
            }
            return brief;
          }
          return "";
        } catch (e) {
          console.warn(
            `[GeminiClient] Image brief attempt ${attempt + 1} failed:`,
            e,
          );
          lastError = e;
          attempt++;
          if (attempt < 3) {
            const jitter = Math.floor(Math.random() * 500);
            await new Promise((r) =>
              setTimeout(r, delays[attempt - 1] + jitter),
            );
          }
        }
      }
      console.warn("[GeminiClient] Image brief failed all retries.", lastError);
      return "";
    };

    void generateAndSaveBrief();

    streamWatchdog.touch();
    const providerApiKey = brainSessionStore.storedApiKey;
    if (!providerApiKey) {
      throw new Error("Gemini API Key not set");
    }
    await Promise.race([
      streamGeminiChat({
        apiKey: providerApiKey,
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

    clearActiveProviderTransport(channelId, unlisten);
    brainSessionStore.currentAbortController = null;

    if (brainSessionStore.generationId !== myGenId)
      throw new Error("CANCELLED");

    setImageDescription(fullResponse);
    addToHistory("Assistant", fullResponse);

    console.log(
      `[GeminiClient] Stream Completed successfully. Length: ${fullResponse.length} chars.`,
    );
    return fullResponse.trim();
  } catch (error) {
    streamWatchdog.stop();
    clearActiveProviderTransport(channelId, unlisten);
    brainSessionStore.currentAbortController = null;
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
  await startNewThreadStream(
    modelId,
    imagePath,
    (token) => {
      fullText += token;
    },
    chatId,
  );
  return fullText;
};
