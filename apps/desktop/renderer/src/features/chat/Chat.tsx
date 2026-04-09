/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useRef,
  useState,
  useEffect,
  useLayoutEffect,
  useCallback,
  useDeferredValue,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppContext } from "@/providers/AppProvider";
import { useInlineMenu } from "@/hooks";
import {
  API_STATUS_TEXT,
  ATTACHMENT_ANALYSIS_STATUS_DELAY_MS,
  attachmentFromPath,
  buildAttachmentMention,
  getAttachmentAnalysisStatusText,
  isQuickAnswerSuppressedProgressText,
  parseAttachmentPaths,
  stripImageAttachmentMentions,
  type Attachment,
} from "@/lib";
import {
  useChatScroll,
  useInputHeight,
  useChatWheel,
  useChatError,
} from "@/features";
import { ChatLayout } from "./ChatLayout";
import { ChatContent } from "./ChatContent";
import type { MessageCollapseMode } from "./chat.types";
import styles from "./Chat.module.css";

function getBaseName(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

function getVisibleImageProgressText(text: string | null | undefined): string | null {
  const trimmed = text?.trim();
  if (!trimmed || trimmed === API_STATUS_TEXT.ANALYZING_IMAGE) {
    return null;
  }
  return trimmed;
}

function dedupeAttachmentsByPath(items: Attachment[]): Attachment[] {
  const byPath = new Map<string, Attachment>();
  for (const item of items) {
    if (!byPath.has(item.path)) {
      byPath.set(item.path, item);
    }
  }
  return Array.from(byPath.values());
}

const BOTTOM_SYNC_EPSILON_PX = 2;
const BOTTOM_STABLE_MS = 260;
const BOTTOM_SETTLE_POLL_MS = 100;
const MESSAGE_WINDOW_CHUNK = 24;
const HISTORY_LOAD_TRIGGER_TOP_PX = 2;
const HISTORY_LOAD_DELAY_MS = 180;

function areIdSetsEqual(left: Set<string>, right: Set<string>): boolean {
  if (left === right) return true;
  if (left.size !== right.size) return false;

  for (const value of left) {
    if (!right.has(value)) {
      return false;
    }
  }

  return true;
}

export const Chat: React.FC = () => {
  const app = useAppContext();
  const [pendingUndoMessageId, setPendingUndoMessageId] = useState<
    string | null
  >(null);
  const [delayedImageAttachmentStatus, setDelayedImageAttachmentStatus] =
    useState<{
      turnId: string;
      text: string;
    } | null>(null);
  const [retainedStartupImage, setRetainedStartupImage] = useState(
    app.system.startupImage,
  );
  const [isStreamingAutoScrollEnabled, setIsStreamingAutoScrollEnabled] =
    useState(false);
  const [autoCollapsedMessageIds, setAutoCollapsedMessageIds] = useState<
    Set<string>
  >(() => new Set());
  const [manuallyExpandedMessageIds, setManuallyExpandedMessageIds] = useState<
    Set<string>
  >(() => new Set());
  const [loadedMessageCount, setLoadedMessageCount] = useState(
    MESSAGE_WINDOW_CHUNK,
  );
  const [isLoadingOlderMessages, setIsLoadingOlderMessages] = useState(false);

  // Refs
  const headerRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const bottomAnchorRef = useRef<HTMLDivElement>(null);
  const scrollToBottomButtonRef = useRef<HTMLButtonElement>(null);
  const wasAtBottomRef = useRef(false);
  const imageProgressTurnIdRef = useRef<string | null>(null);
  const isProgrammaticScrollRef = useRef(false);
  const previousCommittedMessageIdsRef = useRef<string[]>([]);
  const collapseAllOnLoadRef = useRef(true);
  const pendingHistoryPrependRef = useRef<{ previousScrollHeight: number } | null>(
    null,
  );
  const historyLoadTimerRef = useRef<number | null>(null);
  const latestCommittedMessageCountRef = useRef(app.chat.messages.length);

  // Hooks
  const error = app.chat.error || app.system.systemError;
  const { isErrorOpen, parsedError, errorActions } = useChatError(
    error,
    app.system.openSettings,
  );
  const { isImageExpanded, setIsImageExpanded } = useChatWheel(
    headerRef,
    app.chatHistory.activeSessionId,
  );

  const { inputContainerRef, inputHeight } = useInputHeight();

  useEffect(() => {
    if (app.system.startupImage) {
      setRetainedStartupImage(app.system.startupImage);
      return;
    }

    if (!app.isNavigating) {
      setRetainedStartupImage(null);
    }
  }, [app.isNavigating, app.system.startupImage]);

  useEffect(() => {
    const lockWindowScroll = () => {
      const html = document.documentElement;
      const body = document.body;

      if (
        window.scrollX === 0 &&
        window.scrollY === 0 &&
        html.scrollTop === 0 &&
        html.scrollLeft === 0 &&
        body.scrollTop === 0 &&
        body.scrollLeft === 0
      ) {
        return;
      }

      window.scrollTo({ top: 0, left: 0, behavior: "auto" });
      html.scrollTop = 0;
      html.scrollLeft = 0;
      body.scrollTop = 0;
      body.scrollLeft = 0;
    };

    lockWindowScroll();
    window.addEventListener("scroll", lockWindowScroll, { passive: true });

    return () => {
      window.removeEventListener("scroll", lockWindowScroll);
    };
  }, []);

  const visibleStartupImage =
    app.system.startupImage ?? (app.isNavigating ? retainedStartupImage : null);
  const isNavigationLoading = app.isNavigating || !app.isChatContentReady;
  const showArtifactPlaceholder = isNavigationLoading && !visibleStartupImage;

  const revealTarget = app.searchOverlay.pendingReveal;
  const isRevealPendingForActiveChat =
    revealTarget?.chatId === app.chatHistory.activeSessionId;

  const { isSpinnerVisible, isAtBottom } = useChatScroll({
    isNavigating: app.isNavigating,
    scrollContainerRef,
    bottomAnchorRef,
    wasAtBottomRef,
  });
  const [showLoadingOverlay, setShowLoadingOverlay] = useState(
    isNavigationLoading,
  );
  const [isContentMounted, setIsContentMounted] = useState(
    !isNavigationLoading,
  );
  const deferredMessages = useDeferredValue(app.chat.messages);
  const [pendingAssistantTurnBridge, setPendingAssistantTurnBridge] = useState(
    app.chat.pendingAssistantTurn,
  );
  const lastPendingAssistantTurnRef = useRef(app.chat.pendingAssistantTurn);
  const effectivePendingAssistantTurn =
    app.chat.pendingAssistantTurn ?? pendingAssistantTurnBridge;
  const shouldUseDeferredMessages =
    !!effectivePendingAssistantTurn &&
    deferredMessages.length <= app.chat.messages.length &&
    deferredMessages.every(
      (message, index) => message.id === app.chat.messages[index]?.id,
    );
  const visibleMessages = shouldUseDeferredMessages
    ? deferredMessages
    : app.chat.messages;
  const clampedLoadedMessageCount = Math.max(
    MESSAGE_WINDOW_CHUNK,
    loadedMessageCount,
  );
  const messageWindowStartIndex = Math.max(
    0,
    visibleMessages.length - clampedLoadedMessageCount,
  );
  const visibleWindowedMessages = visibleMessages.slice(messageWindowStartIndex);
  const hasOlderHiddenMessages =
    app.chat.messages.length > visibleWindowedMessages.length;
  const showScrollToBottomButton =
    !isSpinnerVisible &&
    !showLoadingOverlay &&
    !isRevealPendingForActiveChat &&
    !isAtBottom;

  const scrollBottomIntoView = useCallback(
    (behavior: ScrollBehavior = "auto") => {
      const container = scrollContainerRef.current;
      if (!container) return;

      isProgrammaticScrollRef.current = true;
      const maxY = Math.max(0, container.scrollHeight - container.clientHeight);
      container.scrollTo({ top: maxY, behavior });
      window.requestAnimationFrame(() => {
        window.requestAnimationFrame(() => {
          isProgrammaticScrollRef.current = false;
        });
      });
    },
    [],
  );

  const snapToBottomAfterSend = useCallback(() => {
    wasAtBottomRef.current = true;
    scrollBottomIntoView("auto");

    window.requestAnimationFrame(() => {
      // Run one more frame to catch late layout updates from streamed placeholders.
      scrollBottomIntoView("auto");
    });
  }, [scrollBottomIntoView]);

  const handleScrollToBottom = useCallback(() => {
    wasAtBottomRef.current = true;
    if (app.chat.pendingAssistantTurn) {
      setIsStreamingAutoScrollEnabled(true);
      scrollBottomIntoView("auto");
      return;
    }
    scrollBottomIntoView(showLoadingOverlay ? "auto" : "smooth");
  }, [app.chat.pendingAssistantTurn, scrollBottomIntoView, showLoadingOverlay]);

  const triggerScrollToBottomButton = useCallback(() => {
    const button = scrollToBottomButtonRef.current;
    if (button) {
      button.click();
      return;
    }

    handleScrollToBottom();
  }, [handleScrollToBottom]);

  useEffect(() => {
    latestCommittedMessageCountRef.current = app.chat.messages.length;
  }, [app.chat.messages.length]);

  useEffect(() => {
    if (!app.chat.pendingAssistantTurn) {
      setIsStreamingAutoScrollEnabled(false);
    }
  }, [app.chat.pendingAssistantTurn]);

  useEffect(() => {
    const livePendingTurn = app.chat.pendingAssistantTurn;

    if (livePendingTurn) {
      lastPendingAssistantTurnRef.current = livePendingTurn;
      setPendingAssistantTurnBridge((previous) =>
        previous ? null : previous,
      );
      return;
    }

    const lastPendingTurn = lastPendingAssistantTurnRef.current;
    if (!lastPendingTurn) {
      setPendingAssistantTurnBridge((previous) =>
        previous ? null : previous,
      );
      return;
    }

    const wasCompletingTurn =
      lastPendingTurn.phase === "complete" || lastPendingTurn.phase === "stopped";
    const hasCommittedTurn = app.chat.messages.some(
      (message) => message.id === lastPendingTurn.id,
    );

    if (wasCompletingTurn && !hasCommittedTurn) {
      setPendingAssistantTurnBridge((previous) =>
        previous?.id === lastPendingTurn.id ? previous : lastPendingTurn,
      );
      return;
    }

    setPendingAssistantTurnBridge((previous) =>
      previous ? null : previous,
    );
    lastPendingAssistantTurnRef.current = null;
  }, [app.chat.messages, app.chat.pendingAssistantTurn]);

  useEffect(() => {
    setIsStreamingAutoScrollEnabled(false);
  }, [app.chatHistory.activeSessionId]);

  useEffect(() => {
    if (historyLoadTimerRef.current !== null) {
      window.clearTimeout(historyLoadTimerRef.current);
      historyLoadTimerRef.current = null;
    }
    setAutoCollapsedMessageIds(new Set());
    setManuallyExpandedMessageIds(new Set());
    setLoadedMessageCount(MESSAGE_WINDOW_CHUNK);
    setIsLoadingOlderMessages(false);
    previousCommittedMessageIdsRef.current = [];
    collapseAllOnLoadRef.current = true;
    pendingHistoryPrependRef.current = null;
  }, [app.chatHistory.activeSessionId]);

  useEffect(() => {
    const currentMessages = app.chat.messages;
    const currentIds = new Set(currentMessages.map((message) => message.id));
    const previousIds = previousCommittedMessageIdsRef.current;
    const previousIdSet = new Set(previousIds);
    const newlyAddedMessages = currentMessages.filter(
      (message) => !previousIdSet.has(message.id),
    );

    const prunedAutoCollapsed = new Set(
      Array.from(autoCollapsedMessageIds).filter((id) => currentIds.has(id)),
    );
    const prunedManualExpanded = new Set(
      Array.from(manuallyExpandedMessageIds).filter((id) => currentIds.has(id)),
    );

    const shouldCollapseAllOnLoad =
      collapseAllOnLoadRef.current &&
      previousIds.length === 0 &&
      currentMessages.length > 0 &&
      !app.chat.pendingAssistantTurn;

    if (shouldCollapseAllOnLoad) {
      for (const message of currentMessages) {
        if (message.role === "user" || message.role === "model") {
          prunedAutoCollapsed.add(message.id);
        }
      }
      collapseAllOnLoadRef.current = false;
    } else {
      for (const message of newlyAddedMessages) {
        if (message.role === "user") {
          prunedAutoCollapsed.add(message.id);
        }
      }

      const hasNewBotMessage = newlyAddedMessages.some(
        (message) => message.role === "model",
      );
      if (hasNewBotMessage) {
        const allBotMessages = currentMessages.filter(
          (message) => message.role === "model",
        );
        const latestBotId =
          allBotMessages.length > 0
            ? allBotMessages[allBotMessages.length - 1].id
            : null;

        for (const botMessage of allBotMessages) {
          if (botMessage.id !== latestBotId) {
            prunedAutoCollapsed.add(botMessage.id);
          }
        }
      }
    }

    if (!areIdSetsEqual(prunedAutoCollapsed, autoCollapsedMessageIds)) {
      setAutoCollapsedMessageIds(prunedAutoCollapsed);
    }
    if (!areIdSetsEqual(prunedManualExpanded, manuallyExpandedMessageIds)) {
      setManuallyExpandedMessageIds(prunedManualExpanded);
    }

    previousCommittedMessageIdsRef.current = currentMessages.map(
      (message) => message.id,
    );
  }, [
    app.chat.messages,
    app.chat.pendingAssistantTurn,
    autoCollapsedMessageIds,
    manuallyExpandedMessageIds,
  ]);

  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const handleHistoryTopLoad = () => {
      if (isProgrammaticScrollRef.current) {
        return;
      }
      if (container.scrollTop > HISTORY_LOAD_TRIGGER_TOP_PX) {
        return;
      }
      if (isLoadingOlderMessages || historyLoadTimerRef.current !== null) {
        return;
      }
      if (loadedMessageCount >= latestCommittedMessageCountRef.current) {
        return;
      }

      pendingHistoryPrependRef.current = {
        previousScrollHeight: container.scrollHeight,
      };
      setIsLoadingOlderMessages(true);
      historyLoadTimerRef.current = window.setTimeout(() => {
        historyLoadTimerRef.current = null;
        setLoadedMessageCount((previous) =>
          Math.min(
            previous + MESSAGE_WINDOW_CHUNK,
            latestCommittedMessageCountRef.current,
          ),
        );
      }, HISTORY_LOAD_DELAY_MS);
    };

    container.addEventListener("scroll", handleHistoryTopLoad, { passive: true });
    return () => {
      container.removeEventListener("scroll", handleHistoryTopLoad);
    };
  }, [isLoadingOlderMessages, loadedMessageCount]);

  useLayoutEffect(() => {
    const pendingPrepend = pendingHistoryPrependRef.current;
    if (!pendingPrepend) return;

    const container = scrollContainerRef.current;
    if (container) {
      const addedHeight =
        container.scrollHeight - pendingPrepend.previousScrollHeight;
      if (addedHeight > 0) {
        isProgrammaticScrollRef.current = true;
        container.scrollTop = Math.max(0, container.scrollTop + addedHeight);
        window.requestAnimationFrame(() => {
          isProgrammaticScrollRef.current = false;
        });
      }
    }

    pendingHistoryPrependRef.current = null;
    setIsLoadingOlderMessages(false);
  }, [visibleWindowedMessages.length]);

  useEffect(() => {
    return () => {
      if (historyLoadTimerRef.current !== null) {
        window.clearTimeout(historyLoadTimerRef.current);
      }
    };
  }, []);

  useLayoutEffect(() => {
    if (isNavigationLoading) {
      setShowLoadingOverlay(true);
      setIsContentMounted(false);
      return;
    }

    if (!isContentMounted) {
      setIsContentMounted(true);
      return;
    }

    if (isRevealPendingForActiveChat) {
      setShowLoadingOverlay(false);
      return;
    }
  }, [
    app.chatHistory.activeSessionId,
    isContentMounted,
    isNavigationLoading,
    isRevealPendingForActiveChat,
  ]);

  useLayoutEffect(() => {
    const shouldFollowStreaming =
      !!app.chat.pendingAssistantTurn && isStreamingAutoScrollEnabled;
    const shouldKeepPinnedForNonStreaming =
      !app.chat.pendingAssistantTurn && wasAtBottomRef.current;

    if (
      isNavigationLoading ||
      showLoadingOverlay ||
      isRevealPendingForActiveChat ||
      (!shouldFollowStreaming && !shouldKeepPinnedForNonStreaming)
    ) {
      return;
    }

    scrollBottomIntoView("auto");
  }, [
    app.chat.messages,
    app.chat.pendingAssistantTurn,
    isNavigationLoading,
    isStreamingAutoScrollEnabled,
    isRevealPendingForActiveChat,
    scrollBottomIntoView,
    showLoadingOverlay,
  ]);

  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const handleScroll = () => {
      if (!isStreamingAutoScrollEnabled || !app.chat.pendingAssistantTurn) {
        return;
      }
      if (isProgrammaticScrollRef.current) {
        return;
      }

      const maxY = Math.max(0, container.scrollHeight - container.clientHeight);
      const y = Math.max(0, container.scrollTop);
      if (Math.abs(maxY - y) > BOTTOM_SYNC_EPSILON_PX) {
        setIsStreamingAutoScrollEnabled(false);
      }
    };

    container.addEventListener("scroll", handleScroll, { passive: true });
    return () => {
      container.removeEventListener("scroll", handleScroll);
    };
  }, [app.chat.pendingAssistantTurn, isStreamingAutoScrollEnabled]);

  useEffect(() => {
    if (!isNavigationLoading) {
      return;
    }

    wasAtBottomRef.current = false;
    setShowLoadingOverlay(true);
    setIsContentMounted(false);
  }, [app.chatHistory.activeSessionId, isNavigationLoading]);

  const isDeferredContentReady = deferredMessages === app.chat.messages;

  useEffect(() => {
    if (
      isNavigationLoading ||
      !isContentMounted ||
      isRevealPendingForActiveChat ||
      !showLoadingOverlay
    ) {
      return;
    }

    let bottomStableSince: number | null = null;

    const syncOverlayToRealScrollPosition = () => {
      const container = scrollContainerRef.current;
      if (!container) {
        return;
      }

      const maxY = Math.max(0, container.scrollHeight - container.clientHeight);
      const y = Math.max(0, container.scrollTop);
      const isAtBottomByPosition =
        Math.abs(maxY - y) <= BOTTOM_SYNC_EPSILON_PX;

      if (!isAtBottomByPosition) {
        bottomStableSince = null;
        triggerScrollToBottomButton();
        return;
      }

      if (!isDeferredContentReady) {
        bottomStableSince = null;
        return;
      }

      if (bottomStableSince === null) {
        bottomStableSince = Date.now();
        return;
      }

      if (Date.now() - bottomStableSince >= BOTTOM_STABLE_MS) {
        wasAtBottomRef.current = true;
        setShowLoadingOverlay(false);
      }
    };

    syncOverlayToRealScrollPosition();

    const settleIntervalId = window.setInterval(
      syncOverlayToRealScrollPosition,
      BOTTOM_SETTLE_POLL_MS,
    );

    return () => {
      window.clearInterval(settleIntervalId);
    };
  }, [
    app.chat.messages,
    deferredMessages,
    isContentMounted,
    isDeferredContentReady,
    isNavigationLoading,
    isRevealPendingForActiveChat,
    showLoadingOverlay,
    triggerScrollToBottomButton,
  ]);

  // Capture listen
  useEffect(() => {
    const unlistenPromise = listen<{ tempPath: string }>(
      "capture-to-input",
      (event) => {
        if (event.payload && event.payload.tempPath) {
          app.addAttachmentFromPath(event.payload.tempPath);
        }
      },
    );
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [app.addAttachmentFromPath]);

  // Menu Hooks
  const showFlatMenuRef = useRef<
    ((rect: { left: number; width: number; top: number }) => void) | null
  >(null);

  const handleSelectAll = () => {
    const selection = window.getSelection();
    if (!selection || !selection.anchorNode) return;

    const anchorNode = selection.anchorNode;
    const element =
      anchorNode instanceof Element ? anchorNode : anchorNode.parentElement;
    const bubble = element?.closest('[data-component="chat-bubble"]');

    if (bubble) {
      selection.removeAllRanges();
      const range = document.createRange();
      range.selectNodeContents(bubble);
      selection.addRange(range);

      const rect = bubble.getBoundingClientRect();
      const menuWidth = 250;
      const centerX = rect.left + rect.width / 2;
      const targetLeft = Math.max(
        10,
        Math.min(centerX - menuWidth / 2, window.innerWidth - menuWidth - 10),
      );
      const targetTop = Math.max(10, rect.top + 2);

      if (showFlatMenuRef.current) {
        showFlatMenuRef.current({
          left: targetLeft,
          top: targetTop,
          width: menuWidth,
        });
      }
    }
  };

  const {
    menuRef,
    sliderRef,
    page1Ref,
    page2Ref,
    pageFlatRef,
    handleAction,
    switchPage,
    showFlatMenu,
  } = useInlineMenu({
    containerRef: scrollContainerRef,
    onSelectAll: handleSelectAll,
  });

  useEffect(() => {
    showFlatMenuRef.current = showFlatMenu;
  }, [showFlatMenu]);

  // Handlers
  const handleSend = useCallback(() => {
    const existingPaths = new Set(parseAttachmentPaths(app.input));
    const imageAttachments = app.attachments.filter(
      (attachment) =>
        attachment.type === "image" && !existingPaths.has(attachment.path),
    );
    let finalInput = app.input;
    if (imageAttachments.length > 0) {
      const mentions = imageAttachments
        .map((a) => buildAttachmentMention(a.path, a.name))
        .join("\n");
      finalInput = `${app.input}\n\n${mentions}`.trim();
    }
    if (!finalInput.trim() || app.chat.isLoading) {
      return;
    }

    const parsedPromptAttachments = parseAttachmentPaths(app.input).map(
      (path) => {
        const sourcePath = app.getAttachmentSourcePath(path) || undefined;
        const originalName = sourcePath ? getBaseName(sourcePath) : undefined;
        return attachmentFromPath(path, undefined, originalName, sourcePath);
      },
    );

    app.trackPendingPromptAttachmentAnalysis(
      dedupeAttachmentsByPath([...app.attachments, ...parsedPromptAttachments]),
    );
    app.chat.handleSend(finalInput, app.inputModel);
    app.setInput("");
    app.clearAttachments();
    snapToBottomAfterSend();
  }, [
    app.attachments,
    app.chat,
    app.clearAttachments,
    app.getAttachmentSourcePath,
    app.input,
    app.inputModel,
    app.setInput,
    app.trackPendingPromptAttachmentAnalysis,
    snapToBottomAfterSend,
  ]);

  const handleCaptureToInput = useCallback(async () => {
    try {
      await invoke("spawn_capture_to_input");
    } catch (err) {
      console.error("Failed to spawn capture to input:", err);
    }
  }, []);

  const handleRequestUndoMessage = useCallback((messageId: string) => {
    setPendingUndoMessageId(messageId);
  }, []);

  const getRetryAttachments = useCallback(
    (messageId: string) => {
      const targetIndex = app.chat.messages.findIndex(
        (message) => message.id === messageId,
      );
      if (targetIndex === -1) {
        return [];
      }

      if (targetIndex === 0 && app.system.startupImage) {
        return [
          attachmentFromPath(
            app.system.startupImage.path,
            app.system.startupImage.imageId,
          ),
        ];
      }

      const sourceUserMessage = app.chat.messages
        .slice(0, targetIndex)
        .reverse()
        .find((message) => message.role === "user");

      if (!sourceUserMessage) {
        return [];
      }

      return parseAttachmentPaths(sourceUserMessage.text).map((path) =>
        attachmentFromPath(path),
      );
    },
    [app.chat.messages, app.system.startupImage],
  );

  const handleRetryMessage = useCallback(
    (messageId: string, modelId?: string) => {
      app.trackPendingPromptAttachmentAnalysis(getRetryAttachments(messageId));
      app.chat.handleRetryMessage(messageId, modelId);
    },
    [app, getRetryAttachments],
  );

  const handleUndoDialogAction = useCallback(
    (actionKey: string) => {
      const messageId = pendingUndoMessageId;
      setPendingUndoMessageId(null);

      if (actionKey !== "confirm" || !messageId) {
        return;
      }

      const targetMessage = app.chat.messages.find(
        (message) => message.id === messageId && message.role === "user",
      );
      if (!targetMessage) return;

      const restoredAttachments = parseAttachmentPaths(targetMessage.text)
        .map((path) => {
          const sourcePath = app.getAttachmentSourcePath(path) || undefined;
          const originalName = sourcePath ? getBaseName(sourcePath) : undefined;
          return attachmentFromPath(path, undefined, originalName, sourcePath);
        })
        .filter((attachment) => attachment.type === "image");

      app.setInput(stripImageAttachmentMentions(targetMessage.text));
      app.setAttachments(restoredAttachments);
      app.chat.handleUndoMessage(messageId);
    },
    [app, pendingUndoMessageId],
  );

  useEffect(() => {
    setPendingUndoMessageId(null);
  }, [app.chatHistory.activeSessionId]);

  useEffect(() => {
    const target = revealTarget;
    if (!target) return;
    if (app.chatHistory.activeSessionId !== target.chatId) return;
    if (target.messageIndex >= 0) {
      const requiredVisibleCount =
        latestCommittedMessageCountRef.current - target.messageIndex;
      if (
        Number.isFinite(requiredVisibleCount) &&
        requiredVisibleCount > loadedMessageCount
      ) {
        setLoadedMessageCount(
          Math.min(
            latestCommittedMessageCountRef.current,
            requiredVisibleCount + MESSAGE_WINDOW_CHUNK,
          ),
        );
        return;
      }
    }

    wasAtBottomRef.current = false;

    let hideHighlightTimer: number | null = null;

    const revealBubble = (): boolean => {
      const container = scrollContainerRef.current;
      if (!container) return false;

      const selector = `[data-component="chat-bubble"][data-message-index="${target.messageIndex}"]`;
      const bubble = container.querySelector<HTMLElement>(selector);
      if (!bubble) return false;

      const containerRect = container.getBoundingClientRect();
      const bubbleRect = bubble.getBoundingClientRect();
      const maxY = Math.max(0, container.scrollHeight - container.clientHeight);
      const bubbleTopInContainer =
        bubbleRect.top - containerRect.top + container.scrollTop;
      const centeredTop =
        bubbleTopInContainer - (container.clientHeight - bubbleRect.height) / 2;
      const targetTop = Math.min(maxY, Math.max(0, centeredTop));

      container.scrollTo({
        top: targetTop,
        behavior: "smooth",
      });
      bubble.classList.add(styles.revealFlash);

      if (hideHighlightTimer !== null) {
        window.clearTimeout(hideHighlightTimer);
      }
      hideHighlightTimer = window.setTimeout(() => {
        bubble.classList.remove(styles.revealFlash);
      }, 1400);

      app.clearSearchReveal();
      return true;
    };

    if (revealBubble()) {
      return () => {
        if (hideHighlightTimer !== null) {
          window.clearTimeout(hideHighlightTimer);
        }
      };
    }

    let attempts = 0;
    const pollTimer = window.setInterval(() => {
      attempts += 1;
      if (revealBubble() || attempts >= 30) {
        window.clearInterval(pollTimer);
        if (attempts >= 30) {
          app.clearSearchReveal();
        }
      }
    }, 60);

    return () => {
      window.clearInterval(pollTimer);
      if (hideHighlightTimer !== null) {
        window.clearTimeout(hideHighlightTimer);
      }
    };
  }, [
    app.chatHistory.activeSessionId,
    app.clearSearchReveal,
    loadedMessageCount,
    revealTarget,
  ]);

  const isImageProgressVisible =
    !!app.system.startupImage &&
    app.chat.messages.length === 0 &&
    !app.chat.streamingText &&
    app.chat.isAnalyzing;
  const imageProgressText = getVisibleImageProgressText(app.chat.toolStatus);
  const isInitialRetryTurn =
    app.chat.pendingAssistantTurn?.requestKind === "initial" &&
    !!app.chat.retryingMessageId;
  const delayedImageAttachmentProgressText = getAttachmentAnalysisStatusText(
    app.pendingPromptAttachmentAnalysis,
  );

  useEffect(() => {
    const turnId = app.chat.pendingAssistantTurn?.id ?? null;

    if (!turnId || !isImageProgressVisible || isInitialRetryTurn) {
      imageProgressTurnIdRef.current = null;
      setDelayedImageAttachmentStatus(null);
      return;
    }

    if (imageProgressText) {
      imageProgressTurnIdRef.current = turnId;
      setDelayedImageAttachmentStatus((previous) =>
        previous?.turnId === turnId ? null : previous,
      );
      return;
    }

    if (imageProgressTurnIdRef.current !== turnId) {
      setDelayedImageAttachmentStatus((previous) =>
        previous?.turnId === turnId ? previous : null,
      );
    }
  }, [
    app.chat.pendingAssistantTurn?.id,
    imageProgressText,
    isImageProgressVisible,
    isInitialRetryTurn,
  ]);

  useEffect(() => {
    const turnId = app.chat.pendingAssistantTurn?.id;

    if (
      !turnId ||
      !isImageProgressVisible ||
      isInitialRetryTurn ||
      !!imageProgressText ||
      !delayedImageAttachmentProgressText ||
      imageProgressTurnIdRef.current === turnId ||
      delayedImageAttachmentStatus?.turnId === turnId
    ) {
      return;
    }

    const timerId = window.setTimeout(() => {
      setDelayedImageAttachmentStatus((previous) => {
        if (previous?.turnId === turnId) {
          return previous;
        }

        return {
          turnId,
          text: delayedImageAttachmentProgressText,
        };
      });
    }, ATTACHMENT_ANALYSIS_STATUS_DELAY_MS);

    return () => {
      window.clearTimeout(timerId);
    };
  }, [
    app.chat.pendingAssistantTurn?.id,
    delayedImageAttachmentProgressText,
    delayedImageAttachmentStatus?.turnId,
    imageProgressText,
    isImageProgressVisible,
    isInitialRetryTurn,
  ]);

  const visibleImageProgressText = imageProgressText
    ? imageProgressText
    : !isInitialRetryTurn &&
        isImageProgressVisible &&
        delayedImageAttachmentStatus?.turnId === app.chat.pendingAssistantTurn?.id
      ? delayedImageAttachmentStatus?.text ?? null
      : null;
  const hasRunningToolStep = app.chat.streamingToolSteps.some(
    (step) => step.status === "running",
  );
  const hasPreStepSearchStatus =
    !!app.chat.toolStatus &&
    app.chat.streamingToolSteps.length === 0 &&
    app.chat.isSearching;
  const showQuickAnswer =
    !isQuickAnswerSuppressedProgressText(visibleImageProgressText) &&
    (hasRunningToolStep || hasPreStepSearchStatus);
  const getMessageCollapseMode = useCallback(
    (messageId: string): MessageCollapseMode => {
      if (manuallyExpandedMessageIds.has(messageId)) {
        return "expanded";
      }
      if (autoCollapsedMessageIds.has(messageId)) {
        return "collapsed";
      }
      return "none";
    },
    [autoCollapsedMessageIds, manuallyExpandedMessageIds],
  );
  const handleToggleMessageCollapse = useCallback(
    (messageId: string, nextExpanded: boolean) => {
      if (nextExpanded) {
        setManuallyExpandedMessageIds((previous) => {
          if (previous.has(messageId)) {
            return previous;
          }
          const next = new Set(previous);
          next.add(messageId);
          return next;
        });
        return;
      }

      setManuallyExpandedMessageIds((previous) => {
        if (!previous.has(messageId)) {
          return previous;
        }
        const next = new Set(previous);
        next.delete(messageId);
        return next;
      });
    },
    [],
  );

  return (
    <ChatLayout
      headerRef={headerRef}
      scrollContainerRef={scrollContainerRef}
      bottomAnchorRef={bottomAnchorRef}
      inputContainerRef={inputContainerRef}
      inputHeight={inputHeight}
      visibleStartupImage={visibleStartupImage}
      showArtifactPlaceholder={showArtifactPlaceholder}
      showLoadingState={showLoadingOverlay}
      showHistoryLoadSpinner={isLoadingOlderMessages && hasOlderHiddenMessages}
      isContentMounted={isContentMounted}
      isNavigating={app.isNavigating}
      isImageExpanded={isImageExpanded}
      onToggleImageExpanded={() => setIsImageExpanded(!isImageExpanded)}
      sessionLensUrl={app.sessionLensUrl}
      setSessionLensUrl={app.handleUpdateLensUrl}
      chatTitle={app.chatTitle}
      onDescribeEdits={app.chat.handleDescribeEdits}
      ocrData={app.ocrData}
      onUpdateOCRData={app.handleUpdateOCRData}
      onOpenSettings={app.system.openSettings}
      chatId={app.chatHistory.activeSessionId}
      imageInput={app.imageInput}
      onImageInputChange={app.setImageInput}
      ocrEnabled={app.system.ocrEnabled}
      autoExpandOCR={app.system.autoExpandOCR}
      activeProfileId={app.system.activeProfile?.id || null}
      currentOcrModel={app.system.sessionOcrLanguage}
      onOcrModelChange={app.system.setSessionOcrLanguage}
      isOcrScanning={app.isOcrScanning}
      onOcrScanningChange={app.setIsOcrScanning}
      inputValue={app.input}
      onInputChange={app.setInput}
      onSend={handleSend}
      isChatLoading={app.chat.isLoading}
      isAiTyping={app.chat.isAiTyping}
      isStoppable={app.chat.isAnalyzing || app.chat.isGenerating}
      onStopGeneration={app.chat.handleStopGeneration}
      selectedModel={app.inputModel}
      onModelChange={app.setInputModel}
      attachments={app.attachments}
      onAttachmentsChange={app.setAttachments}
      onCaptureToInput={handleCaptureToInput}
      onPreviewAttachment={app.openMediaViewer}
      rememberAttachmentSourcePath={app.rememberAttachmentSourcePath}
      showScrollToBottomButton={showScrollToBottomButton}
      keepScrollToBottomButtonMounted={showLoadingOverlay}
      scrollToBottomButtonRef={scrollToBottomButtonRef}
      onScrollToBottom={handleScrollToBottom}
      menuRef={menuRef}
      sliderRef={sliderRef}
      page1Ref={page1Ref}
      page2Ref={page2Ref}
      pageFlatRef={pageFlatRef}
      onInlineMenuAction={handleAction}
      onInlineMenuSwitchPage={switchPage}
    >
      <ChatContent
        parsedError={parsedError}
        isErrorOpen={isErrorOpen}
        errorActions={errorActions}
        pendingUndoMessageId={pendingUndoMessageId}
        onUndoDialogAction={handleUndoDialogAction}
        isImageProgressVisible={isImageProgressVisible}
        showQuickAnswer={showQuickAnswer}
        visibleImageProgressText={visibleImageProgressText}
        onQuickAnswer={app.chat.handleQuickAnswer}
        messages={visibleWindowedMessages}
        pendingAssistantTurn={effectivePendingAssistantTurn}
        pendingPromptAttachmentAnalysis={app.pendingPromptAttachmentAnalysis}
        hideThinkingProgress={app.chat.isAnalyzing}
        selectedModel={app.inputModel}
        getMessageCollapseMode={getMessageCollapseMode}
        onToggleMessageCollapse={handleToggleMessageCollapse}
        onRetryMessage={handleRetryMessage}
        onUndoMessage={handleRequestUndoMessage}
        onSystemAction={app.handleSystemAction}
      />
    </ChatLayout>
  );
};
