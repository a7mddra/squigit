/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export interface PreferencesPort {
  hasAgreedFlag(): Promise<boolean>;
  setAgreedFlag(): Promise<void>;
  hasPreferencesFile(fileName: string): Promise<boolean>;
  readPreferencesFile(fileName: string): Promise<string>;
  writePreferencesFile(fileName: string, content: string): Promise<void>;
}

let preferencesPort: PreferencesPort | null = null;

export function setPreferencesPort(port: PreferencesPort): void {
  preferencesPort = port;
}

export function getPreferencesPort(): PreferencesPort {
  if (!preferencesPort) {
    throw new Error("PreferencesPort is not initialized");
  }

  return preferencesPort;
}
