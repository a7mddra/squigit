/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

export enum ModelType {
  GEMINI_3_1_FLASH = "gemini-3.1-flash-lite-preview",
  GEMINI_3_1_PRO = "gemini-3.1-pro",
}

export const MODELS = [
  { id: ModelType.GEMINI_3_1_FLASH, name: "Gemini 3.1 Flash" },
  { id: ModelType.GEMINI_3_1_PRO, name: "Gemini 3.1 Pro" },
] as const;

export const DEFAULT_MODEL_ID = MODELS[0].id;

const SUPPORTED_MODEL_ID_SET = new Set<string>(MODELS.map((model) => model.id));

export const isSupportedModelId = (modelId?: string | null): boolean => {
  if (!modelId) return false;
  return SUPPORTED_MODEL_ID_SET.has(modelId);
};

export const resolveModelId = (
  modelId?: string | null,
  fallback: string = DEFAULT_MODEL_ID,
): string => {
  if (modelId && isSupportedModelId(modelId)) {
    return modelId;
  }
  return isSupportedModelId(fallback) ? fallback : DEFAULT_MODEL_ID;
};

export const DEFAULT_OCR_MODEL_ID = "pp-ocr-v5-en";

export const SUPPORTED_OCR_MODEL_IDS = [
  "pp-ocr-v5-en",
  "pp-ocr-v5-latin",
  "pp-ocr-v5-cyrillic",
  "pp-ocr-v5-korean",
  "pp-ocr-v5-cjk",
  "pp-ocr-v5-devanagari",
] as const;

const SUPPORTED_OCR_MODEL_ID_SET = new Set<string>(SUPPORTED_OCR_MODEL_IDS);

export const isSupportedOcrModelId = (modelId?: string | null): boolean => {
  if (!modelId) return false;
  return SUPPORTED_OCR_MODEL_ID_SET.has(modelId);
};

export const resolveOcrModelId = (
  modelId?: string | null,
  fallback: string = DEFAULT_OCR_MODEL_ID,
): string => {
  if (modelId && isSupportedOcrModelId(modelId)) {
    return modelId;
  }
  if (fallback === "") {
    return "";
  }
  return isSupportedOcrModelId(fallback) ? fallback : DEFAULT_OCR_MODEL_ID;
};
