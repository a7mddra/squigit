/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { DialogVariant } from "@/components";

export interface DialogContent {
  title?: string;
  message: string;
  variant: DialogVariant;
  actions: {
    label: string;
    variant?: "primary" | "secondary" | "danger";
    actionKey: string;
    disabled?: boolean;
  }[];
}

export const getDialogs = (appName: string): Record<string, DialogContent> => ({
  CAPTURE_PERMISSION_DENIED: {
    title: "Screen Capture Denied",
    message: `${appName} needs permission to record your screen.\n\nPlease enable Screen Recording for ${appName} (or your terminal) in your OS Privacy/Security Settings, then try again.`,
    variant: "warning",
    actions: [{ label: "Understood", variant: "primary", actionKey: "close" }],
  },
  GEMINI_AUTH: {
    title: "Gemini API Key Required",
    message: "To begin using Squigit, please configure your Gemini API key.",
    variant: "info",
    actions: [
      { label: "Cancel", variant: "secondary", actionKey: "cancel" },
      { label: "Configure API Key", variant: "primary", actionKey: "confirm" },
    ],
  },
  IMGBB_AUTH: {
    title: "ImgBB API Key Required",
    message:
      "To use Google Lens features, please configure your ImgBB API key.",
    variant: "info",
    actions: [
      { label: "Cancel", variant: "secondary", actionKey: "cancel" },
      { label: "Configure API Key", variant: "primary", actionKey: "confirm" },
    ],
  },
  EXISTING_PROFILE: {
    title: "Account Already Signed In",
    message: "This account is already signed in.",
    variant: "info",
    actions: [{ label: "Close", variant: "primary", actionKey: "close" }],
  },
  REMOVE_ACCOUNT: {
    title: "Confirm Account Removal",
    message:
      "Are you sure you want to remove this account?\nThis action is irreversible.",
    variant: "warning",
    actions: [
      { label: "Cancel", variant: "secondary", actionKey: "cancel" },
      { label: "Remove Account", variant: "danger", actionKey: "confirm" },
    ],
  },
  DELETE_CHAT: {
    title: "Delete Conversation",
    message:
      "Are you sure you want to delete this conversation?\nThis action is irreversible.",
    variant: "error",
    actions: [
      { label: "Cancel", variant: "secondary", actionKey: "cancel" },
      { label: "Delete Conversation", variant: "danger", actionKey: "confirm" },
    ],
  },
  UNDO_MESSAGE: {
    title: "Undo to This Message",
    message:
      "This will remove this message and all following messages from the conversation.\nThe message text and attachments will be restored to the input editor.",
    variant: "warning",
    actions: [
      { label: "Cancel", variant: "secondary", actionKey: "cancel" },
      { label: "Undo and Edit", variant: "danger", actionKey: "confirm" },
    ],
  },
  LOGIN_REQUIRED: {
    title: "Login Required",
    message: `You need to sign in first to use ${appName}.`,
    variant: "info",
    actions: [
      { label: "Cancel", variant: "secondary", actionKey: "cancel" },
      { label: "Sign In", variant: "primary", actionKey: "confirm" },
    ],
  },
});

// Helper for dynamic messages
export const getDeleteMultipleChatsDialog = (count: number): DialogContent => ({
  title: "Delete Multiple Conversations",
  message: `Are you sure you want to delete ${count} conversations?\nThis action is irreversible.`,
  variant: "error",
  actions: [
    { label: "Cancel", variant: "secondary", actionKey: "cancel" },
    { label: "Delete All", variant: "danger", actionKey: "confirm" },
  ],
});

export const getAppBusyDialog = (reason: string): DialogContent => ({
  title: "Application Busy",
  message: `The application is currently ${reason}. Please wait until it finishes.`,
  variant: "info",
  actions: [
    { label: "Keep Working", variant: "secondary", actionKey: "cancel" },
    {
      label: "Continue",
      variant: "danger",
      actionKey: "confirm",
    },
  ],
});

import { parseAppError } from "./error-parser";

export const getErrorDialog = (error: any): DialogContent => {
  const parsed = parseAppError(error);

  return {
    title: parsed.title,
    message: parsed.message,
    variant: "error",
    actions: [{ label: "Close", variant: "primary", actionKey: "close" }],
  };
};

export const getMissingPackageDialog = (pkgName: string, installCmd: string): DialogContent => ({
  title: "Missing System Dependency",
  message: `A runtime dependency is missing for ${pkgName}. You can quickly install it using your terminal:\n\n${installCmd}`,
  variant: "warning",
  actions: [
    { label: "Dismiss", variant: "secondary", actionKey: "cancel" },
  ],
});

export const getOutdatedPackageDialog = (pkgName: string): DialogContent => ({
  title: "Version Mismatch",
  message: `Your installed version of ${pkgName} is incompatible with this application version. Please upgrade or downgrade it to match the required package lock specifications.`,
  variant: "warning",
  actions: [
    { label: "Close", variant: "primary", actionKey: "cancel" },
  ],
});

export const getUpdateAvailableDialog = (pkgName: string): DialogContent => ({
  title: "Update Available",
  message: `This package has new versions. Update your system: ${pkgName}`,
  variant: "info",
  actions: [
    { label: "Cancel", variant: "secondary", actionKey: "cancel" },
    { label: "Show Changelog", variant: "primary", actionKey: "show_changelog" },
  ],
});
