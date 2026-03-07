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
  token: string;
}
