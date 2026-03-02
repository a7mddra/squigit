/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

export enum ModelType {
  GEMINI_2_5_FLASH = "gemini-2.5-flash",
  GEMINI_FLASH_LITE = "gemini-flash-lite-latest",
  GEMINI_2_5_PRO = "gemini-2.5-pro",
}

export const MODELS = [
  { id: ModelType.GEMINI_2_5_FLASH, name: "Gemini 2.5 Flash" },
  { id: ModelType.GEMINI_FLASH_LITE, name: "Gemini Flash Lite" },
  { id: ModelType.GEMINI_2_5_PRO, name: "Gemini 2.5 Pro" },
];

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
