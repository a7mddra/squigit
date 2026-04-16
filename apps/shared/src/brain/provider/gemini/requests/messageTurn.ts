/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { brainSessionStore } from "../../../session/store";
import type { ProviderStreamEvent } from "../../../engine/types";
import {
  cancelCurrentRequest,
  clearActiveProviderTransport,
  createProviderChannelId,
  createStreamWatchdog,
} from "../transport";
import { setUserFirstMsg, addToHistory } from "../../../session/context";
import {
  buildContextWindow,
  maybeCompressHistory,
} from "../../../session/summarizer";
import { normalizeMessageForHistory } from "../../../session/attachments/memory";
import { listenGeminiStream, streamGeminiChat } from "../commands";

export const sendMessage = async (
  text: string,
  modelId?: string,
  onToken?: (token: string) => void,
  chatId?: string | null,
  onEvent?: (event: ProviderStreamEvent) => void,
): Promise<string> => {
  if (!brainSessionStore.storedApiKey)
    throw new Error("Gemini API Key not set");
  if (!brainSessionStore.imageDescription)
    throw new Error("No active chat session");

  if (modelId) {
    brainSessionStore.currentModelId = modelId;
  }

  const myGenId = brainSessionStore.generationId;
  const normalizedUserText = normalizeMessageForHistory(text);
  setUserFirstMsg(normalizedUserText);
  addToHistory("User", normalizedUserText);

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
      onToken?.(token);
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
    console.log(`[GeminiClient] Sending Message`);
    console.log(
      `[GeminiClient] Target Model: ${brainSessionStore.currentModelId}`,
    );
    console.log(`[GeminiClient] Prompt: "${text}"`);
    console.log(
      `[GeminiClient] Image Brief Present: ${Boolean(brainSessionStore.imageBrief)}`,
    );

    const { historyLog, rollingSummary } = buildContextWindow();
    const providerApiKey = brainSessionStore.storedApiKey;
    if (!providerApiKey) {
      throw new Error("Gemini API Key not set");
    }

    streamWatchdog.touch();
    await Promise.race([
      streamGeminiChat({
        apiKey: providerApiKey,
        model: brainSessionStore.currentModelId,
        isInitialTurn: false,
        imagePath: null, // Image never re-sent, brief is used instead
        imageDescription: brainSessionStore.imageDescription,
        userFirstMsg: brainSessionStore.userFirstMsg,
        historyLog,
        rollingSummary,
        userMessage: normalizedUserText,
        channelId: channelId,
        chatId: chatId ?? null,
        userName: brainSessionStore.userName ?? undefined,
        userEmail: brainSessionStore.userEmail ?? undefined,
        userInstruction: null, // One-time intent hook only sent on initial turn
        imageBrief: brainSessionStore.imageBrief,
      }),
      streamWatchdog.stallPromise,
    ]);
    streamWatchdog.stop();
    clearActiveProviderTransport(channelId, unlisten);

    if (brainSessionStore.generationId !== myGenId)
      throw new Error("CANCELLED");

    addToHistory("Assistant", fullResponse);

    // Fire-and-forget: compress older turns if threshold reached
    maybeCompressHistory(chatId ?? null);

    console.log(
      `[GeminiClient] Stream Completed. Final Response: "${fullResponse}"`,
    );
    return fullResponse;
  } catch (error) {
    streamWatchdog.stop();
    clearActiveProviderTransport(channelId, unlisten);
    if (error instanceof Error && error.message === "CANCELLED") throw error;
    console.error("SendMessage error:", error);
    throw error;
  }
};
