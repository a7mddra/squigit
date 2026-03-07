/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { KeyboardEvent } from "react";
import type { Attachment } from "@/features";

export type ChatSubmitHandler = () => void;

export type ChatModelSelectHandler = (model: string) => void;

export interface ChatInputProps {
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
  } | null;
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
}
