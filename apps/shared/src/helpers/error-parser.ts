/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { BrainParsedError as ParsedError } from "../brain/provider";

export type { ParsedError };

const normalizeErrorMessage = (error: any): string => {
  if (typeof error === "string") return error;
  if (error?.message && typeof error.message === "string") {
    return error.message;
  }
  if (error?.statusText && typeof error.statusText === "string") {
    return error.statusText;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return "An unknown error occurred.";
  }
};

export const parseAppError = (error: any): ParsedError => {
  const message = normalizeErrorMessage(error);
  const searchStr = message.toLowerCase();

  if (
    searchStr.includes("ocr sidecar") ||
    searchStr.includes("paddleocr") ||
    searchStr.includes("ocr processing failed") ||
    searchStr.includes("failed to initialize paddleocr")
  ) {
    return {
      title: "OCR Error",
      message,
      actionType: "RETRY_ONLY",
    };
  }

  if (
    searchStr.includes("failed to download") ||
    (searchStr.includes("download") && searchStr.includes("model"))
  ) {
    return {
      title: "Model Download Failed",
      message,
      actionType: "RETRY_ONLY",
    };
  }

  if (searchStr.includes("cancelled")) {
    return {
      title: "Cancelled",
      message,
      actionType: "DISMISS_ONLY",
    };
  }

  return {
    title: "Error",
    message,
    actionType: "DISMISS_ONLY",
  };
};
