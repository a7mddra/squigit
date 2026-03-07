/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export interface OcrModel {
  id: string;
  name: string;
  lang: string;
  size: string;
  downloadUrl: string;
}

export interface OcrModelStatus extends OcrModel {
  state:
    | "idle"
    | "checking"
    | "downloading"
    | "paused"
    | "downloaded"
    | "extracting";
  progress?: number;
}

export const AVAILABLE_MODELS: OcrModel[] = [
  {
    id: "pp-ocr-v5-en",
    name: "PP-OCR-V5 English",
    lang: "en",
    size: "~16 MB",
    downloadUrl:
      "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/en_PP-OCRv5_mobile_rec_infer.tar",
  },
  {
    id: "pp-ocr-v5-latin",
    name: "PP-OCR-V5 Latin",
    lang: "la",
    size: "~17 MB",
    downloadUrl:
      "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/latin_PP-OCRv5_mobile_rec_infer.tar",
  },
  {
    id: "pp-ocr-v5-cyrillic",
    name: "PP-OCR-V5 Cyrillic",
    lang: "ru",
    size: "~17 MB",
    downloadUrl:
      "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/cyrillic_PP-OCRv5_mobile_rec_infer.tar",
  },
  {
    id: "pp-ocr-v5-korean",
    name: "PP-OCR-V5 Korean",
    lang: "ko",
    size: "~18 MB",
    downloadUrl:
      "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/korean_PP-OCRv5_mobile_rec_infer.tar",
  },
  {
    id: "pp-ocr-v5-cjk",
    name: "PP-OCR-V5 CJK",
    lang: "ch",
    size: "~83 MB",
    downloadUrl:
      "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/PP-OCRv5_server_rec_infer.tar",
  },
  {
    id: "pp-ocr-v5-devanagari",
    name: "PP-OCR-V5 Devanagari",
    lang: "hi",
    size: "~17 MB",
    downloadUrl:
      "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/devanagari_PP-OCRv5_mobile_rec_infer.tar",
  },
];

export const getLanguageCode = (modelId: string) => {
  if (!modelId) return "EN";
  const model = AVAILABLE_MODELS.find((m) => m.id === modelId);
  return model ? model.lang.toUpperCase() : "EN";
};
