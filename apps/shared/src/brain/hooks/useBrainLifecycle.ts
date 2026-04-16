/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useEffect, useRef } from "react";
import type { Message } from "../engine/types";
import { restoreBrainSession } from "../session";

export const useBrainLifecycle = (config: {
  enabled: boolean;
  chatId: string | null;
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
    fromHistory?: boolean;
  } | null;
  prompt: string;
  apiKey: string;
  currentModel: string;
  onMissingApiKey?: () => void;
  state: any; // from useChatState
  engine: any; // from useBrainEngine
}) => {
  const {
    messages,
    setMessages,
    setIsLoading,
    setFirstResponseId,
    setLastSentMessage,
    setError,
    setIsStreaming,
    setPendingAssistantTurn,
  } = config.state;

  const sessionStartedForImageRef = useRef<string | null>(null);

  useEffect(() => {
    if (config.enabled && !config.startupImage?.fromHistory) {
      setIsLoading(true);
    }

    return () => {
      config.engine.cleanupAbortController();
    };
  }, [config.enabled, config.startupImage?.fromHistory]);

  useEffect(() => {
    if (
      config.enabled &&
      config.startupImage &&
      config.prompt &&
      !config.startupImage.fromHistory &&
      config.chatId
    ) {
      const imageKey =
        config.startupImage.path?.substring(0, 50) ?? config.chatId;
      if (sessionStartedForImageRef.current === imageKey) {
        return;
      }

      if (messages.length > 0) {
        return;
      }

      sessionStartedForImageRef.current = imageKey;

      console.log(
        "[useChatLifecycle] startSession triggered for imageKey:",
        imageKey,
        "chatId:",
        config.chatId,
      );

      if (config.apiKey) {
        config.engine.startSession(
          config.apiKey,
          config.currentModel,
          config.startupImage,
        );
      } else {
        if (config.onMissingApiKey) config.onMissingApiKey();
        setIsLoading(false);
      }
    }
  }, [
    config.apiKey,
    config.prompt,
    config.startupImage,
    config.currentModel,
    config.enabled,
    config.chatId,
    messages.length,
  ]);

  useEffect(() => {
    if (config.chatId === null) {
      setMessages([]);
      setFirstResponseId(null);
      setLastSentMessage(null);
      setError(null);
      setPendingAssistantTurn(null);
      sessionStartedForImageRef.current = null;
    }
  }, [config.chatId]);

  const getCurrentState = () => ({
    messages: config.state.messages,
    firstResponseId: config.state.firstResponseId,
  });

  const restoreState = async (
    state: {
      messages: Message[];
      firstResponseId: string | null;
    },
    image?: { path: string; mimeType: string; imageId: string },
    rollingSummary?: string | null,
    imageBrief?: string | null,
  ) => {
    setMessages(state.messages);
    setFirstResponseId(state.firstResponseId);
    setIsLoading(false);
    setIsStreaming(false);
    setPendingAssistantTurn(null);

    // Clear the current image ref so a session doesn't auto-start
    sessionStartedForImageRef.current = image?.path?.substring(0, 50) ?? null;

    if (state.messages.length > 0) {
      try {
        const firstMsg = state.messages[0];
        const savedHistory = state.messages.map((m) => ({
          role: m.role === "model" ? "Assistant" : "User",
          content: m.text,
        }));

        const firstUserMsg = state.messages.find((m) => m.role === "user");

        let imagePath = null;

        if (image) {
          imagePath = image.path;
        }

        restoreBrainSession(
          config.currentModel,
          firstMsg.text,
          firstUserMsg?.text || null,
          savedHistory,
          imagePath,
          imageBrief ?? firstMsg.text, // fallback for older sessions
          rollingSummary ?? null,
        );
      } catch (e) {
        console.error("Failed to restore brain session:", e);
      }
    }
  };

  return { getCurrentState, restoreState, sessionStartedForImageRef };
};
