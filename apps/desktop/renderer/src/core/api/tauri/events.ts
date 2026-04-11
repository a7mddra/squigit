/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { listen } from "@tauri-apps/api/event";

export interface TauriEventMap {
  "gemini-stream-token":
    | { type: "token"; token: string }
    | { type: "reset" }
    | { type: "tool_status"; message: string }
    | { type: "tool_start"; id: string; name: string; args: Record<string, unknown>; message: string }
    | { type: "tool_end"; id: string; name: string; status: string; result: Record<string, unknown>; message: string };
}

export function listenTo<K extends keyof TauriEventMap>(
  event: K,
  handler: (payload: TauriEventMap[K]) => void,
) {
  return listen<TauriEventMap[K]>(event, (e) => handler(e.payload));
}
