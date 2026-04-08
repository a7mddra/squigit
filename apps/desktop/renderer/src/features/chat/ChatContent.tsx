/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Dialog, TextShimmer } from "@/components";
import {
  API_STATUS_TEXT,
  type AttachmentAnalysisCounts,
} from "@/lib";
import { MessageList } from "@/features";
import type { Message, PendingAssistantTurn } from "./chat.types";
import styles from "./Chat.module.css";

interface ChatContentProps {
  parsedError: { title: string; message: string } | null;
  isErrorOpen: boolean;
  errorActions: any[];
  pendingUndoMessageId: string | null;
  onUndoDialogAction: (actionKey: string) => void;
  isImageProgressVisible: boolean;
  showQuickAnswer: boolean;
  visibleImageProgressText: string | null;
  onQuickAnswer?: () => void;
  messages: Message[];
  pendingAssistantTurn?: PendingAssistantTurn | null;
  pendingPromptAttachmentAnalysis?: AttachmentAnalysisCounts | null;
  hideThinkingProgress?: boolean;
  selectedModel: string;
  onRetryMessage?: (messageId: string, modelId?: string) => void;
  onUndoMessage?: (messageId: string) => void;
  onSystemAction?: (actionId: string, value?: string) => void;
}

const ChatContentComponent: React.FC<ChatContentProps> = ({
  parsedError,
  isErrorOpen,
  errorActions,
  pendingUndoMessageId,
  onUndoDialogAction,
  isImageProgressVisible,
  showQuickAnswer,
  visibleImageProgressText,
  onQuickAnswer,
  messages,
  pendingAssistantTurn,
  pendingPromptAttachmentAnalysis,
  hideThinkingProgress = false,
  selectedModel,
  onRetryMessage,
  onUndoMessage,
  onSystemAction,
}) => {
  return (
    <>
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
        onAction={onUndoDialogAction}
      />

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
            {showQuickAnswer && (
              <button
                type="button"
                className={styles.quickAnswerButton}
                onClick={onQuickAnswer}
              >
                {API_STATUS_TEXT.QUICK_ANSWER_BUTTON}
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
        messages={messages}
        pendingAssistantTurn={pendingAssistantTurn}
        pendingPromptAttachmentAnalysis={pendingPromptAttachmentAnalysis}
        hideThinkingProgress={hideThinkingProgress}
        selectedModel={selectedModel}
        onQuickAnswer={onQuickAnswer}
        onRetryMessage={onRetryMessage}
        onUndoMessage={onUndoMessage}
        onSystemAction={onSystemAction}
      />
    </>
  );
};

export const ChatContent = React.memo(
  ChatContentComponent,
  (prevProps, nextProps) => {
    const areErrorActionsStable =
      !prevProps.parsedError && !nextProps.parsedError
        ? true
        : prevProps.errorActions === nextProps.errorActions;

    return (
      prevProps.parsedError === nextProps.parsedError &&
      prevProps.isErrorOpen === nextProps.isErrorOpen &&
      areErrorActionsStable &&
      prevProps.pendingUndoMessageId === nextProps.pendingUndoMessageId &&
      prevProps.onUndoDialogAction === nextProps.onUndoDialogAction &&
      prevProps.isImageProgressVisible === nextProps.isImageProgressVisible &&
      prevProps.showQuickAnswer === nextProps.showQuickAnswer &&
      prevProps.visibleImageProgressText ===
        nextProps.visibleImageProgressText &&
      prevProps.onQuickAnswer === nextProps.onQuickAnswer &&
      prevProps.messages === nextProps.messages &&
      prevProps.pendingAssistantTurn === nextProps.pendingAssistantTurn &&
      prevProps.pendingPromptAttachmentAnalysis ===
        nextProps.pendingPromptAttachmentAnalysis &&
      prevProps.hideThinkingProgress === nextProps.hideThinkingProgress &&
      prevProps.selectedModel === nextProps.selectedModel &&
      prevProps.onRetryMessage === nextProps.onRetryMessage &&
      prevProps.onUndoMessage === nextProps.onUndoMessage &&
      prevProps.onSystemAction === nextProps.onSystemAction
    );
  },
);
