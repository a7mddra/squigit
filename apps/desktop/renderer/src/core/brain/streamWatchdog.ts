/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

const DEFAULT_STREAM_STALL_TIMEOUT_MS = 120_000;

interface StreamWatchdog {
  touch: () => void;
  stop: () => void;
  stallPromise: Promise<never>;
}

export const createStreamWatchdog = (
  onStall: () => void,
  timeoutMs = DEFAULT_STREAM_STALL_TIMEOUT_MS,
): StreamWatchdog => {
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
};

