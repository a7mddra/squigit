/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type {
  BrainEngineHandle,
  BrainLifecycleState,
  BrainParsedError,
  BrainSessionSnapshot,
  BrainStartupImage,
  ProviderContent,
  ProviderStreamEvent,
} from "./engine/types";

export {
  buildCommittedAssistantMessage,
  markPendingTurnTransportDone,
} from "./engine/controller";
export {
  getDefaultProgressText,
  getElapsedThoughtSeconds,
  getThoughtSecondsFromToolSteps,
  isUntitledThreadTitle,
  normalizeThreadTitle,
} from "./engine/pendingTurn";
export { runWithRetries, waitForRetryDelay } from "./engine/retryPolicy";
export {
  STREAM_PLAYBACK_INTERVAL_MS,
  STREAM_PRIME_DELAY_MS,
  advanceStreamCursorByWords,
  countRemainingStreamWords,
  getRenderableStreamingText,
  getStreamBatchSize,
} from "./engine/playback";
export { createToolEventHandler } from "./engine/toolEvents";

export {
  addToHistory,
  getImageBrief,
  getImageDescription,
  initializeBrainProvider,
  replaceLastAssistantHistory,
  resetBrainContext,
  setImageBrief,
  setImageDescription,
  setUserFirstMsg,
  setUserInfo,
} from "./session/context";
export { getBrainSessionSnapshot, restoreBrainSession } from "./session/snapshot";
export {
  brainSessionStore,
  type BrainSessionStoreState,
} from "./session/store";
export {
  buildContextWindow,
  getConversationSummary,
  maybeCompressHistory,
  setConversationSummary,
} from "./session/summarizer";
export {
  buildRichUserHistoryContent,
  extractAttachmentPaths,
  formatAttachmentReferences,
  normalizeMessageForHistory,
  stripAttachmentMentionsForHistory,
} from "./session/attachmentMemory";

export {
  cancelActiveProviderRequest as cancelActiveBrainRequest,
  compressConversationHistory,
  DEFAULT_PROVIDER_FALLBACK_MODEL_ID as DEFAULT_BRAIN_FALLBACK_MODEL_ID,
  generateProviderTitle as generateBrainTitle,
  getFriendlyProviderErrorMessage as getFriendlyBrainErrorMessage,
  getProviderHighDemandExhaustedMessage as getBrainHighDemandExhaustedMessage,
  getProviderHighDemandMessage as getBrainHighDemandMessage,
  isProviderHighDemandError as isBrainHighDemandError,
  parseProviderError as parseBrainError,
  persistConversationSummary,
  requestProviderQuickAnswer as requestBrainQuickAnswer,
  retryProviderMessage as retryBrainMessage,
  sendProviderMessage as sendBrainMessage,
  shouldFallbackToDefaultProviderModel as shouldFallbackToDefaultBrainModel,
  startProviderSession as startBrainSession,
  startProviderSessionStream as startBrainSessionStream,
} from "./provider";
