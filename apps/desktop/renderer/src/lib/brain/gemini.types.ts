/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

export interface AppConfig {
  google_gemini: {
    api_key: string;
    api_endpoint: string;
  };
}

export interface GeminiPart {
  text?: string;
  inlineData?: {
    mimeType: string;
    data: string;
  };
}

export interface GeminiContent {
  role: string;
  parts: GeminiPart[];
}

export type Content = GeminiContent;

export interface GeminiEvent {
  type: "token";
  token: string;
}

export interface GeminiResetEvent {
  type: "reset";
}

export interface GeminiToolStatusEvent {
  type: "tool_status";
  message: string;
}

export interface GeminiToolStartEvent {
  type: "tool_start";
  id: string;
  name: string;
  args: Record<string, unknown>;
  message: string;
}

export interface GeminiToolEndEvent {
  type: "tool_end";
  id: string;
  name: string;
  status: "done" | "error" | string;
  result: Record<string, unknown>;
  message: string;
}

export type GeminiStreamEvent =
  | GeminiEvent
  | GeminiResetEvent
  | GeminiToolStatusEvent
  | GeminiToolStartEvent
  | GeminiToolEndEvent;
