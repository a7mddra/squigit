/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef, useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppContext } from "@/providers/AppProvider";
import { useInlineMenu } from "@/hooks";
import { InlineMenu, LoadingSpinner, Dialog, TextShimmer } from "@/components";
import {
  API_STATUS_TEXT,
  ATTACHMENT_ANALYSIS_STATUS_DELAY_MS,
  getAttachmentAnalysisStatusText,
  isAnswerNowSuppressedProgressText,
} from "@/lib";
import {
  buildAttachmentMention,
  parseAttachmentPaths,
  stripAttachmentMentions,
  attachmentFromPath,
  ChatInput,
  ImageArtifact,
  useChatScroll,
  useInputHeight,
  useChatWheel,
  useChatError,
  MessageList,
} from "@/features";
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

  // Refs
  const headerRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
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

  const { inputContainerRef, inputHeight } = useInputHeight({
    scrollContainerRef,
    wasAtBottomRef,
  });

  const revealTarget = app.searchOverlay.pendingReveal;
  const isRevealPendingForActiveChat =
    revealTarget?.chatId === app.chatHistory.activeSessionId;

  const { isSpinnerVisible } = useChatScroll({
    messages: app.chat.messages,
    chatId: app.chatHistory.activeSessionId,
    isNavigating: app.isNavigating,
    inputHeight,
    scrollContainerRef,
    wasAtBottomRef,
    suspendAutoScroll: isRevealPendingForActiveChat,
  });

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
    let finalInput = inputValue;
    if (app.attachments.length > 0) {
      const mentions = app.attachments
        .map((a) => buildAttachmentMention(a.path, a.name))
        .join("\n");
      finalInput = `${inputValue}\n\n${mentions}`.trim();
    }
    if (!finalInput.trim() || app.chat.isLoading) {
      return;
    }
    app.trackPendingPromptAttachmentAnalysis(app.attachments);
    app.chat.handleSend(finalInput, app.inputModel);
    setInputValue("");
    app.clearAttachments();
  }, [
    app.attachments,
    app.chat,
    app.clearAttachments,
    app.inputModel,
    app.trackPendingPromptAttachmentAnalysis,
    inputValue,
  ]);

  const handleCaptureToInput = async () => {
    try {
      await invoke("spawn_capture_to_input");
    } catch (err) {
      console.error("Failed to spawn capture to input:", err);
    }
  };

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

      const restoredText = stripAttachmentMentions(targetMessage.text);
      const restoredAttachments = parseAttachmentPaths(targetMessage.text).map(
        (path) => {
          const sourcePath = app.getAttachmentSourcePath(path) || undefined;
          const originalName = sourcePath ? getBaseName(sourcePath) : undefined;
          return attachmentFromPath(path, undefined, originalName, sourcePath);
        },
      );

      setInputValue(restoredText);
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

      const containerRect = container.getBoundingClientRect();
      const bubbleRect = bubble.getBoundingClientRect();
      const bubbleTopInContainer = bubbleRect.top - containerRect.top;
      const targetScrollTop =
        container.scrollTop +
        bubbleTopInContainer -
        container.clientHeight * 0.35;
      const maxScrollTop = Math.max(
        0,
        container.scrollHeight - container.clientHeight,
      );
      const clampedTop = Math.max(0, Math.min(targetScrollTop, maxScrollTop));
      container.scrollTo({ top: clampedTop, behavior: "smooth" });
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
    <div className={styles.chatContainer}>
      <div ref={headerRef} className={styles.headerContainer}>
        <ImageArtifact
          startupImage={app.system.startupImage}
          sessionLensUrl={app.sessionLensUrl}
          setSessionLensUrl={app.handleUpdateLensUrl}
          chatTitle={app.chatTitle}
          onDescribeEdits={async (desc) => app.chat.handleDescribeEdits(desc)}
          ocrData={app.ocrData}
          onUpdateOCRData={app.handleUpdateOCRData}
          onOpenSettings={app.system.openSettings}
          isVisible={true}
          scrollContainerRef={scrollContainerRef}
          chatId={app.chatHistory.activeSessionId}
          inputValue={app.imageInput}
          onInputChange={app.setImageInput}
          onToggleExpand={() => setIsImageExpanded(!isImageExpanded)}
          ocrEnabled={app.system.ocrEnabled}
          autoExpandOCR={app.system.autoExpandOCR}
          activeProfileId={app.system.activeProfile?.id || null}
          currentOcrModel={app.system.sessionOcrLanguage}
          onOcrModelChange={app.system.setSessionOcrLanguage}
          isOcrScanning={app.isOcrScanning}
          onOcrScanningChange={app.setIsOcrScanning}
          isExpanded={isImageExpanded}
          isNavigating={app.isNavigating}
        />
      </div>
      <div className={styles.contentColumn}>
        <div
          className={`${styles.container} ${!app.system.startupImage ? styles.noImage : ""}`}
          ref={scrollContainerRef}
          style={
            { "--input-height": `${inputHeight}px` } as React.CSSProperties
          }
        >
          <main style={{ paddingBottom: inputHeight + 10 }}>
            <div
              className={`${styles.contentInner} ${
                isSpinnerVisible
                  ? styles.contentOffsetUp
                  : styles.contentOffsetDown
              }`}
            >
              {parsedError && (
                <Dialog
                  isOpen={isErrorOpen}
                  variant="error"
                  title={parsedError.title}
                  message={parsedError.message}
                  actions={errorActions}
                />
              )}
              <Dialog
                isOpen={!!pendingUndoMessageId}
                type="UNDO_MESSAGE"
                onAction={handleUndoDialogAction}
              />

              {isSpinnerVisible ? (
                <div className={styles.spinnerContainer}>
                  <LoadingSpinner />
                </div>
              ) : (
                <>
                  {isImageProgressVisible && (
                    <div className={styles.imagePendingProgress}>
                      <div className={styles.imageProgressRow}>
                        <TextShimmer
                          text={API_STATUS_TEXT.ANALYZING_IMAGE}
                          compact={true}
                          duration={2}
                          spotWidth={30}
                          angle={90}
                          peakWidth={3}
                          bleedInner={8}
                          bleedOuter={30}
                          className={styles.imageProgressShimmer}
                        />
                        {showAnswerNow && (
                          <button
                            type="button"
                            className={styles.answerNowButton}
                            onClick={app.chat.handleAnswerNow}
                          >
                            {API_STATUS_TEXT.ANSWER_NOW_BUTTON}
                          </button>
                        )}
                      </div>

                      {visibleImageProgressText && (
                        <p
                          key={visibleImageProgressText}
                          className={styles.imageProgressText}
                          aria-live="polite"
                        >
                          {visibleImageProgressText}
                        </p>
                      )}
                    </div>
                  )}
                  <MessageList
                    messages={app.chat.messages}
                    pendingAssistantTurn={app.chat.pendingAssistantTurn}
                    pendingPromptAttachmentAnalysis={
                      app.pendingPromptAttachmentAnalysis
                    }
                    hideThinkingProgress={app.chat.isAnalyzing}
                    selectedModel={app.inputModel}
                    onAnswerNow={app.chat.handleAnswerNow}
                    onRetryMessage={handleRetryMessage}
                    onUndoMessage={handleRequestUndoMessage}
                    onSystemAction={app.handleSystemAction}
                  />
                </>
              )}
            </div>
          </main>
        </div>

        <div
          ref={inputContainerRef}
          className={styles.inputOverlay}
          style={{ pointerEvents: "none" }}
        >
          <div style={{ pointerEvents: "auto", width: "100%" }}>
            <ChatInput
              startupImage={app.system.startupImage}
              input={inputValue}
              onInputChange={setInputValue}
              onSend={handleSend}
              isLoading={app.chat.isLoading}
              isAiTyping={app.chat.isAiTyping}
              isStoppable={app.chat.isAnalyzing || app.chat.isGenerating}
              onStopGeneration={() => {
                app.chat.handleStopGeneration();
              }}
              selectedModel={app.inputModel}
              onModelChange={app.setInputModel}
              attachments={app.attachments}
              onAttachmentsChange={app.setAttachments}
              onCaptureToInput={handleCaptureToInput}
            />
          </div>
        </div>

        <InlineMenu
          menuRef={menuRef}
          sliderRef={sliderRef}
          page1Ref={page1Ref}
          page2Ref={page2Ref}
          pageFlatRef={pageFlatRef}
          onAction={handleAction}
          onSwitchPage={switchPage}
        />
      </div>
    </div>
  );
};
