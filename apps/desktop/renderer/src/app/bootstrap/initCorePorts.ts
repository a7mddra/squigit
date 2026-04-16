/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  BaseDirectory,
  exists,
  mkdir,
  readTextFile,
  writeTextFile,
} from "@tauri-apps/plugin-fs";
import type { ProviderStreamEvent } from "@squigit/core/brain/engine/types";
import {
  setPreferencesPort,
  setProviderPort,
  setStoragePort,
  setSystemPort,
  type StreamGeminiChatInput,
} from "@squigit/core/ports";

let initialized = false;

export function initializeCorePorts(): void {
  if (initialized) return;

  setProviderPort({
    streamChat: (input: StreamGeminiChatInput) => invoke("stream_chat", input),
    generateImageBrief: (apiKey: string, imagePath: string, model?: string) =>
      invoke<string>("generate_image_brief", { apiKey, imagePath, model }),
    generateChatTitle: (apiKey: string, model: string, promptContext: string) =>
      invoke<string>("generate_chat_title", { apiKey, model, promptContext }),
    compressConversation: (
      apiKey: string,
      imageBrief: string,
      historyToCompress: string,
    ) =>
      invoke<string>("compress_conversation", {
        apiKey,
        imageBrief,
        historyToCompress,
      }),
    persistRollingSummary: (chatId: string, summary: string) =>
      invoke("save_rolling_summary", { chatId, summary }),
    cancelRequest: (channelId: string | null) =>
      invoke("cancel_request", { channelId }),
    requestQuickAnswer: (channelId: string) =>
      invoke("quick_answer_request", { channelId }),
    listenToStream: async (
      channelId: string,
      onEvent: (event: ProviderStreamEvent) => void,
    ) => {
      const unlisten = await listen<ProviderStreamEvent>(channelId, (event) => {
        onEvent(event.payload);
      });
      return () => {
        unlisten();
      };
    },
  });

  setStoragePort({
    storeImageBytes: (bytes: number[]) => invoke("store_image_bytes", { bytes }),
    storeImageFromPath: (path: string) => invoke("store_image_from_path", { path }),
    getImagePath: (hash: string) => invoke("get_image_path", { hash }),
    createChat: (title: string, imageHash: string, ocrLang?: string | null) =>
      invoke("create_chat", { title, imageHash, ocrLang }),
    loadChat: (chatId: string) => invoke("load_chat", { chatId }),
    listChats: () => invoke("list_chats"),
    searchChats: (query: string, limit: number) =>
      invoke("search_chats", { query, limit }),
    deleteChat: (chatId: string) => invoke("delete_chat", { chatId }),
    updateChatMetadata: (metadata) => invoke("update_chat_metadata", { metadata }),
    appendChatMessage: (
      chatId: string,
      role: "user" | "assistant",
      content: string,
    ) => invoke("append_chat_message", { chatId, role, content }),
    overwriteChatMessages: (chatId, messages) =>
      invoke("overwrite_chat_messages", { chatId, messages }),
    saveOcrData: (chatId, modelId, ocrData) =>
      invoke("save_ocr_data", { chatId, modelId, ocrData }),
    getOcrData: (chatId, modelId) => invoke("get_ocr_data", { chatId, modelId }),
    getOcrFrame: (chatId) => invoke("get_ocr_frame", { chatId }),
    initOcrFrame: (chatId, modelIds) =>
      invoke("init_ocr_frame", { chatId, modelIds }),
    cancelOcrJob: () => invoke("cancel_ocr_job"),
    saveImgbbUrl: (chatId, url) => invoke("save_imgbb_url", { chatId, url }),
    getImgbbUrl: (chatId) => invoke("get_imgbb_url", { chatId }),
    saveRollingSummary: (chatId, summary) =>
      invoke("save_rolling_summary", { chatId, summary }),
    saveImageTone: (chatId, tone) => invoke("save_image_tone", { chatId, tone }),
    saveImageBrief: (chatId, brief) => invoke("save_image_brief", { chatId, brief }),
  });

  setPreferencesPort({
    hasAgreedFlag: () => invoke<boolean>("has_agreed_flag"),
    setAgreedFlag: () => invoke("set_agreed_flag"),
    hasPreferencesFile: (fileName: string) =>
      exists(fileName, { baseDir: BaseDirectory.AppConfig }),
    readPreferencesFile: (fileName: string) =>
      readTextFile(fileName, { baseDir: BaseDirectory.AppConfig }),
    writePreferencesFile: async (fileName: string, content: string) => {
      await mkdir("", { baseDir: BaseDirectory.AppConfig, recursive: true });
      await writeTextFile(fileName, content, { baseDir: BaseDirectory.AppConfig });
    },
  });

  setSystemPort({
    openExternalUrl: (url: string) => invoke("open_external_url", { url }),
    deleteTempFile: (path: string) => invoke("delete_temp_file", { path }),
    getApiKey: (provider, profileId) =>
      invoke<string>("get_api_key", { provider, profileId }),
    uploadImageToImgBB: (imagePath: string, apiKey: string) =>
      invoke<string>("upload_image_to_imgbb", { imagePath, apiKey }),
    closeImgbbWindow: () => invoke("close_imgbb_window"),
    listenToSystemEvent: async <TPayload>(
      eventName: string,
      onEvent: (payload: TPayload) => void,
    ) => {
      const unlisten = await listen<TPayload>(eventName, (event) => {
        onEvent(event.payload);
      });
      return () => {
        unlisten();
      };
    },
  });

  initialized = true;
}
