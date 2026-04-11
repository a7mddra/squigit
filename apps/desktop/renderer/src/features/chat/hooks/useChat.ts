/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useBrainSession } from "@/hooks";
import { Message } from "@/features";

export const useChat = ({
  apiKey,
  currentModel,
  startupImage,
  prompt,
  setCurrentModel,
  enabled,
  onMessage,
  onOverwriteMessages,
  chatId,
  chatTitle,
  onMissingApiKey,
  onTitleGenerated,
  generateTitle,
  userName,
  userEmail,
}: {
  apiKey: string;
  currentModel: string;
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
    fromHistory?: boolean;
  } | null;
  prompt: string;
  setCurrentModel: (model: string) => void;
  enabled: boolean;
  onMessage?: (message: Message, chatId: string) => void;
  onOverwriteMessages?: (messages: Message[]) => void;
  chatId: string | null;
  chatTitle: string;
  onMissingApiKey?: () => void;
  onTitleGenerated?: (title: string) => void;
  generateTitle?: (text: string) => Promise<string>;
  userName?: string;
  userEmail?: string;
}) => {
  return useBrainSession({
    apiKey,
    currentModel,
    startupImage,
    prompt,
    setCurrentModel,
    enabled,
    onMissingApiKey,
    onMessage,
    onOverwriteMessages,
    chatId,
    chatTitle,
    onTitleGenerated,
    generateTitle,
    userName,
    userEmail,
  });
};
