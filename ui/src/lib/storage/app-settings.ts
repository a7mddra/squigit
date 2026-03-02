/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

import {
  BaseDirectory,
  exists,
  readTextFile,
  writeTextFile,
  mkdir,
} from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core";
import { commands } from "@/lib";
import { resolveOcrModelId } from "@/lib";

export interface UserPreferences {
  model: string;
  theme: "dark" | "light" | "system";
  prompt: string;
  ocrEnabled: boolean;
  autoExpandOCR: boolean;
  captureType: "rectangular" | "squiggle";
  ocrLanguage: string;
  activeAccount: string;
}

export async function getDefaultPreferences(): Promise<UserPreferences> {
  const constants = await commands.getAppConstants();
  return {
    model: constants.defaultModel,
    theme: constants.defaultTheme as "dark" | "light" | "system",
    prompt: constants.defaultPrompt,
    ocrEnabled: true,
    autoExpandOCR: true,
    captureType: constants.defaultCaptureType as "rectangular" | "squiggle",
    ocrLanguage: constants.defaultOcrLanguage,
    activeAccount: constants.defaultActiveAccount,
  };
}

export async function hasAgreedFlag(): Promise<boolean> {
  try {
    return await invoke<boolean>("has_agreed_flag");
  } catch (error) {
    console.warn("Agreed flag check failed:", error);
    return false;
  }
}

export async function setAgreedFlag(): Promise<void> {
  try {
    await invoke("set_agreed_flag");
  } catch (error) {
    console.error("Failed to write agreed flag via native command:", error);
  }
}

export async function hasPreferencesFile(): Promise<boolean> {
  try {
    const constants = await commands.getAppConstants();
    return await exists(constants.preferencesFileName, {
      baseDir: BaseDirectory.AppConfig,
    });
  } catch (error) {
    console.warn("Preferences check failed:", error);
    return false;
  }
}

export async function loadPreferences(): Promise<UserPreferences> {
  try {
    const fileExists = await hasPreferencesFile();
    const defaultPrefs = await getDefaultPreferences();
    if (!fileExists) {
      return defaultPrefs;
    }

    const constants = await commands.getAppConstants();
    const content = await readTextFile(constants.preferencesFileName, {
      baseDir: BaseDirectory.AppConfig,
    });
    const parsed = JSON.parse(content) as Partial<UserPreferences>;

    const normalizedOcrLanguage = resolveOcrModelId(
      parsed.ocrLanguage,
      defaultPrefs.ocrLanguage,
    );
    const merged = {
      ...defaultPrefs,
      ...parsed,
      ocrLanguage: normalizedOcrLanguage,
    };

    // One-time on-load rewrite for unsupported OCR model IDs.
    if (parsed.ocrLanguage !== normalizedOcrLanguage) {
      await savePreferences(merged);
    }

    return merged;
  } catch (error) {
    console.error("Failed to load preferences:", error);
    return await getDefaultPreferences();
  }
}

export async function savePreferences(prefs: UserPreferences): Promise<void> {
  try {
    const constants = await commands.getAppConstants();
    await mkdir("", { baseDir: BaseDirectory.AppConfig, recursive: true });
    await writeTextFile(
      constants.preferencesFileName,
      JSON.stringify(prefs, null, 2),
      {
        baseDir: BaseDirectory.AppConfig,
      },
    );
  } catch (error) {
    console.error("Failed to save preferences:", error);
    if (typeof error === "object" && error !== null) {
      console.error("Error details:", JSON.stringify(error));
    }
    throw error;
  }
}
