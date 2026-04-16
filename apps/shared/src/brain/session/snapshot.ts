/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { BrainConversationEntry, BrainSessionSnapshot } from "../engine/types";
import { brainSessionStore } from "./store";

export function restoreBrainSession(
  modelId: string,
  savedImageDescription: string,
  savedUserFirstMsg: string | null,
  savedHistory: BrainConversationEntry[],
  savedImagePath: string | null,
  savedImageBrief: string | null = null,
  savedSummary: string | null = null,
) {
  brainSessionStore.currentModelId = modelId;
  brainSessionStore.imageDescription = savedImageDescription;
  brainSessionStore.userFirstMsg = savedUserFirstMsg;
  brainSessionStore.conversationHistory = savedHistory;
  brainSessionStore.storedImagePath = savedImagePath;
  brainSessionStore.imageBrief = savedImageBrief;
  brainSessionStore.conversationSummary = savedSummary;
}

export function getBrainSessionSnapshot(): BrainSessionSnapshot {
  return {
    imageDescription: brainSessionStore.imageDescription,
    userFirstMsg: brainSessionStore.userFirstMsg,
    conversationHistory: [...brainSessionStore.conversationHistory],
    imageBrief: brainSessionStore.imageBrief,
    conversationSummary: brainSessionStore.conversationSummary,
    storedImagePath: brainSessionStore.storedImagePath,
    currentModelId: brainSessionStore.currentModelId,
  };
}
