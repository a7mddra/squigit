/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

export interface ImageResponse {
  base64: string;
  mimeType: string;
}

export interface UserData {
  name: string;
  email: string;
  avatar: string;
}

export interface Profile {
  id: string;
  name: string;
  email: string;
  avatar: string | null;
  original_avatar?: string | null;
}

export interface AppConstants {
  appName: string;
  defaultModel: string;
  defaultTheme: string;
  defaultPrompt: string;
  preferencesFileName: string;
  defaultCaptureType: string;
  defaultOcrLanguage: string;
  defaultActiveAccount: string;
}
