/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { geminiStore } from "./store";
import { GeminiEvent } from "./gemini.types";
import { setImageDescription, formatHistoryLog, addToHistory } from "./context";

export const retryFromMessage = async (
  messageIndex: number,
  allMessages: Array<{ role: string; text: string }>,
  modelId: string,
  onToken?: (token: string) => void,
  fallbackImagePath?: string,
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

    const unlisten = await listen<GeminiEvent>(channelId, (event) => {
      if (geminiStore.generationId !== myGenId) return;
      fullResponse += event.payload.token;
      onToken?.(event.payload.token);
    });
    geminiStore.currentUnlisten = unlisten;

    try {
      await invoke("stream_gemini_chat_v2", {
        apiKey: geminiStore.storedApiKey,
        model: geminiStore.currentModelId,
        isInitialTurn: true,
        imageBase64: null,
        imageMimeType: null,
        imagePath: geminiStore.storedImagePath,
        imageDescription: null,
        userFirstMsg: null,
        historyLog: null,
        userMessage: "",
        channelId,
      });

      unlisten();
      if (geminiStore.currentUnlisten === unlisten)
        geminiStore.currentUnlisten = null;

      if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

      setImageDescription(fullResponse);
      geminiStore.conversationHistory = [
        { role: "Assistant", content: fullResponse },
      ];

      return fullResponse;
    } catch (error) {
      unlisten();
      if (geminiStore.currentUnlisten === unlisten)
        geminiStore.currentUnlisten = null;
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
    content: m.text,
  }));

  if (geminiStore.conversationHistory.length > 6) {
    geminiStore.conversationHistory = geminiStore.conversationHistory.slice(-6);
  }

  const lastUserMsg = [...messagesBefore]
    .reverse()
    .find((m) => m.role === "user");
  if (!lastUserMsg) {
    throw new Error("No user message found before the retried message");
  }

  const isFirstTurnWithImage = !firstUser && geminiStore.storedImagePath;

  const channelId = `gemini-stream-${Date.now()}`;
  geminiStore.currentChannelId = channelId;
  let fullResponse = "";

  const unlisten = await listen<GeminiEvent>(channelId, (event) => {
    if (geminiStore.generationId !== myGenId) return;
    fullResponse += event.payload.token;
    onToken?.(event.payload.token);
  });
  geminiStore.currentUnlisten = unlisten;

  try {
    await invoke("stream_gemini_chat_v2", {
      apiKey: geminiStore.storedApiKey,
      model: geminiStore.currentModelId,
      isInitialTurn: false,
      imageBase64: null,
      imageMimeType: null,
      imagePath: isFirstTurnWithImage ? geminiStore.storedImagePath : null,
      imageDescription: imgDesc,
      userFirstMsg: geminiStore.userFirstMsg,
      historyLog: formatHistoryLog(),
      userMessage: lastUserMsg.text,
      channelId,
    });

    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;

    if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

    addToHistory("Assistant", fullResponse);

    return fullResponse;
  } catch (error) {
    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    throw error;
  }
};

export const editUserMessage = async (
  messageIndex: number,
  newText: string,
  allMessages: Array<{ role: string; text: string }>,
  modelId: string,
  onToken?: (token: string) => void,
  fallbackImagePath?: string,
): Promise<string> => {
  if (!geminiStore.storedApiKey) throw new Error("Gemini API Key not set");

  geminiStore.currentModelId = modelId;
  const myGenId = geminiStore.generationId;

  if (!geminiStore.storedImagePath && fallbackImagePath) {
    geminiStore.storedImagePath = fallbackImagePath;
  }

  const messagesBefore = allMessages.slice(0, messageIndex);

  const imgDesc =
    messagesBefore.find((m) => m.role === "model")?.text ||
    geminiStore.imageDescription ||
    "";
  geminiStore.imageDescription = imgDesc;

  const previousUserMsg = messagesBefore.find((m) => m.role === "user");

  if (!previousUserMsg) {
    geminiStore.userFirstMsg = newText;
  } else {
    geminiStore.userFirstMsg =
      messagesBefore.find((m) => m.role === "user")?.text || null;
  }

  geminiStore.conversationHistory = messagesBefore.map((m) => ({
    role: m.role === "user" ? "User" : "Assistant",
    content: m.text,
  }));

  if (geminiStore.conversationHistory.length > 6) {
    geminiStore.conversationHistory = geminiStore.conversationHistory.slice(-6);
  }

  const isFirstTurnWithImage = !previousUserMsg && geminiStore.storedImagePath;

  const channelId = `gemini-stream-${Date.now()}`;
  geminiStore.currentChannelId = channelId;
  let fullResponse = "";

  const unlisten = await listen<GeminiEvent>(channelId, (event) => {
    if (geminiStore.generationId !== myGenId) return;
    fullResponse += event.payload.token;
    onToken?.(event.payload.token);
  });
  geminiStore.currentUnlisten = unlisten;

  try {
    await invoke("stream_gemini_chat_v2", {
      apiKey: geminiStore.storedApiKey,
      model: geminiStore.currentModelId,
      isInitialTurn: false,
      imageBase64: null,
      imageMimeType: null,
      imagePath: isFirstTurnWithImage ? geminiStore.storedImagePath : null,
      imageDescription: imgDesc,
      userFirstMsg: geminiStore.userFirstMsg,
      historyLog: formatHistoryLog(),
      userMessage: newText,
      channelId,
    });

    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;

    if (geminiStore.generationId !== myGenId) throw new Error("CANCELLED");

    addToHistory("User", newText);
    addToHistory("Assistant", fullResponse);

    return fullResponse;
  } catch (error) {
    unlisten();
    if (geminiStore.currentUnlisten === unlisten)
      geminiStore.currentUnlisten = null;
    throw error;
  }
};
