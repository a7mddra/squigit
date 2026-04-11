/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export async function waitForRetryDelay(
  delayMs: number,
  isRequestAborted: (signal?: AbortSignal) => boolean,
  signal?: AbortSignal,
): Promise<void> {
  const startedAtMs = Date.now();

  await new Promise<void>((resolve, reject) => {
    let timerId: number | null = null;

    const tick = () => {
      if (isRequestAborted(signal)) {
        if (timerId !== null) {
          window.clearTimeout(timerId);
        }
        reject(new Error("CANCELLED"));
        return;
      }

      if (Date.now() - startedAtMs >= delayMs) {
        resolve();
        return;
      }

      timerId = window.setTimeout(tick, 150);
    };

    tick();
  });
}

export async function runWithRetries<T>(
  run: () => Promise<T>,
  options: {
    signal?: AbortSignal;
    maxRetries: number;
    retryDelaysMs: number[];
    isRequestAborted: (signal?: AbortSignal) => boolean;
    shouldRetry: (error: unknown, signal?: AbortSignal) => boolean;
    onRetry: (retryCount: number) => void;
    onRetryExhausted: (maxRetries: number) => Error;
  },
): Promise<T> {
  let retriesUsed = 0;

  while (true) {
    try {
      return await run();
    } catch (error: any) {
      if (
        error?.message === "CANCELLED" ||
        options.isRequestAborted(options.signal)
      ) {
        throw new Error("CANCELLED");
      }

      if (!options.shouldRetry(error, options.signal)) {
        throw error;
      }

      if (retriesUsed >= options.maxRetries) {
        throw options.onRetryExhausted(options.maxRetries);
      }

      retriesUsed += 1;
      options.onRetry(retriesUsed);

      await waitForRetryDelay(
        options.retryDelaysMs[retriesUsed - 1],
        options.isRequestAborted,
        options.signal,
      );
    }
  }
}
