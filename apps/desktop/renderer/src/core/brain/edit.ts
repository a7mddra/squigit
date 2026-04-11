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
import { setImageDescription, setImageBrief, addToHistory } from "./context";
import { buildContextWindow } from "./summarize";
import { normalizeMessageForHistory } from "./attachmentMemory";
import { saveImageBrief } from "@/core";

export const retryFromMessage = async (
  messageIndex: number,
  allMessages: Array<{ role: string; text: string }>,
  modelId: string,
  chatId?: string | null,
  onToken?: (token: string) => void,
  fallbackImagePath?: string,
  onBriefReady?: (brief: string) => void,
  onEvent?: (event: GeminiStreamEvent) => void,
): Promise<string> => {
  if (!geminiStore.storedApiKey) throw new Error("Gemini API Key not set");

  geminiStore.currentModelId = modelId;
  const myGenId = geminiStore.generationId;

  if (!geminiStore.storedImagePath && fallbackImagePath) {
    geminiStore.storedImagePath = fallbackImagePath;
  }

  if (messageIndex === 0) {
    if (!geminiStore.storedImagePath) {
      throw new Error("Image not found");
    }

    geminiStore.imageDescription = null;
    geminiStore.userFirstMsg = null;
    geminiStore.conversationHistory = [];

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
        onToken?.(token);
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
      const generateAndSaveBrief = async () => {
        let attempt = 0;
        let lastError = null;
        while (attempt < 5) {
          try {
            const brief = await invoke<string>("generate_image_brief", {
              apiKey: geminiStore.storedApiKey,
              imagePath: geminiStore.storedImagePath,
            });
            if (brief) {
              if (chatId) {
                saveImageBrief(chatId, brief).catch(console.error);
              }
              if (geminiStore.generationId === myGenId) {
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
      await Promise.race([
        invoke("stream_gemini_chat_v2", {
          apiKey: geminiStore.storedApiKey,
          model: geminiStore.currentModelId,
          isInitialTurn: true,
          imagePath: geminiStore.storedImagePath,
          imageDescription: null,
          userFirstMsg: null,
          historyLog: null,
          rollingSummary: null,
          userMessage: "",
          channelId,
          chatId: chatId ?? null,
          userName: geminiStore.userName,
          userEmail: geminiStore.userEmail,
          userInstruction: geminiStore.userInstruction,
          imageBrief: "",
        }),
        streamWatchdog.stallPromise,
      ]);
      streamWatchdog.stop();

      unlisten();
      if (geminiStore.currentUnlisten === unlisten)
        geminiStore.currentUnlisten = null;
      if (geminiStore.currentChannelId === channelId)
        geminiStore.currentChannelId = null;

      if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

      setImageDescription(fullResponse);
      geminiStore.conversationHistory = [
        { role: "Assistant", content: fullResponse },
      ];

      return fullResponse;
    } catch (error) {
      streamWatchdog.stop();
      unlisten();
      if (geminiStore.currentUnlisten === unlisten)
        geminiStore.currentUnlisten = null;
      if (geminiStore.currentChannelId === channelId)
        geminiStore.currentChannelId = null;
      throw error;
    }
  }

  const imgDesc = allMessages[0]?.text || geminiStore.imageDescription || "";
  geminiStore.imageDescription = imgDesc;

  const messagesBefore = allMessages.slice(0, messageIndex);
  const firstUser = messagesBefore.find((m) => m.role === "user");
  geminiStore.userFirstMsg = firstUser?.text || null;

  geminiStore.conversationHistory = messagesBefore.map((m) => ({
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
      onToken?.(token);
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
    const { historyLog, rollingSummary } = buildContextWindow();

    streamWatchdog.touch();
    await Promise.race([
      invoke("stream_gemini_chat_v2", {
        apiKey: geminiStore.storedApiKey,
        model: geminiStore.currentModelId,
        isInitialTurn: false,
        imagePath: null, // Image never re-sent, brief is used instead
        imageDescription: imgDesc,
        userFirstMsg: geminiStore.userFirstMsg,
        historyLog,
        rollingSummary,
        userMessage: retryUserMessage,
        channelId,
        chatId: chatId ?? null,
        userName: geminiStore.userName,
        userEmail: geminiStore.userEmail,
        userInstruction: null, // One-time intent hook not needed on retries
        imageBrief: geminiStore.imageBrief,
      }),
      streamWatchdog.stallPromise,
    ]);
    streamWatchdog.stop();

    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    if (geminiStore.currentChannelId === channelId)
      geminiStore.currentChannelId = null;

    if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

    addToHistory("Assistant", fullResponse);

    return fullResponse;
  } catch (error) {
    streamWatchdog.stop();
    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    if (geminiStore.currentChannelId === channelId)
      geminiStore.currentChannelId = null;
    throw error;
  }
};
