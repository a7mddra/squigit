/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Message, Citation, ToolStep } from "../../chat.types";
import { ChatBubble } from "./ChatBubble";
import styles from "./MessageList.module.css";

interface MessageListProps {
  messages: Message[];
  streamingText: string;
  retryingMessageId?: string | null;
  stopRequested: boolean;
  selectedModel: string;
  streamingToolSteps?: ToolStep[];
  streamingCitations?: Citation[];
  onStreamComplete?: () => void;
  onTypingChange?: (isTyping: boolean) => void;
  onStopGeneration?: (truncatedText?: string) => void;
  onStopRequestedChange?: (requested: boolean) => void;
  onRetryMessage?: (messageId: string, modelId?: string) => void;
  onUndoMessage?: (messageId: string) => void;
  onSystemAction?: (actionId: string, value?: string) => void;
}

const MessageListComponent: React.FC<MessageListProps> = ({
  messages,
  streamingText,
  retryingMessageId,
  stopRequested,
  selectedModel,
  streamingToolSteps = [],
  streamingCitations = [],
  onStreamComplete,
  onTypingChange,
  onStopGeneration,
  onStopRequestedChange,
  onRetryMessage,
  onUndoMessage,
  onSystemAction,
}) => {
  const retryIndex = retryingMessageId
    ? messages.findIndex((m) => m.id === retryingMessageId)
    : -1;

  const displayMessages =
    retryIndex !== -1 ? messages.slice(0, retryIndex + 1) : messages;

  return (
    <div className={styles.container}>
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
      prevProps.retryingMessageId === nextProps.retryingMessageId &&
      prevProps.stopRequested === nextProps.stopRequested &&
      prevProps.selectedModel === nextProps.selectedModel &&
      prevProps.streamingToolSteps === nextProps.streamingToolSteps &&
      prevProps.streamingCitations === nextProps.streamingCitations
    );
  },
);
