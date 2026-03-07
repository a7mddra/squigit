/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { geminiStore } from "./store";

export const initializeGemini = (apiKey: string) => {
  geminiStore.storedApiKey = apiKey;
};

export const resetBrainContext = () => {
  geminiStore.imageDescription = null;
  geminiStore.userFirstMsg = null;
  geminiStore.conversationHistory = [];
  geminiStore.storedImagePath = null;
};

export const setImageDescription = (description: string) => {
  geminiStore.imageDescription = description;
};

export const getImageDescription = () => geminiStore.imageDescription;

export const setUserFirstMsg = (msg: string) => {
  if (!geminiStore.userFirstMsg && msg) {
    geminiStore.userFirstMsg = msg;
  }
};

export const addToHistory = (role: "User" | "Assistant", content: string) => {
  geminiStore.conversationHistory.push({ role, content });

  if (geminiStore.conversationHistory.length > 6) {
    geminiStore.conversationHistory = geminiStore.conversationHistory.slice(-6);
  }
};

export const replaceLastAssistantHistory = (content: string) => {
  for (let i = geminiStore.conversationHistory.length - 1; i >= 0; i--) {
    if (geminiStore.conversationHistory[i].role === "Assistant") {
      geminiStore.conversationHistory[i] = { role: "Assistant", content };
      return;
    }
  }

  addToHistory("Assistant", content);
};

export const formatHistoryLog = (): string => {
  if (geminiStore.conversationHistory.length === 0)
    return "(No previous messages)";

  return geminiStore.conversationHistory
    .map(({ role, content }) => `**${role}**: ${content}`)
    .join("\n\n");
};

export const restoreSession = (
  modelId: string,
  savedImageDescription: string,
  savedUserFirstMsg: string | null,
  savedHistory: Array<{ role: string; content: string }>,
  savedImagePath: string | null,
) => {
  geminiStore.currentModelId = modelId;
  geminiStore.imageDescription = savedImageDescription;
  geminiStore.userFirstMsg = savedUserFirstMsg;
  geminiStore.conversationHistory = savedHistory.slice(-6);
  geminiStore.storedImagePath = savedImagePath;
};

export const getSessionState = () => ({
  imageDescription: geminiStore.imageDescription,
  userFirstMsg: geminiStore.userFirstMsg,
  conversationHistory: [...geminiStore.conversationHistory],
});
