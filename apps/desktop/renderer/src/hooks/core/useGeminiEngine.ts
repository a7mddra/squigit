/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef } from "react";
import { Message } from "@/features";
import {
  startNewChatStream,
  sendMessage as apiSendMessage,
  retryFromMessage as apiRetryFromMessage,
  editUserMessage as apiEditUserMessage,
  cancelCurrentRequest,
  replaceLastAssistantHistory,
  setImageDescription,
  ModelType,
} from "@/lib";

export const useGeminiEngine = (config: {
  apiKey: string;
  currentModel: string;
  setCurrentModel: (model: string) => void;
  chatId: string | null;
  chatTitle: string;
  startupImage: { path: string; mimeType: string; imageId: string } | null;
  onMissingApiKey?: () => void;
  onMessage?: (message: Message, chatId: string) => void;
  onOverwriteMessages?: (messages: Message[]) => void;
  onTitleGenerated?: (title: string) => void;
  generateTitle?: (text: string) => Promise<string>;
  state: any; // from useChatState
}) => {
  const {
    messages,
    setMessages,
    setIsLoading,
    setIsStreaming,
    setIsAiTyping,
    setStreamingText,
    setFirstResponseId,
    setRetryingMessageId,
    setLastSentMessage,
    resetInitialUi,
    appendErrorMessage,
    streamingText,
    firstResponseId,
    lastSentMessage,
  } = config.state;

  const sessionChatIdRef = useRef<string | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const isRequestCancelledRef = useRef(false);
  const preRetryMessagesRef = useRef<Message[]>([]);

  const cleanupAbortController = () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
  };

  const startSession = async (
    key: string,
    modelId: string,
    imgData: {
      path: string;
      mimeType: string;
      imageId: string;
      fromHistory?: boolean;
    } | null,
    isRetry = false,
  ) => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    abortControllerRef.current = new AbortController();
    const signal = abortControllerRef.current.signal;

    sessionChatIdRef.current = config.chatId;
    isRequestCancelledRef.current = false;

    setIsLoading(true);

    if (!key) {
      if (config.onMissingApiKey) config.onMissingApiKey();
      setIsLoading(false);
      return;
    }

    if (!isRetry) {
      resetInitialUi();
      setMessages([]);
      setFirstResponseId(null);
      setLastSentMessage(null);
      await new Promise((resolve) => setTimeout(resolve, 3000));
      if (signal.aborted) {
        setIsLoading(false);
        return;
      }
    }

    if (!imgData) {
      setIsLoading(false);
      return;
    }

    setIsStreaming(true);
    setIsAiTyping(true);

    try {
      let fullResponse = "";
      const responseId = Date.now().toString();
      setFirstResponseId(responseId);

      if (signal.aborted) {
        setIsLoading(false);
        return;
      }

      let hasTriggeredTitle = false;

      console.log(
        "[useGeminiEngine] Calling startNewChatStream with model:",
        modelId,
      );
      await startNewChatStream(modelId, imgData.path, (token: string) => {
        if (signal.aborted) return;
        fullResponse += token;

        if (
          !isRetry &&
          !imgData.fromHistory &&
          !hasTriggeredTitle &&
          fullResponse.length > 50
        ) {
          console.log(
            "[useGeminiEngine] Triggering title generation due to stream length > 50",
          );
          hasTriggeredTitle = true;
          if (config.generateTitle && config.onTitleGenerated) {
            config
              .generateTitle(fullResponse)
              .then((title) => {
                console.log("[useGeminiEngine] Title generated:", title);
                if (!signal.aborted) config.onTitleGenerated?.(title);
              })
              .catch(console.error);
          }
        }
      });
      console.log("[useGeminiEngine] startNewChatStream finished!");

      if (!isRetry && !imgData.fromHistory && !hasTriggeredTitle) {
        hasTriggeredTitle = true;
        if (
          config.generateTitle &&
          config.onTitleGenerated &&
          fullResponse.length > 0
        ) {
          config
            .generateTitle(fullResponse)
            .then((title) => {
              if (!signal.aborted) config.onTitleGenerated?.(title);
            })
            .catch(console.error);
        }
      }

      if (signal.aborted) {
        setIsLoading(false);
        return;
      }

      setStreamingText(fullResponse);

      const targetChatId = sessionChatIdRef.current;

      if (config.onMessage && targetChatId) {
        config.onMessage(
          {
            id: responseId,
            role: "model",
            text: fullResponse,
            timestamp: Date.now(),
          },
          targetChatId,
        );
      }

      setIsStreaming(false);
      setIsLoading(false);
    } catch (apiError: any) {
      if (
        signal.aborted ||
        apiError?.message === "CANCELLED" ||
        isRequestCancelledRef.current
      ) {
        setIsLoading(false);
        setIsAiTyping(false);
        return;
      }

      console.error(apiError);
      if (
        !isRetry &&
        (apiError.message?.includes("429") || apiError.message?.includes("503"))
      ) {
        if (config.currentModel !== ModelType.GEMINI_3_1_FLASH) {
          console.log("Model failed, trying lite version...");
          config.setCurrentModel(ModelType.GEMINI_3_1_FLASH);
          return;
        }
      }

      setIsAiTyping(false);
      cancelCurrentRequest();
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }
      let errorMsg = "An error occurred while connecting to Gemini.";
      if (apiError.message?.includes("429"))
        errorMsg = "Quota limit reached or server busy.";
      else if (apiError.message?.includes("503"))
        errorMsg = "Service temporarily unavailable.";
      else if (apiError.message) errorMsg = apiError.message;

      appendErrorMessage(errorMsg, sessionChatIdRef.current || config.chatId);
    } finally {
      if (abortControllerRef.current?.signal === signal) {
        abortControllerRef.current = null;
      }
    }
  };

  const handleDescribeEdits = async (_editDescription: string) => {
    if (!config.apiKey || !config.startupImage) {
      if (!config.apiKey && config.onMissingApiKey) {
        config.onMissingApiKey();
        return;
      }
      appendErrorMessage(
        "Cannot start session. Missing required data.",
        config.chatId,
      );
      return;
    }
    const targetChatId = config.chatId;
    isRequestCancelledRef.current = false;

    setIsLoading(true);
    resetInitialUi();
    setMessages([]);
    setFirstResponseId(null);
    setLastSentMessage(null);

    setIsStreaming(true);
    setIsAiTyping(true);

    try {
      let fullResponse = "";
      const responseId = Date.now().toString();
      setFirstResponseId(responseId);

      await startNewChatStream(
        config.currentModel,
        config.startupImage.path,
        (token: string) => {
          fullResponse += token;
        },
      );

      setStreamingText(fullResponse);

      if (config.onMessage && targetChatId) {
        config.onMessage(
          {
            id: responseId,
            role: "model",
            text: fullResponse,
            timestamp: Date.now(),
          },
          targetChatId,
        );
      }

      setIsStreaming(false);
      setIsLoading(false);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsAiTyping(false);
        return;
      }
      console.error(apiError);
      let errorMsg = "An error occurred while connecting to Gemini.";
      if (apiError.message?.includes("429"))
        errorMsg = "Quota limit reached or server busy.";
      else if (apiError.message?.includes("503"))
        errorMsg = "Service temporarily unavailable.";
      else if (apiError.message) errorMsg = apiError.message;

      appendErrorMessage(errorMsg, targetChatId);
    }
  };

  const handleRetrySend = async () => {
    if (!lastSentMessage) return;
    setIsLoading(true);
    isRequestCancelledRef.current = false;
    setMessages((prev: Message[]) => [...prev, lastSentMessage]);
    const targetChatId = config.chatId;

    try {
      const responseText = await apiSendMessage(lastSentMessage.text);
      setIsAiTyping(true);
      const botMsg: Message = {
        id: (Date.now() + 1).toString(),
        role: "model",
        text: responseText,
        timestamp: Date.now(),
      };
      setMessages((prev: Message[]) => [...prev, botMsg]);
      if (config.onMessage && targetChatId)
        config.onMessage(botMsg, targetChatId);
      setLastSentMessage(null);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsAiTyping(false);
        return;
      }
      appendErrorMessage(
        "An error occurred while sending the message. " +
          (apiError.message || ""),
        targetChatId,
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleSend = async (userText: string, modelId?: string) => {
    if (!userText.trim() || config.state.isLoading) return;
    const targetChatId = config.chatId;

    if (streamingText && firstResponseId) {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }

      const botMsg: Message = {
        id: firstResponseId,
        role: "model",
        text: streamingText,
        timestamp: Date.now(),
      };
      setMessages((prev: Message[]) => {
        const idx = prev.findIndex((m) => m.id === botMsg.id);
        if (idx !== -1) {
          const newMsgs = [...prev];
          newMsgs[idx] = botMsg;
          return newMsgs;
        }
        return [...prev, botMsg];
      });

      if (config.state.isStreaming && config.onMessage && targetChatId) {
        config.onMessage(botMsg, targetChatId);
      }

      setStreamingText("");
      setFirstResponseId(null);
    }

    const userMsg: Message = {
      id: Date.now().toString(),
      role: "user",
      text: userText,
      timestamp: Date.now(),
    };

    setLastSentMessage(userMsg);
    setMessages((prev: Message[]) => [...prev, userMsg]);
    if (config.onMessage && targetChatId)
      config.onMessage(userMsg, targetChatId);
    setIsLoading(true);
    isRequestCancelledRef.current = false;

    try {
      const responseText = await apiSendMessage(userText, modelId);
      setIsAiTyping(true);
      const botMsg: Message = {
        id: (Date.now() + 1).toString(),
        role: "model",
        text: responseText,
        timestamp: Date.now(),
      };
      setMessages((prev: Message[]) => [...prev, botMsg]);
      if (config.onMessage && targetChatId)
        config.onMessage(botMsg, targetChatId);
      setLastSentMessage(null);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsAiTyping(false);
        return;
      }
      appendErrorMessage(
        "An error occurred while sending the message. " +
          (apiError.message || ""),
        targetChatId,
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleRetryMessage = async (messageId: string, modelId?: string) => {
    const msgIndex = messages.findIndex((m: Message) => m.id === messageId);
    if (msgIndex === -1) return;

    const truncatedMessages = messages.slice(0, msgIndex);
    const retryModelId = modelId || config.currentModel;

    preRetryMessagesRef.current = [...messages];
    setRetryingMessageId(messageId);
    isRequestCancelledRef.current = false;

    const newResponseId = Date.now().toString();

    let fallbackImagePath: string | undefined;
    if (config.startupImage) {
      fallbackImagePath = config.startupImage.path;
    }

    try {
      const responseText = await apiRetryFromMessage(
        msgIndex,
        messages,
        retryModelId,
        undefined,
        fallbackImagePath,
      );

      if (
        msgIndex === 0 &&
        (!config.chatTitle || config.chatTitle === "New Chat") &&
        responseText.length > 50 &&
        config.generateTitle &&
        config.onTitleGenerated
      ) {
        config
          .generateTitle(responseText)
          .then((title) => config.onTitleGenerated?.(title))
          .catch(console.error);
      }

      setRetryingMessageId(null);
      setMessages(truncatedMessages);
      setIsStreaming(true);
      setIsAiTyping(true);
      setFirstResponseId(newResponseId);
      setStreamingText(responseText);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsAiTyping(false);
        return;
      }
      console.error("Retry failed:", apiError);

      const errorMsg =
        "An error occurred while regenerating the response. " +
        (apiError.message || "");

      const errorBubble: Message = {
        id: Date.now().toString(),
        role: "model",
        text: errorMsg,
        timestamp: Date.now(),
        stopped: true,
      };

      const newMessages = [...truncatedMessages, errorBubble];
      setMessages(newMessages);

      if (config.onOverwriteMessages) {
        config.onOverwriteMessages(newMessages);
      }

      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
      setStreamingText("");
      setFirstResponseId(null);
      setRetryingMessageId(null);
    }
  };

  const handleEditMessage = async (
    messageId: string,
    newText: string,
    modelId?: string,
  ) => {
    const msgIndex = messages.findIndex((m: Message) => m.id === messageId);
    if (msgIndex === -1) return;

    const truncatedMessages = messages.slice(0, msgIndex);
    const retryModelId = modelId || config.currentModel;

    preRetryMessagesRef.current = [...messages];
    const editedUserMsg: Message = {
      ...messages[msgIndex],
      text: newText,
    };
    setMessages([...truncatedMessages, editedUserMsg]);
    setIsLoading(true);
    isRequestCancelledRef.current = false;

    const newResponseId = Date.now().toString();

    let fallbackImagePath: string | undefined;
    if (config.startupImage) {
      fallbackImagePath = config.startupImage.path;
    }

    try {
      const responseText = await apiEditUserMessage(
        msgIndex,
        newText,
        messages,
        retryModelId,
        undefined,
        fallbackImagePath,
      );

      setMessages([...truncatedMessages, editedUserMsg]);
      setIsLoading(false);
      setIsStreaming(true);
      setIsAiTyping(true);
      setFirstResponseId(newResponseId);
      setStreamingText(responseText);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsLoading(false);
        setIsStreaming(false);
        setIsAiTyping(false);
        setStreamingText("");
        setFirstResponseId(null);
        return;
      }
      console.error("Edit failed:", apiError);

      const errorMsg =
        "An error occurred while generating the response. " +
        (apiError.message || "");

      const errorBubble: Message = {
        id: Date.now().toString(),
        role: "model",
        text: errorMsg,
        timestamp: Date.now(),
        stopped: true,
      };

      const newMessages = [...truncatedMessages, editedUserMsg, errorBubble];
      setMessages(newMessages);

      if (config.onOverwriteMessages) {
        config.onOverwriteMessages(newMessages);
      }

      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
      setStreamingText("");
      setFirstResponseId(null);
    }
  };

  const handleStopGeneration = (truncatedText?: string) => {
    isRequestCancelledRef.current = true;
    if (typeof truncatedText === "string") {
      cancelCurrentRequest();
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }
      replaceLastAssistantHistory(truncatedText);
      if (messages.length === 0) {
        setImageDescription(truncatedText);
      }

      if (streamingText && firstResponseId) {
        const botMsg: Message = {
          id: firstResponseId,
          role: "model",
          text: truncatedText,
          timestamp: Date.now(),
          stopped: true,
          alreadyStreamed: true,
        };
        setMessages((prev: Message[]) => {
          const idx = prev.findIndex((m) => m.id === botMsg.id);
          if (idx !== -1) {
            const newMsgs = [...prev];
            newMsgs[idx] = botMsg;
            config.onOverwriteMessages?.(newMsgs);
            return newMsgs;
          }
          const newMsgs = [...prev, botMsg];
          config.onOverwriteMessages?.(newMsgs);
          return newMsgs;
        });
        setStreamingText("");
        setFirstResponseId(null);
        setIsLoading(false);
        setIsStreaming(false);
        setIsAiTyping(false);
        return;
      }

      setMessages((prev: Message[]) => {
        const updated = [...prev];
        for (let i = updated.length - 1; i >= 0; i--) {
          if (updated[i].role === "model") {
            updated[i] = {
              ...updated[i],
              text: truncatedText,
              stopped: true,
              alreadyStreamed: true,
            };
            break;
          }
        }

        config.onOverwriteMessages?.(updated);
        return updated;
      });
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
      return;
    }

    if (config.state.retryingMessageId) {
      cancelCurrentRequest();
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }
      const oldMessages = preRetryMessagesRef.current;
      setMessages(oldMessages);
      config.onOverwriteMessages?.(oldMessages);
      setRetryingMessageId(null);
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
      setStreamingText("");
      setFirstResponseId(null);
      return;
    }

    cancelCurrentRequest();
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }

    const stoppedMsg: Message = {
      id: Date.now().toString(),
      role: "model",
      text: "You stopped this response.",
      timestamp: Date.now(),
      stopped: true,
    };
    setMessages((prev: Message[]) => [...prev, stoppedMsg]);
    const targetChatId = sessionChatIdRef.current;
    if (config.onMessage && targetChatId) {
      config.onMessage(stoppedMsg, targetChatId);
    }

    setIsLoading(false);
    setIsStreaming(false);
    setIsAiTyping(false);
    setStreamingText("");
    setFirstResponseId(null);
  };

  const handleStreamComplete = () => {
    if (streamingText && firstResponseId) {
      const botMsg: Message = {
        id: firstResponseId,
        role: "model",
        text: streamingText,
        timestamp: Date.now(),
        alreadyStreamed: true,
      };
      setMessages((prev: Message[]) => {
        const idx = prev.findIndex((m) => m.id === botMsg.id);
        if (idx !== -1) {
          const newMsgs = [...prev];
          newMsgs[idx] = botMsg;
          config.onOverwriteMessages?.(newMsgs);
          return newMsgs;
        }
        const newMsgs = [...prev, botMsg];
        config.onOverwriteMessages?.(newMsgs);
        return newMsgs;
      });
      setStreamingText("");
      setFirstResponseId(null);
    }
    setIsStreaming(false);
    setIsAiTyping(false);
  };

  return {
    startSession,
    handleSend,
    handleRetrySend,
    handleRetryMessage,
    handleEditMessage,
    handleDescribeEdits,
    handleStopGeneration,
    handleStreamComplete,
    cleanupAbortController,
  };
};
