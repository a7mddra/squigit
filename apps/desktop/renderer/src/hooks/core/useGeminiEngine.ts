/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef } from "react";
import { Citation, Message, ToolStep } from "@/features";
import {
  startNewThreadStream,
  sendMessage as apiSendMessage,
  retryFromMessage as apiRetryFromMessage,
  cancelCurrentRequest,
  answerNowCurrentRequest,
  replaceLastAssistantHistory,
  restoreSession as apiRestoreSession,
  getImageDescription,
  setImageDescription,
  API_STATUS_TEXT,
  mapToolStatusText,
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
  userName?: string;
  userEmail?: string;
  userInstruction?: string;
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
    streamingToolSteps,
    streamingCitations,
    setToolStatus,
    setStreamingToolSteps,
    setStreamingCitations,
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

  const resetToolStreamingState = () => {
    setToolStatus(null);
    setStreamingToolSteps([]);
    setStreamingCitations([]);
  };

  const getThoughtSecondsFromToolSteps = (steps: ToolStep[]): number | undefined => {
    const seconds = steps.reduce((sum, step) => {
      if (
        typeof step.startedAtMs === "number" &&
        typeof step.endedAtMs === "number" &&
        step.endedAtMs >= step.startedAtMs
      ) {
        return sum + Math.max(1, Math.round((step.endedAtMs - step.startedAtMs) / 1000));
      }
      const match = step.message?.trim().match(/^Thought for (\d+)s$/i);
      if (!match) return sum;
      const parsed = Number.parseInt(match[1], 10);
      if (!Number.isFinite(parsed) || parsed <= 0) return sum;
      return sum + parsed;
    }, 0);

    return seconds > 0 ? seconds : undefined;
  };

  const getElapsedThoughtSeconds = (startedAtMs: number): number => {
    return Math.max(1, Math.round((Date.now() - startedAtMs) / 1000));
  };

  const createToolEventHandler = (onResetText?: () => void) => {
    let steps: ToolStep[] = [];
    let citations: Citation[] = [];

    const mergeCitations = (incoming: Citation[]) => {
      if (!incoming.length) return;
      const byUrl = new Map<string, Citation>();
      for (const c of citations) byUrl.set(c.url, c);
      for (const c of incoming) byUrl.set(c.url, c);
      citations = Array.from(byUrl.values());
    };

    const parseCitationsFromResult = (result: unknown): Citation[] => {
      if (!result || typeof result !== "object") return [];
      const payload = result as { sources?: unknown };
      if (!Array.isArray(payload.sources)) return [];
      return payload.sources
        .map((s) => {
          const src = s as {
            title?: string;
            url?: string;
            summary?: string;
            favicon?: string;
          };
          if (!src.url || !src.title) return null;
          return {
            title: String(src.title),
            url: String(src.url),
            summary: String(src.summary || ""),
            favicon:
              typeof src.favicon === "string" && src.favicon.trim().length > 0
                ? src.favicon
                : undefined,
          } as Citation;
        })
        .filter((v): v is Citation => !!v);
    };

    const onEvent = (event: any) => {
      if (!event?.type) return;

      if (event.type === "reset") {
        onResetText?.();
        return;
      }

      if (event.type === "tool_status") {
        const mappedStatus = mapToolStatusText(event.message);
        if (mappedStatus.type === "set") {
          setToolStatus(mappedStatus.text);
        } else if (mappedStatus.type === "clear") {
          setToolStatus(null);
        }
        return;
      }

      if (event.type === "tool_start") {
        const startedAtMs = Date.now();
        const next: ToolStep = {
          id: String(event.id || `tool-${Date.now()}`),
          name: String(event.name || "tool"),
          status: "running",
          args: event.args || {},
          message: event.message || "",
          startedAtMs,
        };
        steps = [...steps, next];
        setStreamingToolSteps([...steps]);
        return;
      }

      if (event.type === "tool_end") {
        const id = String(event.id || "");
        const endedAtMs = Date.now();
        let matched = false;
        steps = steps.map((step) => {
          if (step.id !== id) return step;
          matched = true;
          const startedAtMs = step.startedAtMs || endedAtMs;
          return {
            ...step,
            status: event.status === "error" ? "error" : "done",
            endedAtMs,
            message: `Thought for ${Math.max(
              1,
              Math.round((endedAtMs - startedAtMs) / 1000),
            )}s`,
          };
        });
        if (!matched) {
          steps = [
            ...steps,
            {
              id: id || `tool-${endedAtMs}`,
              name: String(event.name || "tool"),
              status: event.status === "error" ? "error" : "done",
              args: {},
              startedAtMs: endedAtMs,
              endedAtMs,
              message: "Thought for 1s",
            },
          ];
        }
        setStreamingToolSteps([...steps]);

        const parsed = parseCitationsFromResult(event.result);
        if (parsed.length > 0) {
          mergeCitations(parsed);
          setStreamingCitations([...citations]);
        }
      }
    };

    const snapshot = () => ({
      steps: [...steps],
      citations: [...citations],
    });

    return { onEvent, snapshot };
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

    resetToolStreamingState();
    setIsStreaming(true);
    setIsAiTyping(true);
    const requestStartedAtMs = Date.now();

    try {
      let fullResponse = "";
      const toolTracker = createToolEventHandler(() => {
        fullResponse = "";
        setStreamingText("");
      });
      const responseId = Date.now().toString();
      setFirstResponseId(responseId);

      if (signal.aborted) {
        setIsLoading(false);
        return;
      }

      console.log(
        "[useGeminiEngine] Calling startNewThreadStream with model:",
        modelId,
      );
      await startNewThreadStream(
        modelId,
        imgData.path,
        (token: string) => {
          if (signal.aborted) return;
          fullResponse += token;
          setStreamingText(fullResponse);
        },
        config.userName,
        config.userEmail,
        config.userInstruction,
        (brief: string) => {
          if (!isRetry && !imgData.fromHistory && config.generateTitle && config.onTitleGenerated) {
            console.log("[useGeminiEngine] Triggering title generation using image brief");
            config
              .generateTitle(brief)
              .then((title) => {
                console.log("[useGeminiEngine] Title generated:", title);
                if (!signal.aborted) config.onTitleGenerated?.(title);
              })
              .catch(console.error);
          }
        },
        toolTracker.onEvent,
      );
      console.log("[useGeminiEngine] startNewThreadStream finished!");

      if (signal.aborted) {
        setIsLoading(false);
        return;
      }

      const botMsg: Message = {
        id: responseId,
        role: "model",
        text: fullResponse,
        timestamp: Date.now(),
        thoughtSeconds: getElapsedThoughtSeconds(requestStartedAtMs),
        alreadyStreamed: true,
        toolSteps: toolTracker.snapshot().steps,
        citations: toolTracker.snapshot().citations,
      };
      setMessages((prev: Message[]) => {
        const newMsgs = [...prev, botMsg];
        if (config.onOverwriteMessages) {
          config.onOverwriteMessages(newMsgs);
        }
        return newMsgs;
      });
      setStreamingText("");
      resetToolStreamingState();
      setFirstResponseId(null);

      setIsStreaming(false);
      setIsAiTyping(false);
      setIsLoading(false);
    } catch (apiError: any) {
      if (
        signal.aborted ||
        apiError?.message === "CANCELLED" ||
        isRequestCancelledRef.current
      ) {
        setIsLoading(false);
        setIsStreaming(false);
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
          setIsLoading(false);
          setIsStreaming(false);
          setIsAiTyping(false);
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
      resetToolStreamingState();
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
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

    resetToolStreamingState();
    setIsStreaming(true);
    setIsAiTyping(true);
    const requestStartedAtMs = Date.now();

    try {
      let fullResponse = "";
      const toolTracker = createToolEventHandler(() => {
        fullResponse = "";
        setStreamingText("");
      });
      const responseId = Date.now().toString();
      setFirstResponseId(responseId);

      await startNewThreadStream(
        config.currentModel,
        config.startupImage.path,
        (token: string) => {
          fullResponse += token;
          setStreamingText(fullResponse); // ← LIVE UPDATE
        },
        undefined,
        undefined,
        undefined,
        undefined,
        toolTracker.onEvent,
      );

      const botMsg: Message = {
        id: responseId,
        role: "model",
        text: fullResponse,
        timestamp: Date.now(),
        thoughtSeconds: getElapsedThoughtSeconds(requestStartedAtMs),
        alreadyStreamed: true,
        toolSteps: toolTracker.snapshot().steps,
        citations: toolTracker.snapshot().citations,
      };
      setMessages((prev: Message[]) => {
        const newMsgs = [...prev, botMsg];
        if (config.onOverwriteMessages) {
          config.onOverwriteMessages(newMsgs);
        }
        return newMsgs;
      });
      setStreamingText("");
      resetToolStreamingState();
      setFirstResponseId(null);

      setIsStreaming(false);
      setIsAiTyping(false);
      setIsLoading(false);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsLoading(false);
        setIsStreaming(false);
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
    } finally {
      resetToolStreamingState();
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
    }
  };

  const handleRetrySend = async () => {
    if (!lastSentMessage) return;
    setIsLoading(true);
    isRequestCancelledRef.current = false;
    setMessages((prev: Message[]) => [...prev, lastSentMessage]);
    const targetChatId = config.chatId;
    resetToolStreamingState();
    const requestStartedAtMs = Date.now();

    try {
      let fullResponse = "";
      const toolTracker = createToolEventHandler(() => {
        fullResponse = "";
        setStreamingText("");
      });
      const responseId = (Date.now() + 1).toString();
      setIsAiTyping(true);
      setIsStreaming(true);
      setFirstResponseId(responseId);

      const responseText = await apiSendMessage(
        lastSentMessage.text,
        undefined,
        (token: string) => {
          fullResponse += token;
          setStreamingText(fullResponse);
        },
        config.chatId,
        toolTracker.onEvent,
      );
      const botMsg: Message = {
        id: responseId,
        role: "model",
        text: responseText,
        timestamp: Date.now(),
        thoughtSeconds: getElapsedThoughtSeconds(requestStartedAtMs),
        alreadyStreamed: true,
        toolSteps: toolTracker.snapshot().steps,
        citations: toolTracker.snapshot().citations,
      };
      setMessages((prev: Message[]) => {
        const newMsgs = [...prev, botMsg];
        if (config.onOverwriteMessages) {
          config.onOverwriteMessages(newMsgs);
        }
        return newMsgs;
      });
      setLastSentMessage(null);
      setStreamingText("");
      resetToolStreamingState();
      setFirstResponseId(null);
      setIsStreaming(false);
      setIsAiTyping(false);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsStreaming(false);
        setIsAiTyping(false);
        return;
      }
      appendErrorMessage(
        "An error occurred while sending the message. " +
          (apiError.message || ""),
        targetChatId,
      );
    } finally {
      resetToolStreamingState();
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
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
        thoughtSeconds: getThoughtSecondsFromToolSteps(streamingToolSteps),
        citations: streamingCitations,
        toolSteps: streamingToolSteps,
      };
      setMessages((prev: Message[]) => {
        const idx = prev.findIndex((m) => m.id === botMsg.id);
        if (idx !== -1) {
          const newMsgs = [...prev];
          newMsgs[idx] = botMsg;
          if (config.onOverwriteMessages) {
            config.onOverwriteMessages(newMsgs);
          }
          return newMsgs;
        }
        const newMsgs = [...prev, botMsg];
        if (config.onOverwriteMessages) {
          config.onOverwriteMessages(newMsgs);
        }
        return newMsgs;
      });

      setStreamingText("");
      resetToolStreamingState();
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
    resetToolStreamingState();
    const requestStartedAtMs = Date.now();

    try {
      let fullResponse = "";
      const toolTracker = createToolEventHandler(() => {
        fullResponse = "";
        setStreamingText("");
      });
      const responseId = (Date.now() + 1).toString();
      setIsAiTyping(true);
      setIsStreaming(true);
      setFirstResponseId(responseId);

      const responseText = await apiSendMessage(
        userText,
        modelId,
        (token: string) => {
          fullResponse += token;
          setStreamingText(fullResponse);
        },
        config.chatId,
        toolTracker.onEvent,
      );
      const botMsg: Message = {
        id: responseId,
        role: "model",
        text: responseText,
        timestamp: Date.now(),
        thoughtSeconds: getElapsedThoughtSeconds(requestStartedAtMs),
        alreadyStreamed: true,
        toolSteps: toolTracker.snapshot().steps,
        citations: toolTracker.snapshot().citations,
      };
      setMessages((prev: Message[]) => {
        const newMsgs = [...prev, botMsg];
        if (config.onOverwriteMessages) {
          config.onOverwriteMessages(newMsgs);
        }
        return newMsgs;
      });
      setLastSentMessage(null);
      setStreamingText("");
      resetToolStreamingState();
      setFirstResponseId(null);
      setIsStreaming(false);
      setIsAiTyping(false);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsStreaming(false);
        setIsAiTyping(false);
        return;
      }
      appendErrorMessage(
        "An error occurred while sending the message. " +
          (apiError.message || ""),
        targetChatId,
      );
    } finally {
      resetToolStreamingState();
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
    }
  };

  const handleRetryMessage = async (messageId: string, modelId?: string) => {
    const msgIndex = messages.findIndex((m: Message) => m.id === messageId);
    if (msgIndex === -1) return;

    const truncatedMessages = messages.slice(0, msgIndex);
    const retryModelId = modelId || config.currentModel;

    preRetryMessagesRef.current = [...messages];
    
    // Immediately truncate messages to avoid full text flash on completion
    setMessages(truncatedMessages);
    if (config.onOverwriteMessages) {
      config.onOverwriteMessages(truncatedMessages);
    }
    
    setRetryingMessageId(messageId); // Keep for "Analyzing" shimmer if needed
    isRequestCancelledRef.current = false;
    resetToolStreamingState();
    const requestStartedAtMs = Date.now();

    const newResponseId = Date.now().toString();

    let fallbackImagePath: string | undefined;
    if (config.startupImage) {
      fallbackImagePath = config.startupImage.path;
    }

    try {
      let fullResponse = "";
      const toolTracker = createToolEventHandler(() => {
        fullResponse = "";
        setStreamingText("");
      });
      let hasReceivedFirstToken = false;
      setFirstResponseId(newResponseId);
      setIsStreaming(true);
      setIsAiTyping(true);

      const responseText = await apiRetryFromMessage(
        msgIndex,
        messages,
        retryModelId,
        (token: string) => {
          fullResponse += token;
          if (!hasReceivedFirstToken) {
            hasReceivedFirstToken = true;
          }
          setStreamingText(fullResponse);
        },
        fallbackImagePath,
        undefined,
        toolTracker.onEvent,
      );

      if (
        msgIndex === 0 &&
        (!config.chatTitle || config.chatTitle === "New thread") &&
        responseText.length > 50 &&
        config.generateTitle &&
        config.onTitleGenerated
      ) {
        config
          .generateTitle(responseText)
          .then((title) => config.onTitleGenerated?.(title))
          .catch(console.error);
      }

      const botMsg: Message = {
        id: newResponseId,
        role: "model",
        text: responseText,
        timestamp: Date.now(),
        thoughtSeconds: getElapsedThoughtSeconds(requestStartedAtMs),
        alreadyStreamed: true,
        toolSteps: toolTracker.snapshot().steps,
        citations: toolTracker.snapshot().citations,
      };
      setRetryingMessageId(null);
      const newMessages = [...truncatedMessages, botMsg];
      setMessages(newMessages);
      
      if (config.onMessage && config.chatId) {
        config.onMessage(botMsg, config.chatId);
      }
      
      if (config.onOverwriteMessages) {
        config.onOverwriteMessages(newMessages);
      }
      
      setIsStreaming(false);
      setIsAiTyping(false);
      setFirstResponseId(null);
      setStreamingText("");
      resetToolStreamingState();
      setIsLoading(false);
    } catch (apiError: any) {
      if (apiError?.message === "CANCELLED" || isRequestCancelledRef.current) {
        setIsLoading(false);
        setIsStreaming(false);
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
      resetToolStreamingState();
    } finally {
      resetToolStreamingState();
      setIsLoading(false);
      setIsStreaming(false);
      setIsAiTyping(false);
    }
  };

  const handleUndoMessage = (messageId: string) => {
    const msgIndex = messages.findIndex((m: Message) => m.id === messageId);
    if (msgIndex === -1) return;

    const truncatedMessages = messages.slice(0, msgIndex);

    cancelCurrentRequest();
    cleanupAbortController();
    isRequestCancelledRef.current = true;

    setMessages(truncatedMessages);
    config.onOverwriteMessages?.(truncatedMessages);

    setRetryingMessageId(null);
    setLastSentMessage(null);
    setIsLoading(false);
    setIsStreaming(false);
    setIsAiTyping(false);
    setStreamingText("");
    setFirstResponseId(null);
    resetToolStreamingState();

    const firstAssistantMessage = truncatedMessages.find(
      (message: Message) => message.role === "model",
    );
    const firstUserMessage = truncatedMessages.find(
      (message: Message) => message.role === "user",
    );
    const savedHistory = truncatedMessages.map((message: Message) => ({
      role: message.role === "model" ? "Assistant" : "User",
      content: message.text,
    }));

    apiRestoreSession(
      config.currentModel,
      firstAssistantMessage?.text || getImageDescription() || "",
      firstUserMessage?.text || null,
      savedHistory,
      config.startupImage?.path || null,
    );
  };

  const handleStopGeneration = (truncatedText?: string) => {
    isRequestCancelledRef.current = true;
    cancelCurrentRequest();
    cleanupAbortController();

    if (typeof truncatedText === "string") {
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
          thoughtSeconds: getThoughtSecondsFromToolSteps(streamingToolSteps),
          stopped: true,
          alreadyStreamed: true,
          citations: streamingCitations,
          toolSteps: streamingToolSteps,
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
      } else {
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
      }
    } else if (config.state.retryingMessageId) {
      const oldMessages = preRetryMessagesRef.current;
      setMessages(oldMessages);
      config.onOverwriteMessages?.(oldMessages);
    } else {
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
    }

    setRetryingMessageId(null);
    setIsLoading(false);
    setIsStreaming(false);
    setIsAiTyping(false);
    setStreamingText("");
    setFirstResponseId(null);
    resetToolStreamingState();
  };

  const handleAnswerNow = async () => {
    await answerNowCurrentRequest();
    setToolStatus(API_STATUS_TEXT.WRAPPING_UP);
  };

  const handleStreamComplete = () => {
    if (streamingText && firstResponseId) {
      const botMsg: Message = {
        id: firstResponseId,
        role: "model",
        text: streamingText,
        timestamp: Date.now(),
        thoughtSeconds: getThoughtSecondsFromToolSteps(streamingToolSteps),
        alreadyStreamed: true,
        citations: streamingCitations,
        toolSteps: streamingToolSteps,
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
      resetToolStreamingState();
    }
    setIsStreaming(false);
    setIsAiTyping(false);
  };

  return {
    startSession,
    handleSend,
    handleRetrySend,
    handleRetryMessage,
    handleUndoMessage,
    handleDescribeEdits,
    handleStopGeneration,
    handleAnswerNow,
    handleStreamComplete,
    cleanupAbortController,
  };
};
