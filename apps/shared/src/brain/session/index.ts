/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export {
  addToHistory,
  getImageBrief,
  getImageDescription,
  initializeBrainProvider,
  popLastUserHistory,
  replaceLastAssistantHistory,
  resetBrainContext,
  setImageBrief,
  setImageDescription,
  setUserFirstMsg,
  setUserInfo,
} from "./context";
export { getBrainSessionSnapshot, restoreBrainSession } from "./snapshot";
export { brainSessionStore, type BrainSessionStoreState } from "./store";
export {
  buildContextWindow,
  getConversationSummary,
  maybeCompressHistory,
  setConversationSummary,
} from "./summarizer";
