/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */
import { invoke } from "@tauri-apps/api/core";
import { OcrModel } from "../ocr-models.types";

/**
 * Triggers the backend to download and extract an OCR model.
 * @param model - The model metadata containing the download URL and ID.
 * @returns A promise that resolves to the path of the downloaded model.
 * @throws If the download or extraction fails.
 */
export const downloadModel = async (model: OcrModel): Promise<string> => {
  try {
    const downloadedPath = await invoke<string>("download_ocr_model", {
      url: model.downloadUrl,
      modelId: model.id,
    });
    return downloadedPath;
  } catch (error) {
    console.error(`Failed to download model ${model.id}:`, error);
    throw new Error(`Failed to download ${model.name}: ${error}`);
  }
};
