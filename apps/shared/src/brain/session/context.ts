/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { brainSessionStore } from "./store";

export const initializeBrainProvider = (apiKey: string) => {
  brainSessionStore.storedApiKey = apiKey;
};

export const resetBrainContext = () => {
  brainSessionStore.imageDescription = null;
  brainSessionStore.userFirstMsg = null;
  brainSessionStore.conversationHistory = [];
  brainSessionStore.storedImagePath = null;
  brainSessionStore.imageBrief = null;
  brainSessionStore.userName = null;
  brainSessionStore.userEmail = null;
  brainSessionStore.userInstruction = null;
  brainSessionStore.conversationSummary = null;
};

export const setImageDescription = (description: string) => {
  brainSessionStore.imageDescription = description;
};

export const getImageDescription = () => brainSessionStore.imageDescription;

export const setUserInfo = (name?: string, email?: string, instruction?: string) => {
  if (name) brainSessionStore.userName = name;
  if (email) brainSessionStore.userEmail = email;
  if (instruction) brainSessionStore.userInstruction = instruction;
};

export const setImageBrief = (brief: string) => {
  brainSessionStore.imageBrief = brief;
};

export const getImageBrief = () => brainSessionStore.imageBrief;

export const setUserFirstMsg = (msg: string) => {
  if (!brainSessionStore.userFirstMsg && msg) {
    brainSessionStore.userFirstMsg = msg;
  }
};

export const addToHistory = (role: "User" | "Assistant", content: string) => {
  brainSessionStore.conversationHistory.push({ role, content });
  // No more slice(-6) — summarize.ts handles windowing via maybeCompressHistory()
};

export const popLastUserHistory = () => {
  if (brainSessionStore.conversationHistory.length > 0) {
    const last = brainSessionStore.conversationHistory[brainSessionStore.conversationHistory.length - 1];
    if (last.role === "User") {
      brainSessionStore.conversationHistory.pop();
    }
  }
};

export const replaceLastAssistantHistory = (content: string) => {
  for (let i = brainSessionStore.conversationHistory.length - 1; i >= 0; i--) {
    if (brainSessionStore.conversationHistory[i].role === "Assistant") {
      brainSessionStore.conversationHistory[i] = { role: "Assistant", content };
      return;
    }
  }

  addToHistory("Assistant", content);
};

export const formatHistoryLog = (): string => {
  if (brainSessionStore.conversationHistory.length === 0)
    return "(No previous messages)";

  return brainSessionStore.conversationHistory
    .map(({ role, content }) => `**${role}**: ${content}`)
    .join("\n\n");
};
