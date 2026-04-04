/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { Message } from "@/features";
import { useChatState } from "./useChatState";
import { useGeminiEngine, useChatLifecycle } from "@/hooks";

export const useChat = ({
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

  const engine = useGeminiEngine({
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

  const lifecycle = useChatLifecycle({
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
    lastSentMessage: state.lastSentMessage,
    isSearching:
      (state.streamingToolSteps.length > 0 ||
        !!state.toolStatus ||
        state.streamingCitations.length > 0) &&
      (state.isLoading || !!state.retryingMessageId),
    isAnalyzing:
      !!startupImage &&
      (state.isLoading || !!state.retryingMessageId) &&
      !state.streamingText &&
      state.messages.length === 0,
    isGenerating:
      (state.isLoading || !!state.retryingMessageId) &&
      state.messages.length > 0,
    handleSend: engine.handleSend,
    handleRetrySend: engine.handleRetrySend,
    handleRetryMessage: engine.handleRetryMessage,
    handleUndoMessage: engine.handleUndoMessage,
    handleDescribeEdits: engine.handleDescribeEdits,
    handleStreamComplete: engine.handleStreamComplete,
    handleStopGeneration: engine.handleStopGeneration,
    handleAnswerNow: engine.handleAnswerNow,
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
