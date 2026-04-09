/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { type RefObject } from "react";
import { InlineMenu, LoadingSpinner } from "@/components";
import type { Attachment, OcrFrame } from "@/lib";
import { ChatInput, ImageArtifact } from "@/features";
import styles from "./Chat.module.css";

type StartupImage = {
  path: string;
  mimeType: string;
  imageId: string;
  tone?: string;
} | null;

interface ChatLayoutProps {
  headerRef: RefObject<HTMLDivElement | null>;
  scrollContainerRef: RefObject<HTMLDivElement | null>;
  bottomAnchorRef: RefObject<HTMLDivElement | null>;
  inputContainerRef: RefObject<HTMLDivElement | null>;
  inputHeight: number;
  visibleStartupImage: StartupImage;
  showArtifactPlaceholder: boolean;
  showLoadingState: boolean;
  showHistoryLoadSpinner?: boolean;
  isContentMounted: boolean;
  isNavigating: boolean;
  isImageExpanded: boolean;
  onToggleImageExpanded: () => void;
  sessionLensUrl: string | null;
  setSessionLensUrl: (url: string | null) => void;
  chatTitle: string;
  onDescribeEdits: (description: string) => void | Promise<void>;
  ocrData: OcrFrame;
  onUpdateOCRData: (
    chatId: string | null,
    modelId: string,
    data: { text: string; box: number[][] }[],
  ) => void;
  onOpenSettings: (section: any) => void;
  chatId: string | null;
  imageInput: string;
  onImageInputChange: (value: string) => void;
  ocrEnabled: boolean;
  autoExpandOCR?: boolean;
  activeProfileId: string | null;
  currentOcrModel: string;
  onOcrModelChange: (model: string) => void;
  isOcrScanning: boolean;
  onOcrScanningChange: (scanning: boolean) => void;
  inputValue: string;
  onInputChange: (value: string) => void;
  onSend: () => void;
  isChatLoading: boolean;
  isAiTyping: boolean;
  isStoppable: boolean;
  onStopGeneration: () => void;
  selectedModel: string;
  onModelChange: (model: string) => void;
  attachments: Attachment[];
  onAttachmentsChange: (attachments: Attachment[]) => void;
  onCaptureToInput: () => void | Promise<void>;
  onPreviewAttachment: (attachment: Attachment) => void | Promise<void>;
  rememberAttachmentSourcePath: (
    storedPath: string,
    sourcePath: string,
  ) => void;
  showScrollToBottomButton: boolean;
  keepScrollToBottomButtonMounted: boolean;
  scrollToBottomButtonRef: RefObject<HTMLButtonElement | null>;
  onScrollToBottom: () => void;
  menuRef: RefObject<HTMLDivElement | null>;
  sliderRef: RefObject<HTMLDivElement | null>;
  page1Ref: RefObject<HTMLDivElement | null>;
  page2Ref: RefObject<HTMLDivElement | null>;
  pageFlatRef: RefObject<HTMLDivElement | null>;
  onInlineMenuAction: (action: string) => void;
  onInlineMenuSwitchPage: (page: number) => void;
  children: React.ReactNode;
}

const ChatLayoutComponent: React.FC<ChatLayoutProps> = ({
  headerRef,
  scrollContainerRef,
  bottomAnchorRef,
  inputContainerRef,
  inputHeight,
  visibleStartupImage,
  showArtifactPlaceholder,
  showLoadingState,
  showHistoryLoadSpinner = false,
  isContentMounted,
  isNavigating,
  isImageExpanded,
  onToggleImageExpanded,
  sessionLensUrl,
  setSessionLensUrl,
  chatTitle,
  onDescribeEdits,
  ocrData,
  onUpdateOCRData,
  onOpenSettings,
  chatId,
  imageInput,
  onImageInputChange,
  ocrEnabled,
  autoExpandOCR,
  activeProfileId,
  currentOcrModel,
  onOcrModelChange,
  isOcrScanning,
  onOcrScanningChange,
  inputValue,
  onInputChange,
  onSend,
  isChatLoading,
  isAiTyping,
  isStoppable,
  onStopGeneration,
  selectedModel,
  onModelChange,
  attachments,
  onAttachmentsChange,
  onCaptureToInput,
  onPreviewAttachment,
  rememberAttachmentSourcePath,
  showScrollToBottomButton,
  keepScrollToBottomButtonMounted,
  scrollToBottomButtonRef,
  onScrollToBottom,
  menuRef,
  sliderRef,
  page1Ref,
  page2Ref,
  pageFlatRef,
  onInlineMenuAction,
  onInlineMenuSwitchPage,
  children,
}) => {
  const hasArtifactShell = !!visibleStartupImage || showArtifactPlaceholder;

  return (
    <div className={styles.chatContainer}>
      <div className={styles.overlayLayer}>
        <div ref={headerRef} className={styles.headerContainer}>
          {visibleStartupImage ? (
            <ImageArtifact
              startupImage={visibleStartupImage}
              sessionLensUrl={sessionLensUrl}
              setSessionLensUrl={setSessionLensUrl}
              chatTitle={chatTitle}
              onDescribeEdits={onDescribeEdits}
              ocrData={ocrData}
              onUpdateOCRData={onUpdateOCRData}
              onOpenSettings={onOpenSettings}
              isVisible={true}
              scrollContainerRef={scrollContainerRef}
              chatId={chatId}
              inputValue={imageInput}
              onInputChange={onImageInputChange}
              onToggleExpand={onToggleImageExpanded}
              ocrEnabled={ocrEnabled}
              autoExpandOCR={autoExpandOCR}
              activeProfileId={activeProfileId}
              currentOcrModel={currentOcrModel}
              onOcrModelChange={onOcrModelChange}
              isOcrScanning={isOcrScanning}
              onOcrScanningChange={onOcrScanningChange}
              isExpanded={isImageExpanded}
              isNavigating={isNavigating}
            />
          ) : showArtifactPlaceholder ? (
            <div className={styles.artifactPlaceholder} aria-hidden="true" />
          ) : null}
        </div>

        <div
          ref={inputContainerRef}
          className={styles.inputOverlay}
          style={{ pointerEvents: "none" }}
        >
          <div style={{ pointerEvents: "auto", width: "100%" }}>
            <ChatInput
              startupImage={visibleStartupImage}
              forceVisible={isNavigating}
              isNavigating={isNavigating}
              input={inputValue}
              onInputChange={onInputChange}
              onSend={onSend}
              isLoading={isChatLoading}
              isAiTyping={isAiTyping}
              isStoppable={isStoppable}
              onStopGeneration={onStopGeneration}
              selectedModel={selectedModel}
              onModelChange={onModelChange}
              attachments={attachments}
              onAttachmentsChange={onAttachmentsChange}
              onCaptureToInput={onCaptureToInput}
              onPreviewAttachment={onPreviewAttachment}
              rememberAttachmentSourcePath={rememberAttachmentSourcePath}
              showScrollToBottomButton={showScrollToBottomButton}
              keepScrollToBottomButtonMounted={keepScrollToBottomButtonMounted}
              scrollToBottomButtonRef={scrollToBottomButtonRef}
              onScrollToBottom={onScrollToBottom}
            />
          </div>
        </div>
      </div>
      <div className={styles.contentColumn}>
        <div
          className={`${styles.container} ${!hasArtifactShell ? styles.noImage : ""}`}
          ref={scrollContainerRef}
          style={
            { "--input-height": `${inputHeight}px` } as React.CSSProperties
          }
        >
          <main className={styles.scrollContent}>
            {showHistoryLoadSpinner && (
              <div
                className={styles.historyLoadSpinner}
                role="status"
                aria-live="polite"
                aria-label="Loading older messages"
              >
                <LoadingSpinner />
              </div>
            )}
            <div className={styles.contentViewport}>
              <div className={styles.contentStage}>
                <div
                  className={`${styles.contentInner} ${
                    isContentMounted
                      ? styles.contentOffsetDown
                      : styles.contentOffsetUp
                  } ${
                    isContentMounted && !showLoadingState
                      ? styles.contentVisible
                      : styles.contentHidden
                  }`}
                  aria-hidden={!isContentMounted || showLoadingState}
                >
                  {isContentMounted ? children : null}
                </div>
              </div>
            </div>
          </main>
          <div
            className={styles.scrollBottomSpacer}
            style={{ height: inputHeight + 10 }}
            aria-hidden="true"
          />
          <div
            ref={bottomAnchorRef}
            className={styles.scrollBottomAnchor}
            aria-hidden="true"
          />
        </div>

        <div
          className={`${styles.loadingOverlay} ${
            showLoadingState ? styles.loadingVisible : styles.loadingHidden
          }`}
          role={showLoadingState ? "status" : undefined}
          aria-live={showLoadingState ? "polite" : undefined}
          aria-label={showLoadingState ? "Loading chat" : undefined}
          aria-hidden={!showLoadingState}
        >
          <div className={styles.loadingOverlaySpinner} aria-hidden="true">
            <LoadingSpinner />
          </div>
        </div>

        <InlineMenu
          menuRef={menuRef}
          sliderRef={sliderRef}
          page1Ref={page1Ref}
          page2Ref={page2Ref}
          pageFlatRef={pageFlatRef}
          onAction={onInlineMenuAction}
          onSwitchPage={onInlineMenuSwitchPage}
        />
      </div>
    </div>
  );
};

export const ChatLayout = React.memo(ChatLayoutComponent);
