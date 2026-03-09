/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Message } from "../../chat.types";
import { ChatBubble } from "./ChatBubble";
import { TextShimmer } from "@/components";
import styles from "./MessageList.module.css";

interface MessageListProps {
  messages: Message[];
  streamingText: string;
  isGenerating: boolean;
  retryingMessageId?: string | null;
  stopRequested: boolean;
  selectedModel: string;
  isAnalyzing: boolean;
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
  isGenerating,
  retryingMessageId,
  stopRequested,
  selectedModel,
  isAnalyzing,
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
      {isGenerating && !retryingMessageId && (
        <TextShimmer text="Planning next moves" />
      )}
      {streamingText && (
        <div className={styles.item}>
          <ChatBubble
            message={{
              id: "streaming-temp",
              role: "model",
              text: streamingText,
              timestamp: Date.now(),
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

          if (isAnalyzing && msg.id === retryingMessageId) return null;

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
                isRetrying={msg.id === retryingMessageId}
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
      prevProps.isAnalyzing === nextProps.isAnalyzing
    );
  },
);
