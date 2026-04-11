/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type { BrainParsedError } from "./gemini/errors";

export {
  cancelCurrentRequest as cancelActiveProviderRequest,
  generateProviderTitle,
  getFriendlyProviderErrorMessage,
  getProviderHighDemandExhaustedMessage,
  getProviderHighDemandMessage,
  isProviderHighDemandError,
  parseProviderError,
  quickAnswerCurrentRequest as requestProviderQuickAnswer,
  retryFromMessage as retryProviderMessage,
  sendMessage as sendProviderMessage,
  startNewThread as startProviderSession,
  startNewThreadStream as startProviderSessionStream,
  shouldFallbackToGeminiDefaultModel as shouldFallbackToDefaultProviderModel,
  GEMINI_FALLBACK_MODEL_ID as DEFAULT_PROVIDER_FALLBACK_MODEL_ID,
} from "./gemini";

export {
  compressGeminiConversation as compressConversationHistory,
  persistRollingSummary as persistConversationSummary,
} from "./gemini/commands";
