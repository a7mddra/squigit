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
  isAnswerNowSuppressedProgressText,
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

export const Chat: React.FC = () => {
  const app = useAppContext();
  const [inputValue, setInputValue] = useState("");
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

  // Refs
  const headerRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const bottomAnchorRef = useRef<HTMLDivElement>(null);
  const wasAtBottomRef = useRef(false);
  const imageProgressTurnIdRef = useRef<string | null>(null);

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

  const visibleStartupImage =
    app.system.startupImage ?? (app.isNavigating ? retainedStartupImage : null);
  const showArtifactPlaceholder =
    (app.isNavigating || !app.isChatContentReady) && !visibleStartupImage;

  const revealTarget = app.searchOverlay.pendingReveal;
  const isRevealPendingForActiveChat =
    revealTarget?.chatId === app.chatHistory.activeSessionId;

  const { isSpinnerVisible, isAtBottom } = useChatScroll({
    isNavigating: app.isNavigating,
    scrollContainerRef,
    bottomAnchorRef,
    wasAtBottomRef,
  });
  const showLoadingState = app.isNavigating || !app.isChatContentReady;
  const [showLoadingOverlay, setShowLoadingOverlay] = useState(showLoadingState);
  const [isContentMounted, setIsContentMounted] = useState(
    () => !showLoadingState,
  );
  const deferredMessages = useDeferredValue(app.chat.messages);
  const showScrollToBottomButton =
    !isSpinnerVisible &&
    !showLoadingOverlay &&
    !isRevealPendingForActiveChat &&
    !isAtBottom;

  const handleScrollToBottom = useCallback(() => {
    const bottomAnchor = bottomAnchorRef.current;
    if (!bottomAnchor) return;

    wasAtBottomRef.current = true;
    bottomAnchor.scrollIntoView({
      behavior: "smooth",
      block: "end",
      inline: "nearest",
    });
  }, []);

  useLayoutEffect(() => {
    if (showLoadingState || isRevealPendingForActiveChat) {
      setShowLoadingOverlay(showLoadingState);
      setIsContentMounted(!showLoadingState);
      return;
    }

    if (!isContentMounted) {
      setIsContentMounted(true);
      setShowLoadingOverlay(true);
      return;
    }

    setShowLoadingOverlay(true);

    let revealFrameId: number | null = null;
    const container = scrollContainerRef.current;
    if (container) {
      // Force scrollbar thumb to sync with reversed-flow layout on initial paint.
      void container.offsetHeight;
      container.scrollTop = -1;
      container.scrollTop = 0;
    }
    wasAtBottomRef.current = true;

    revealFrameId = window.requestAnimationFrame(() => {
      setShowLoadingOverlay(false);
    });

    return () => {
      if (revealFrameId !== null) {
        window.cancelAnimationFrame(revealFrameId);
      }
    };
  }, [
    app.chatHistory.activeSessionId,
    isContentMounted,
    isRevealPendingForActiveChat,
    showLoadingState,
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
    const existingPaths = new Set(parseAttachmentPaths(inputValue));
    const imageAttachments = app.attachments.filter(
      (attachment) =>
        attachment.type === "image" && !existingPaths.has(attachment.path),
    );
    let finalInput = inputValue;
    if (imageAttachments.length > 0) {
      const mentions = imageAttachments
        .map((a) => buildAttachmentMention(a.path, a.name))
        .join("\n");
      finalInput = `${inputValue}\n\n${mentions}`.trim();
    }
    if (!finalInput.trim() || app.chat.isLoading) {
      return;
    }

    const parsedPromptAttachments = parseAttachmentPaths(inputValue).map((path) => {
      const sourcePath = app.getAttachmentSourcePath(path) || undefined;
      const originalName = sourcePath ? getBaseName(sourcePath) : undefined;
      return attachmentFromPath(path, undefined, originalName, sourcePath);
    });

    app.trackPendingPromptAttachmentAnalysis(
      dedupeAttachmentsByPath([...app.attachments, ...parsedPromptAttachments]),
    );
    app.chat.handleSend(finalInput, app.inputModel);
    setInputValue("");
    app.clearAttachments();
  }, [
    app.attachments,
    app.chat,
    app.clearAttachments,
    app.getAttachmentSourcePath,
    app.inputModel,
    app.trackPendingPromptAttachmentAnalysis,
    inputValue,
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

      setInputValue(stripImageAttachmentMentions(targetMessage.text));
      app.setAttachments(restoredAttachments);
      app.chat.handleUndoMessage(messageId);
    },
    [app, pendingUndoMessageId],
  );

  useEffect(() => {
    setPendingUndoMessageId(null);
  }, [app.chatHistory.activeSessionId]);

  useEffect(() => {
    setInputValue("");
  }, [app.chatHistory.activeSessionId]);

  useEffect(() => {
    const target = revealTarget;
    if (!target) return;
    if (app.chatHistory.activeSessionId !== target.chatId) return;

    let hideHighlightTimer: number | null = null;

    const revealBubble = (): boolean => {
      const container = scrollContainerRef.current;
      if (!container) return false;

      const selector = `[data-component="chat-bubble"][data-message-index="${target.messageIndex}"]`;
      const bubble = container.querySelector<HTMLElement>(selector);
      if (!bubble) return false;

      bubble.scrollIntoView({
        behavior: "smooth",
        block: "center",
        inline: "nearest",
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
  }, [app.chatHistory.activeSessionId, app.clearSearchReveal, revealTarget]);

  const isImageProgressVisible =
    !!app.system.startupImage &&
    app.chat.messages.length === 0 &&
    !app.chat.streamingText &&
    app.chat.isAnalyzing;
  const imageProgressText = getVisibleImageProgressText(app.chat.toolStatus);
  const delayedImageAttachmentProgressText = getAttachmentAnalysisStatusText(
    app.pendingPromptAttachmentAnalysis,
  );

  useEffect(() => {
    const turnId = app.chat.pendingAssistantTurn?.id ?? null;

    if (!turnId || !isImageProgressVisible) {
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
  }, [app.chat.pendingAssistantTurn?.id, imageProgressText, isImageProgressVisible]);

  useEffect(() => {
    const turnId = app.chat.pendingAssistantTurn?.id;

    if (
      !turnId ||
      !isImageProgressVisible ||
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
  ]);

  const visibleImageProgressText = imageProgressText
    ? imageProgressText
    : isImageProgressVisible &&
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
  const showAnswerNow =
    !isAnswerNowSuppressedProgressText(visibleImageProgressText) &&
    (hasRunningToolStep || hasPreStepSearchStatus);

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
      inputValue={inputValue}
      onInputChange={setInputValue}
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
        showAnswerNow={showAnswerNow}
        visibleImageProgressText={visibleImageProgressText}
        onAnswerNow={app.chat.handleAnswerNow}
        messages={deferredMessages}
        pendingAssistantTurn={app.chat.pendingAssistantTurn}
        pendingPromptAttachmentAnalysis={app.pendingPromptAttachmentAnalysis}
        hideThinkingProgress={app.chat.isAnalyzing}
        selectedModel={app.inputModel}
        onRetryMessage={handleRetryMessage}
        onUndoMessage={handleRequestUndoMessage}
        onSystemAction={app.handleSystemAction}
      />
    </ChatLayout>
  );
};
