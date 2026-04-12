/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect, useCallback } from "react";
import {
  OcrModelStatus,
  AVAILABLE_MODELS,
  getInstalledModelIds,
  downloadModel,
} from "@/features/ocr";
import { DEFAULT_OCR_MODEL_ID, resolveOcrModelId } from "@/core";

export const useOCRModels = () => {
  const [ocrModels, setOCRModels] = useState<OcrModelStatus[]>(() =>
    AVAILABLE_MODELS.map((m) => ({ ...m, state: "idle" })),
  );
  const [isLoading, setIsLoading] = useState(true);

  const refreshOCRModels = useCallback(async () => {
    setIsLoading(true);
    try {
      const installedIds = (await getInstalledModelIds())
        .map((id) => resolveOcrModelId(id, ""))
        .filter((id) => id.length > 0);
      setOCRModels((prevModels) =>
        prevModels.map((model) => {
          const isInstalled = installedIds.includes(model.id);
          if (isInstalled) {
            return { ...model, state: "downloaded" };
          }
          return model.state === "downloading"
            ? model
            : { ...model, state: "idle" };
        }),
      );
    } catch (error) {
      console.error("Failed to refresh model list:", error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    setOCRModels((prev) =>
      prev.map((m) =>
        m.id === DEFAULT_OCR_MODEL_ID ? { ...m, state: "downloaded" } : m,
      ),
    );
    refreshOCRModels();
  }, [refreshOCRModels]);

  const startDownload = useCallback(
    async (modelId: string) => {
      const modelToDownload = ocrModels.find((m) => m.id === modelId);
      if (!modelToDownload || modelToDownload.state !== "idle") return;

      setOCRModels((prev) =>
        prev.map((m) =>
          m.id === modelId ? { ...m, state: "downloading" } : m,
        ),
      );

      try {
        await downloadModel(modelToDownload);
        setOCRModels((prev) =>
          prev.map((m) =>
            m.id === modelId ? { ...m, state: "downloaded" } : m,
          ),
        );
      } catch (error) {
        console.error(`Download failed for ${modelId}:`, error);
        setOCRModels((prev) =>
          prev.map((m) => (m.id === modelId ? { ...m, state: "idle" } : m)),
        );
        throw error;
      }
    },
    [ocrModels],
  );

  return { ocrModels, isLoading, refreshOCRModels, startDownload };
};
