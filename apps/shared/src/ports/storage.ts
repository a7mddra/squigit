/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type {
  ChatData,
  ChatMessage,
  ChatMetadata,
  ChatSearchResult,
  OcrFrame,
  OcrRegion,
  StoredImage,
} from "../config/chat-storage";

export interface StoragePort {
  storeImageBytes(bytes: number[]): Promise<StoredImage>;
  storeImageFromPath(path: string): Promise<StoredImage>;
  getImagePath(hash: string): Promise<string>;
  createChat(
    title: string,
    imageHash: string,
    ocrLang?: string | null,
  ): Promise<ChatMetadata>;
  loadChat(chatId: string): Promise<ChatData>;
  listChats(): Promise<ChatMetadata[]>;
  searchChats(query: string, limit: number): Promise<ChatSearchResult[]>;
  deleteChat(chatId: string): Promise<void>;
  updateChatMetadata(metadata: ChatMetadata): Promise<void>;
  appendChatMessage(
    chatId: string,
    role: "user" | "assistant",
    content: string,
  ): Promise<void>;
  overwriteChatMessages(chatId: string, messages: ChatMessage[]): Promise<void>;
  saveOcrData(chatId: string, modelId: string, ocrData: OcrRegion[]): Promise<void>;
  getOcrData(chatId: string, modelId: string): Promise<OcrRegion[] | null>;
  getOcrFrame(chatId: string): Promise<OcrFrame>;
  initOcrFrame(chatId: string, modelIds: string[]): Promise<void>;
  cancelOcrJob(): Promise<void>;
  saveImgbbUrl(chatId: string, url: string): Promise<void>;
  getImgbbUrl(chatId: string): Promise<string | null>;
  saveRollingSummary(chatId: string, summary: string): Promise<void>;
  saveImageTone(chatId: string, tone: string): Promise<void>;
  saveImageBrief(chatId: string, brief: string): Promise<void>;
}

let storagePort: StoragePort | null = null;

export function setStoragePort(port: StoragePort): void {
  storagePort = port;
}

export function getStoragePort(): StoragePort {
  if (!storagePort) {
    throw new Error("StoragePort is not initialized");
  }

  return storagePort;
}
