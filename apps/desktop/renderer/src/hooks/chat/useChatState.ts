/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useRef, useState } from "react";
import {
  Message,
  ToolStep,
  Citation,
  PendingAssistantTurn,
} from "@/features/chat";
import { appendChatMessage } from "@/core";

export const useChatState = (enabled: boolean) => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isAiTyping, setIsAiTyping] = useState(false);
  const [retryingMessageId, setRetryingMessageId] = useState<string | null>(
    null,
  );
  const [firstResponseId, setFirstResponseId] = useState<string | null>(null);
  const [lastSentMessage, setLastSentMessage] = useState<Message | null>(null);
  const [toolStatus, setToolStatus] = useState<string | null>(null);
  const [streamingToolSteps, setStreamingToolSteps] = useState<ToolStep[]>([]);
  const [streamingCitations, setStreamingCitations] = useState<Citation[]>([]);
  const [pendingAssistantTurnState, setPendingAssistantTurnState] =
    useState<PendingAssistantTurn | null>(null);
  const pendingAssistantTurnRef = useRef<PendingAssistantTurn | null>(null);

  const setPendingAssistantTurn = useCallback(
    (
      value:
        | PendingAssistantTurn
        | null
        | ((
            previous: PendingAssistantTurn | null,
          ) => PendingAssistantTurn | null),
    ) => {
      setPendingAssistantTurnState((previous) => {
        const next =
          typeof value === "function"
            ? (
                value as (
                  previous: PendingAssistantTurn | null,
                ) => PendingAssistantTurn | null
              )(previous)
            : value;
        pendingAssistantTurnRef.current = next;
        return next;
      });
    },
    [],
  );

  const clearError = () => setError(null);

  const resetInitialUi = () => {
    setError(null);
    setToolStatus(null);
    setStreamingToolSteps([]);
    setStreamingCitations([]);
    setPendingAssistantTurn(null);
  };

  const appendErrorMessage = (
    errorMsg: string,
    targetChatId?: string | null,
  ) => {
    const errorBubble: Message = {
      id: Date.now().toString(),
      role: "model",
      text: errorMsg,
      timestamp: Date.now(),
      stopped: true,
    };

    setMessages((prev) => [...prev, errorBubble]);

    if (targetChatId) {
      appendChatMessage(targetChatId, "assistant", errorMsg).catch(
        console.error,
      );
    }

    setIsLoading(false);
    setIsStreaming(false);
    setIsAiTyping(false);
    setFirstResponseId(null);
    setRetryingMessageId(null);
    setToolStatus(null);
    setStreamingToolSteps([]);
    setStreamingCitations([]);
    setPendingAssistantTurn(null);
  };

  return {
    messages,
    setMessages,
    isLoading,
    setIsLoading,
    error,
    setError,
    clearError,
    isStreaming,
    setIsStreaming,
    isAiTyping,
    setIsAiTyping,
    retryingMessageId,
    setRetryingMessageId,
    firstResponseId,
    setFirstResponseId,
    lastSentMessage,
    setLastSentMessage,
    toolStatus,
    setToolStatus,
    streamingToolSteps,
    setStreamingToolSteps,
    streamingCitations,
    setStreamingCitations,
    pendingAssistantTurn: pendingAssistantTurnState,
    pendingAssistantTurnRef,
    setPendingAssistantTurn,
    resetInitialUi,
    appendErrorMessage,
  };
};
