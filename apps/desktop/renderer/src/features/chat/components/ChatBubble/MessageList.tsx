/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Message, Citation, ToolStep } from "../../chat.types";
import { ChatBubble } from "./ChatBubble";
import { TextShimmer } from "@/components";
import { API_STATUS_TEXT, getProgressStatusText } from "@/lib";
import styles from "./MessageList.module.css";

interface MessageListProps {
  messages: Message[];
  streamingText: string;
  isGenerating: boolean;
  retryingMessageId?: string | null;
  stopRequested: boolean;
  selectedModel: string;
  isAnalyzing: boolean;
  isSearching?: boolean;
  toolStatus?: string | null;
  streamingToolSteps?: ToolStep[];
  streamingCitations?: Citation[];
  onStreamComplete?: () => void;
  onTypingChange?: (isTyping: boolean) => void;
  onStopGeneration?: (truncatedText?: string) => void;
  onStopRequestedChange?: (requested: boolean) => void;
  onAnswerNow?: () => void;
  onRetryMessage?: (messageId: string, modelId?: string) => void;
  onUndoMessage?: (messageId: string) => void;
  onSystemAction?: (actionId: string, value?: string) => void;
}

const MessageListComponent: React.FC<MessageListProps> = ({
  messages,
  streamingText,
  isGenerating,
  retryingMessageId,
  stopRequested,
  selectedModel,
  isAnalyzing,
  isSearching,
  toolStatus,
  streamingToolSteps = [],
  streamingCitations = [],
  onStreamComplete,
  onTypingChange,
  onStopGeneration,
  onStopRequestedChange,
  onAnswerNow,
  onRetryMessage,
  onUndoMessage,
  onSystemAction,
}) => {
  const retryIndex = retryingMessageId
    ? messages.findIndex((m) => m.id === retryingMessageId)
    : -1;

  const displayMessages =
    retryIndex !== -1 ? messages.slice(0, retryIndex + 1) : messages;
  const isProgressVisible = (isGenerating || isAnalyzing || isSearching) && !streamingText;
  const progressText = getProgressStatusText({
    toolStatus,
    isAnalyzing,
    isRetrying: !!retryingMessageId,
  });
  const hasRunningToolStep = streamingToolSteps.some(
    (step) => step.status === "running",
  );
  const hasPreStepSearchStatus =
    !!toolStatus && streamingToolSteps.length === 0 && !!isSearching;
  const showAnswerNow =
    !!onAnswerNow && (hasRunningToolStep || hasPreStepSearchStatus);

  return (
    <div className={styles.container}>
      {isProgressVisible && (
        <div className={styles.progressRow}>
          <TextShimmer text={progressText} />
          {showAnswerNow && (
            <button
              type="button"
              className={styles.answerNowButton}
              onClick={onAnswerNow}
            >
              {API_STATUS_TEXT.ANSWER_NOW_BUTTON}
            </button>
          )}
        </div>
      )}
      {streamingText && (
        <div className={styles.item}>
          <ChatBubble
            message={{
              id: "streaming-temp",
              role: "model",
              text: streamingText,
              timestamp: Date.now(),
              toolSteps: streamingToolSteps,
              citations: streamingCitations,
            }}
            isStreamed={true}
            onStreamComplete={onStreamComplete}
            onTypingChange={onTypingChange}
            stopRequested={stopRequested}
            onStopGeneration={(truncatedText) => {
              onStopRequestedChange?.(false);
              onStopGeneration?.(truncatedText);
            }}
          />
        </div>
      )}

      {displayMessages
        .slice()
        .reverse()
        .map((msg, index) => {
          const isLatestModel =
            msg.role === "model" && index === 0 && !msg.alreadyStreamed;

          return (
            <div key={msg.id} className={styles.item}>
              <ChatBubble
                message={msg}
                isStreamed={isLatestModel}
                onStreamComplete={isLatestModel ? onStreamComplete : undefined}
                onTypingChange={isLatestModel ? onTypingChange : undefined}
                stopRequested={isLatestModel ? stopRequested : undefined}
                onStopGeneration={
                  isLatestModel
                    ? (truncatedText) => {
                        onStopRequestedChange?.(false);
                        onStopGeneration?.(truncatedText);
                      }
                    : undefined
                }
                onRetry={
                  msg.role !== "user" && onRetryMessage
                    ? () => onRetryMessage(msg.id, selectedModel)
                    : undefined
                }
                onUndo={
                  msg.role === "user" && onUndoMessage
                    ? () => onUndoMessage(msg.id)
                    : undefined
                }
                onAction={msg.role === "system" ? onSystemAction : undefined}
              />
            </div>
          );
        })}
    </div>
  );
};

export const MessageList = React.memo(
  MessageListComponent,
  (prevProps, nextProps) => {
    return (
      prevProps.messages === nextProps.messages &&
      prevProps.streamingText === nextProps.streamingText &&
      prevProps.isGenerating === nextProps.isGenerating &&
      prevProps.retryingMessageId === nextProps.retryingMessageId &&
      prevProps.stopRequested === nextProps.stopRequested &&
      prevProps.selectedModel === nextProps.selectedModel &&
      prevProps.isAnalyzing === nextProps.isAnalyzing &&
      prevProps.isSearching === nextProps.isSearching &&
      prevProps.toolStatus === nextProps.toolStatus &&
      prevProps.streamingToolSteps === nextProps.streamingToolSteps &&
      prevProps.streamingCitations === nextProps.streamingCitations
    );
  },
);
