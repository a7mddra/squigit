/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";

export interface StreamGeminiChatInput extends Record<string, unknown> {
  apiKey: string;
  model: string;
  isInitialTurn: boolean;
  imagePath: string | null;
  imageDescription: string | null;
  userFirstMsg: string | null;
  historyLog: string | null;
  rollingSummary: string | null;
  userMessage: string;
  channelId: string;
  chatId: string | null;
  userName?: string | null;
  userEmail?: string | null;
  userInstruction?: string | null;
  imageBrief?: string | null;
}

export function streamGeminiChat(input: StreamGeminiChatInput): Promise<void> {
  return invoke("stream_gemini_chat_v2", input);
}

export function generateGeminiImageBrief(
  apiKey: string,
  imagePath: string,
): Promise<string> {
  return invoke<string>("generate_image_brief", { apiKey, imagePath });
}

export function generateGeminiChatTitle(
  apiKey: string,
  model: string,
  promptContext: string,
): Promise<string> {
  return invoke<string>("generate_chat_title", { apiKey, model, promptContext });
}

export function compressGeminiConversation(
  apiKey: string,
  imageBrief: string,
  historyToCompress: string,
): Promise<string> {
  return invoke<string>("compress_conversation", {
    apiKey,
    imageBrief,
    historyToCompress,
  });
}

export function persistRollingSummary(
  chatId: string,
  summary: string,
): Promise<void> {
  return invoke("save_rolling_summary", { chatId, summary });
}

export function cancelGeminiRequest(channelId: string | null): Promise<void> {
  return invoke("cancel_gemini_request", { channelId });
}

export function requestGeminiQuickAnswer(channelId: string): Promise<void> {
  return invoke("quick_answer_gemini_request", { channelId });
}
