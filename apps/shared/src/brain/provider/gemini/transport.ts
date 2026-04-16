/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { cancelGeminiRequest, requestGeminiQuickAnswer } from "./commands";
import { brainSessionStore } from "../../session/store";

const DEFAULT_STREAM_STALL_TIMEOUT_MS = 120_000;

interface StreamWatchdog {
  touch: () => void;
  stop: () => void;
  stallPromise: Promise<never>;
}

export function createStreamWatchdog(
  onStall: () => void,
  timeoutMs = DEFAULT_STREAM_STALL_TIMEOUT_MS,
): StreamWatchdog {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  let settled = false;
  let rejectStall: ((reason?: unknown) => void) | null = null;

  const stallPromise = new Promise<never>((_, reject) => {
    rejectStall = reject;
  });

  const stop = () => {
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
    settled = true;
  };

  const touch = () => {
    if (settled) return;
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
    }

    timeoutId = setTimeout(() => {
      if (settled) return;
      settled = true;
      try {
        onStall();
      } finally {
        rejectStall?.(
          new Error(
            `Streaming stalled with no events for ${Math.round(timeoutMs / 1000)}s.`,
          ),
        );
      }
    }, timeoutMs);
  };

  return { touch, stop, stallPromise };
}

export function createProviderChannelId(): string {
  return `gemini-stream-${Date.now()}`;
}

export function clearActiveProviderTransport(
  channelId: string,
  unlisten: (() => void) | null,
) {
  unlisten?.();
  if (brainSessionStore.currentUnlisten === unlisten) {
    brainSessionStore.currentUnlisten = null;
  }
  if (brainSessionStore.currentChannelId === channelId) {
    brainSessionStore.currentChannelId = null;
  }
}

export function cancelCurrentRequest() {
  brainSessionStore.generationId++;

  if (brainSessionStore.currentAbortController) {
    brainSessionStore.currentAbortController.abort();
    brainSessionStore.currentAbortController = null;
  }

  if (brainSessionStore.currentUnlisten) {
    brainSessionStore.currentUnlisten();
    brainSessionStore.currentUnlisten = null;
  }

  const activeChannelId = brainSessionStore.currentChannelId;
  brainSessionStore.currentChannelId = null;

  cancelGeminiRequest(activeChannelId).catch(console.error);
}

export async function quickAnswerCurrentRequest() {
  const channelId = brainSessionStore.currentChannelId;
  if (!channelId) return;

  try {
    await requestGeminiQuickAnswer(channelId);
  } catch (error) {
    console.error("Failed to request answer-now:", error);
  }
}
