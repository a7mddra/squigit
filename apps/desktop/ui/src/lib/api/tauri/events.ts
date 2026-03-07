/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { listen } from "@tauri-apps/api/event";

export interface TauriEventMap {
  "gemini-stream-token": { token: string };
}

export function listenTo<K extends keyof TauriEventMap>(
  event: K,
  handler: (payload: TauriEventMap[K]) => void,
) {
  return listen<TauriEventMap[K]>(event, (e) => handler(e.payload));
}
