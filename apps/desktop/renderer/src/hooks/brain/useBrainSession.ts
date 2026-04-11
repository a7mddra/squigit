/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { Message } from "@/features";
import { useChatState } from "@/features/chat/hooks/useChatState";
import { useBrainEngine } from "./useBrainEngine";
import { useBrainLifecycle } from "./useBrainLifecycle";

export const useBrainSession = ({
  apiKey,
  currentModel,
  startupImage,
  prompt,
  setCurrentModel,
  enabled,
  onMessage,
  onOverwriteMessages,
  chatId,
  chatTitle,
  onMissingApiKey,
  onTitleGenerated,
  generateTitle,
  userName,
  userEmail,
}: {
  apiKey: string;
  currentModel: string;
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
    fromHistory?: boolean;
  } | null;
  prompt: string;
  setCurrentModel: (model: string) => void;
  enabled: boolean;
  onMessage?: (message: Message, chatId: string) => void;
  onOverwriteMessages?: (messages: Message[]) => void;
  chatId: string | null;
  chatTitle: string;
  onMissingApiKey?: () => void;
  onTitleGenerated?: (title: string) => void;
  generateTitle?: (text: string) => Promise<string>;
  userName?: string;
  userEmail?: string;
}) => {
  const state = useChatState(enabled);

  const engine = useBrainEngine({
    apiKey,
    currentModel,
    setCurrentModel,
    chatId,
    chatTitle,
    startupImage,
    onMissingApiKey,
    onMessage,
    onOverwriteMessages,
    onTitleGenerated,
    generateTitle,
    state,
    userName,
    userEmail,
    userInstruction: prompt,
  });

  const lifecycle = useBrainLifecycle({
    enabled,
    chatId,
    startupImage,
    prompt,
    apiKey,
    currentModel,
    onMissingApiKey,
    state,
    engine,
  });

  return {
    messages: state.messages,
    isLoading: state.isLoading,
    error: state.error,
    clearError: state.clearError,
    isStreaming: state.isStreaming,
    isAiTyping: state.isAiTyping,
    setIsAiTyping: state.setIsAiTyping,
    retryingMessageId: state.retryingMessageId,
    streamingText: state.streamingText,
    toolStatus: state.toolStatus,
    streamingToolSteps: state.streamingToolSteps,
    streamingCitations: state.streamingCitations,
    pendingAssistantTurn: state.pendingAssistantTurn,
    lastSentMessage: state.lastSentMessage,
    isSearching:
      !!state.pendingAssistantTurn &&
      (state.pendingAssistantTurn.toolSteps.length > 0 ||
        state.pendingAssistantTurn.pendingCitations.length > 0 ||
        /search|source|web/i.test(state.pendingAssistantTurn.progressText)),
    isAnalyzing:
      !!startupImage &&
      !!state.pendingAssistantTurn &&
      state.pendingAssistantTurn.requestKind === "initial" &&
      state.pendingAssistantTurn.phase === "thinking",
    isGenerating: !!state.pendingAssistantTurn,
    handleSend: engine.handleSend,
    handleRetrySend: engine.handleRetrySend,
    handleRetryMessage: engine.handleRetryMessage,
    handleUndoMessage: engine.handleUndoMessage,
    handleDescribeEdits: engine.handleDescribeEdits,
    handleStreamComplete: engine.handleStreamComplete,
    handleStopGeneration: engine.handleStopGeneration,
    handleQuickAnswer: engine.handleQuickAnswer,
    startSession: (
      key: string,
      modelId: string,
      imgData: {
        path: string;
        mimeType: string;
        imageId: string;
        fromHistory?: boolean;
      } | null,
      isRetry = false,
    ) => engine.startSession(key, modelId, imgData, isRetry),
    getCurrentState: lifecycle.getCurrentState,
    restoreState: lifecycle.restoreState,
    appendErrorMessage: state.appendErrorMessage,
  };
};
