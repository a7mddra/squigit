/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type SystemApiKeyProvider = "google ai studio" | "imgbb";
export type SystemEventUnlisten = () => void;

export interface SystemPort {
  openExternalUrl(url: string): Promise<void>;
  deleteTempFile(path: string): Promise<void>;
  getApiKey(provider: SystemApiKeyProvider, profileId: string): Promise<string>;
  uploadImageToImgBB(imagePath: string, apiKey: string): Promise<string>;
  closeImgbbWindow(): Promise<void>;
  listenToSystemEvent<TPayload>(
    eventName: string,
    onEvent: (payload: TPayload) => void,
  ): Promise<SystemEventUnlisten>;
}

let systemPort: SystemPort | null = null;

export function setSystemPort(port: SystemPort): void {
  systemPort = port;
}

export function getSystemPort(): SystemPort {
  if (!systemPort) {
    throw new Error("SystemPort is not initialized");
  }

  return systemPort;
}
