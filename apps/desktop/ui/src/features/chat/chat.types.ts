/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export interface Message {
  id: string;
  role: "user" | "model" | "system";
  text: string;
  image?: string;
  timestamp: number;
  stopped?: boolean;
  alreadyStreamed?: boolean;
}

export interface AppConfig {
  google_gemini: {
    api_key: string;
    api_endpoint: string;
  };
}

export interface ChatSession {
  id: string;
  title: string;
  messages: Message[];
  streamingText: string;
  firstResponseId: string | null;
  createdAt: number;
  type: "default" | "edit";
}
