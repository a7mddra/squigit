/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { listen } from "@tauri-apps/api/event";
import { brainSessionStore } from "../../../session/store";
import type { ProviderStreamEvent } from "../../../engine/types";
import { cancelCurrentRequest, clearActiveProviderTransport, createProviderChannelId, createStreamWatchdog } from "../transport";
import { setImageDescription, setImageBrief, addToHistory } from "../../../session/context";
import { buildContextWindow } from "../../../session/summarizer";
import { normalizeMessageForHistory } from "../../../session/attachmentMemory";
import {
  generateGeminiImageBrief,
  streamGeminiChat,
} from "../commands";
import { saveImageBrief } from "@/core/storage/chat";

export const retryFromMessage = async (
  messageIndex: number,
  allMessages: Array<{ role: string; text: string }>,
  modelId: string,
  chatId?: string | null,
  onToken?: (token: string) => void,
  fallbackImagePath?: string,
  onBriefReady?: (brief: string) => void,
  onEvent?: (event: ProviderStreamEvent) => void,
): Promise<string> => {
  if (!brainSessionStore.storedApiKey) throw new Error("Gemini API Key not set");

  brainSessionStore.currentModelId = modelId;
  const myGenId = brainSessionStore.generationId;

  if (!brainSessionStore.storedImagePath && fallbackImagePath) {
    brainSessionStore.storedImagePath = fallbackImagePath;
  }

  if (messageIndex === 0) {
    if (!brainSessionStore.storedImagePath) {
      throw new Error("Image not found");
    }

    brainSessionStore.imageDescription = null;
    brainSessionStore.userFirstMsg = null;
    brainSessionStore.conversationHistory = [];

    const channelId = createProviderChannelId();
    brainSessionStore.currentChannelId = channelId;
    let fullResponse = "";
    const streamWatchdog = createStreamWatchdog(() => cancelCurrentRequest());

    const unlisten = await listen<ProviderStreamEvent>(channelId, (event) => {
      if (brainSessionStore.generationId !== myGenId) return;
      streamWatchdog.touch();
      const payload: any = event.payload;
      if (!payload?.type || payload.type === "token") {
        const token = payload.token || "";
        fullResponse += token;
        onToken?.(token);
        onEvent?.({ type: "token", token });
        return;
      }
      if (payload.type === "reset") {
        fullResponse = "";
        onEvent?.(payload as ProviderStreamEvent);
        return;
      }
      onEvent?.(payload as ProviderStreamEvent);
    });
    brainSessionStore.currentUnlisten = unlisten;

    try {
      const generateAndSaveBrief = async () => {
        let attempt = 0;
        let lastError = null;
        while (attempt < 5) {
          try {
            const providerApiKey = brainSessionStore.storedApiKey;
            const storedImagePath = brainSessionStore.storedImagePath;
            if (!providerApiKey || !storedImagePath) return "";
            const brief = await generateGeminiImageBrief(
              providerApiKey,
              storedImagePath,
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
              `[GeminiClient] Image brief retry attempt ${attempt + 1} failed:`,
              e,
            );
            lastError = e;
            attempt++;
            if (attempt < 5) {
              await new Promise((r) =>
                setTimeout(r, 1000 * Math.pow(2, attempt - 1)),
              );
            }
          }
        }
        console.warn(
          "[GeminiClient] Image brief failed all retries.",
          lastError,
        );
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
          model: brainSessionStore.currentModelId,
          isInitialTurn: true,
          imagePath: brainSessionStore.storedImagePath,
          imageDescription: null,
          userFirstMsg: null,
          historyLog: null,
          rollingSummary: null,
          userMessage: "",
          channelId,
          chatId: chatId ?? null,
          userName: brainSessionStore.userName ?? undefined,
          userEmail: brainSessionStore.userEmail ?? undefined,
          userInstruction: brainSessionStore.userInstruction,
          imageBrief: "",
        }),
        streamWatchdog.stallPromise,
      ]);
      streamWatchdog.stop();

      clearActiveProviderTransport(channelId, unlisten);

      if (brainSessionStore.generationId !== myGenId) throw new Error("CANCELLED");

      setImageDescription(fullResponse);
      brainSessionStore.conversationHistory = [
        { role: "Assistant", content: fullResponse },
      ];

      return fullResponse;
    } catch (error) {
      streamWatchdog.stop();
      clearActiveProviderTransport(channelId, unlisten);
      throw error;
    }
  }

  const imgDesc = allMessages[0]?.text || brainSessionStore.imageDescription || "";
  brainSessionStore.imageDescription = imgDesc;

  const messagesBefore = allMessages.slice(0, messageIndex);
  const firstUser = messagesBefore.find((m) => m.role === "user");
  brainSessionStore.userFirstMsg = firstUser?.text || null;

  brainSessionStore.conversationHistory = messagesBefore.map((m) => ({
    role: m.role === "user" ? "User" : "Assistant",
    content: normalizeMessageForHistory(m.text),
  }));
  // No more slice(-6) — summarize.ts handles windowing

  const lastUserMsg = [...messagesBefore]
    .reverse()
    .find((m) => m.role === "user");
  if (!lastUserMsg) {
    throw new Error("No user message found before the retried message");
  }
  const retryUserMessage = normalizeMessageForHistory(lastUserMsg.text);

  const channelId = createProviderChannelId();
  brainSessionStore.currentChannelId = channelId;
  let fullResponse = "";
  const streamWatchdog = createStreamWatchdog(() => cancelCurrentRequest());

  const unlisten = await listen<ProviderStreamEvent>(channelId, (event) => {
    if (brainSessionStore.generationId !== myGenId) return;
    streamWatchdog.touch();
    const payload: any = event.payload;
    if (!payload?.type || payload.type === "token") {
      const token = payload.token || "";
      fullResponse += token;
      onToken?.(token);
      onEvent?.({ type: "token", token });
      return;
    }
    if (payload.type === "reset") {
      fullResponse = "";
      onEvent?.(payload as ProviderStreamEvent);
      return;
    }
    onEvent?.(payload as ProviderStreamEvent);
  });
  brainSessionStore.currentUnlisten = unlisten;

  try {
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
        imageDescription: imgDesc,
        userFirstMsg: brainSessionStore.userFirstMsg,
        historyLog,
        rollingSummary,
        userMessage: retryUserMessage,
        channelId,
        chatId: chatId ?? null,
        userName: brainSessionStore.userName ?? undefined,
        userEmail: brainSessionStore.userEmail ?? undefined,
        userInstruction: null, // One-time intent hook not needed on retries
        imageBrief: brainSessionStore.imageBrief,
      }),
      streamWatchdog.stallPromise,
    ]);
    streamWatchdog.stop();

    clearActiveProviderTransport(channelId, unlisten);

    if (brainSessionStore.generationId !== myGenId) throw new Error("CANCELLED");

    addToHistory("Assistant", fullResponse);

    return fullResponse;
  } catch (error) {
    streamWatchdog.stop();
    clearActiveProviderTransport(channelId, unlisten);
    throw error;
  }
};
