/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { KeyboardEvent, RefObject } from "react";
import type { Attachment } from "@squigit/core/brain/attachments";

export type ChatSubmitHandler = () => void;

export type ChatModelSelectHandler = (model: string) => void;

export interface ChatInputProps {
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
  } | null;
  forceVisible?: boolean;
  isNavigating?: boolean;
  input: string;
  onInputChange: (value: string) => void;
  onSend: ChatSubmitHandler;
  isLoading: boolean;
  isAiTyping?: boolean;
  isStoppable?: boolean;
  onStopGeneration?: () => void;
  placeholder?: string;
  variant?: "default" | "transparent";
  className?: string;
  onKeyDown?: (e: KeyboardEvent<HTMLTextAreaElement>) => void;
  selectedModel: string;
  onModelChange: ChatModelSelectHandler;
  attachments: Attachment[];
  onAttachmentsChange: (attachments: Attachment[]) => void;
  onCaptureToInput?: () => void;
  onPreviewAttachment?: (attachment: Attachment) => void;
  rememberAttachmentSourcePath?: (
    storedPath: string,
    sourcePath: string,
  ) => void;
  showScrollToBottomButton?: boolean;
  keepScrollToBottomButtonMounted?: boolean;
  scrollToBottomButtonRef?: RefObject<HTMLButtonElement | null>;
  onScrollToBottom?: () => void;
}
