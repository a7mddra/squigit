/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type { UserPreferences } from "./app-settings.ts";
export {
  getDefaultPreferences,
  hasAgreedFlag,
  setAgreedFlag,
  hasPreferencesFile,
  loadPreferences,
  savePreferences,
} from "./app-settings.ts";

export type {
  ChatMetadata,
  ChatCitation,
  ChatToolStep,
  ChatMessage,
  OcrRegion,
  OcrFrame,
  ChatData,
  ChatSearchResult,
  StoredImage,
  ImageResult,
} from "./chat-storage.ts";

export {
  AUTO_OCR_DISABLED_MODEL_ID,
  storeImageBytes,
  storeImageFromPath,
  getImagePath,
  createChat,
  loadChat,
  listChats,
  searchChats,
  deleteChat,
  updateChatMetadata,
  appendChatMessage,
  overwriteChatMessages,
  saveOcrData,
  getOcrData,
  getOcrFrame,
  initOcrFrame,
  cancelOcrJob,
  saveImgbbUrl,
  getImgbbUrl,
  saveRollingSummary,
  saveImageTone,
  saveImageBrief,
  groupChatsByDate,
} from "./chat-storage.ts";

export {
  MODEL_IDS,
  MODELS,
  DEFAULT_MODEL_ID,
  isSupportedModelId,
  resolveModelId,
  DEFAULT_OCR_MODEL_ID,
  SUPPORTED_OCR_MODEL_IDS,
  isSupportedOcrModelId,
  resolveOcrModelId,
} from "./models-config.ts";

export {
  APP_NAME,
  DEFAULT_THEME,
  DEFAULT_PROMPT,
  PREFERENCES_FILE_NAME,
  DEFAULT_CAPTURE_TYPE,
  DEFAULT_ACTIVE_ACCOUNT,
  DEFAULT_PREFERENCES,
} from "./defaults.ts";
