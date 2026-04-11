/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { Message } from "@/features";
import type { BrainParsedError } from "../provider";

export interface BrainStartupImage {
  path: string;
  mimeType: string;
  imageId: string;
  fromHistory?: boolean;
  tone?: string;
}

export interface ProviderPart {
  text?: string;
  inlineData?: {
    mimeType: string;
    data: string;
  };
}

export interface ProviderContent {
  role: string;
  parts: ProviderPart[];
}

export interface ProviderTokenEvent {
  type: "token";
  token: string;
}

export interface ProviderResetEvent {
  type: "reset";
}

export interface ProviderToolStatusEvent {
  type: "tool_status";
  message: string;
}

export interface ProviderToolStartEvent {
  type: "tool_start";
  id: string;
  name: string;
  args: Record<string, unknown>;
  message: string;
}

export interface ProviderToolEndEvent {
  type: "tool_end";
  id: string;
  name: string;
  status: "done" | "error" | string;
  result: Record<string, unknown>;
  message: string;
}

export type ProviderStreamEvent =
  | ProviderTokenEvent
  | ProviderResetEvent
  | ProviderToolStatusEvent
  | ProviderToolStartEvent
  | ProviderToolEndEvent;

export interface BrainConversationEntry {
  role: string;
  content: string;
}

export interface BrainSessionSnapshot {
  imageDescription: string | null;
  userFirstMsg: string | null;
  conversationHistory: BrainConversationEntry[];
  imageBrief: string | null;
  conversationSummary: string | null;
  storedImagePath: string | null;
  currentModelId: string;
}

export interface BrainLifecycleState {
  messages: Message[];
  streamingText: string;
  firstResponseId: string | null;
}

export interface BrainEngineHandle {
  startSession: (
    key: string,
    modelId: string,
    imgData: BrainStartupImage | null,
    isRetry?: boolean,
  ) => Promise<void>;
  handleSend: (userText: string, modelId?: string) => Promise<void>;
  handleRetrySend: () => Promise<void>;
  handleRetryMessage: (messageId: string, modelId?: string) => Promise<void>;
  handleUndoMessage: (messageId: string) => void;
  handleDescribeEdits: (editDescription: string) => Promise<void>;
  handleStopGeneration: () => void;
  handleQuickAnswer: () => Promise<void>;
  handleStreamComplete: () => void;
  cleanupAbortController: () => void;
}

export type { BrainParsedError };
