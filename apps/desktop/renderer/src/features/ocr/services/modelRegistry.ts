/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";
import { OcrModel, AVAILABLE_MODELS } from "@/features";

/**
 * Fetches the list of all model IDs that are currently installed on disk.
 * @returns A promise that resolves to an array of installed model IDs.
 */
export const getInstalledModelIds = async (): Promise<string[]> => {
  try {
    const models = await invoke<string[]>("list_downloaded_models");
    return models;
  } catch (error) {
    console.error("Failed to list downloaded models:", error);
    return [];
  }
};

/**
 * Gets the local file system path for a given model ID.
 * @param modelId - The ID of the model (e.g., "pp-ocr-v5-en").
 * @returns A promise that resolves to the model's path, or null if not found.
 */
export const getModelPath = async (modelId: string): Promise<string | null> => {
  try {
    const path = await invoke<string>("get_model_path", { modelId });
    return path;
  } catch (error) {
    console.warn(`Could not get path for model ${modelId}:`, error);
    return null;
  }
};

/**
 * Gets the full OcrModel object for a given model ID.
 * @param modelId - The ID of the model.
 * @returns The OcrModel object, or undefined if not found.
 */
export const getModelById = (modelId: string): OcrModel | undefined => {
  return AVAILABLE_MODELS.find((m) => m.id === modelId);
};
