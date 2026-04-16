/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { DEFAULT_MODEL_ID } from "../../config/models-config";

export interface BrainSessionStoreState {
  currentAbortController: AbortController | null;
  currentUnlisten: (() => void) | null;
  currentChannelId: string | null;
  generationId: number;
  storedApiKey: string | null;
  currentModelId: string;
  imageDescription: string | null;
  userFirstMsg: string | null;
  conversationHistory: Array<{ role: string; content: string }>;
  storedImagePath: string | null;
  imageBrief: string | null;
  userName: string | null;
  userEmail: string | null;
  userInstruction: string | null;
  conversationSummary: string | null;
}

export const brainSessionStore: BrainSessionStoreState = {
  currentAbortController: null as AbortController | null,
  currentUnlisten: null as (() => void) | null,
  currentChannelId: null as string | null,
  generationId: 0,
  storedApiKey: null as string | null,
  currentModelId: DEFAULT_MODEL_ID,
  imageDescription: null as string | null,
  userFirstMsg: null as string | null,
  conversationHistory: [] as Array<{ role: string; content: string }>,
  storedImagePath: null as string | null,
  imageBrief: null as string | null,
  userName: null as string | null,
  userEmail: null as string | null,
  userInstruction: null as string | null,
  conversationSummary: null as string | null,
};
