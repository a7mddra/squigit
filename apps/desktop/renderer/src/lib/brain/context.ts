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
  geminiStore.imageBrief = null;
  geminiStore.userName = null;
  geminiStore.userEmail = null;
  geminiStore.userInstruction = null;
  geminiStore.conversationSummary = null;
};

export const setImageDescription = (description: string) => {
  geminiStore.imageDescription = description;
};

export const getImageDescription = () => geminiStore.imageDescription;

export const setUserInfo = (name?: string, email?: string, instruction?: string) => {
  if (name) geminiStore.userName = name;
  if (email) geminiStore.userEmail = email;
  if (instruction) geminiStore.userInstruction = instruction;
};

export const setImageBrief = (brief: string) => {
  geminiStore.imageBrief = brief;
};

export const getImageBrief = () => geminiStore.imageBrief;

export const setUserFirstMsg = (msg: string) => {
  if (!geminiStore.userFirstMsg && msg) {
    geminiStore.userFirstMsg = msg;
  }
};

export const addToHistory = (role: "User" | "Assistant", content: string) => {
  geminiStore.conversationHistory.push({ role, content });
  // No more slice(-6) — summarize.ts handles windowing via maybeCompressHistory()
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
  savedImageBrief: string | null = null,
  savedSummary: string | null = null,
) => {
  geminiStore.currentModelId = modelId;
  geminiStore.imageDescription = savedImageDescription;
  geminiStore.userFirstMsg = savedUserFirstMsg;
  geminiStore.conversationHistory = savedHistory;
  geminiStore.storedImagePath = savedImagePath;
  geminiStore.imageBrief = savedImageBrief;
  geminiStore.conversationSummary = savedSummary;
};

export const getSessionState = () => ({
  imageDescription: geminiStore.imageDescription,
  userFirstMsg: geminiStore.userFirstMsg,
  conversationHistory: [...geminiStore.conversationHistory],
  imageBrief: geminiStore.imageBrief,
});
