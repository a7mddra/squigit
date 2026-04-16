/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { getProviderPort, type StreamGeminiChatInput } from "../../../ports/provider";
import type { ProviderStreamEvent } from "../../engine/types";

export type { StreamGeminiChatInput };

export function streamGeminiChat(input: StreamGeminiChatInput): Promise<void> {
  return getProviderPort().streamChat(input);
}

export function generateGeminiImageBrief(
  apiKey: string,
  imagePath: string,
  model?: string,
): Promise<string> {
  return getProviderPort().generateImageBrief(apiKey, imagePath, model);
}

export function generateGeminiChatTitle(
  apiKey: string,
  model: string,
  promptContext: string,
): Promise<string> {
  return getProviderPort().generateChatTitle(apiKey, model, promptContext);
}

export function compressGeminiConversation(
  apiKey: string,
  imageBrief: string,
  historyToCompress: string,
): Promise<string> {
  return getProviderPort().compressConversation(
    apiKey,
    imageBrief,
    historyToCompress,
  );
}

export function persistRollingSummary(
  chatId: string,
  summary: string,
): Promise<void> {
  return getProviderPort().persistRollingSummary(chatId, summary);
}

export function cancelGeminiRequest(channelId: string | null): Promise<void> {
  return getProviderPort().cancelRequest(channelId);
}

export function requestGeminiQuickAnswer(channelId: string): Promise<void> {
  return getProviderPort().requestQuickAnswer(channelId);
}

export function listenGeminiStream(
  channelId: string,
  onEvent: (event: ProviderStreamEvent) => void,
): Promise<() => void> {
  return getProviderPort().listenToStream(channelId, onEvent);
}
