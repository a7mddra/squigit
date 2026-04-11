/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { OcrModelStatus, AVAILABLE_MODELS } from "./ocr-models.types";
import { getInstalledModelIds } from "./services/modelRegistry";
import { downloadModel } from "./services/modelDownloader";

import { commands, DEFAULT_OCR_MODEL_ID, resolveOcrModelId } from "@/core";

interface DownloadProgressPayload {
  id: string;
  progress: number;
  loaded: number;
  total: number;
  status: "checking" | "downloading" | "extracting" | "paused";
}

const clampProgress = (value: number): number =>
  Math.max(0, Math.min(100, Math.round(value)));

const resolveProgress = (
  currentProgress: number | undefined,
  payload: DownloadProgressPayload,
): number => {
  const current = currentProgress ?? 0;

  const ratioProgress =
    payload.total > 0 && payload.loaded > 0
      ? clampProgress((payload.loaded / payload.total) * 100)
      : 0;

  const reportedProgress = clampProgress(payload.progress || 0);
  const nextKnownProgress = Math.max(reportedProgress, ratioProgress);

  if (payload.status === "extracting") {
    return 100;
  }

  if (nextKnownProgress > 0) {
    return Math.max(current, nextKnownProgress);
  }

  if (
    (payload.status === "checking" ||
      payload.status === "paused" ||
      payload.status === "downloading") &&
    current > 0
  ) {
    return current;
  }

  return 0;
};

export interface ModelsState {
  models: OcrModelStatus[];
  isLoading: boolean;
  refresh: () => Promise<void>;
  startDownload: (modelId: string) => Promise<void>;
  cancelDownload: (modelId: string) => Promise<void>;
  installedModels: () => OcrModelStatus[];
}

export const useModelsStore = create<ModelsState>((set, get) => ({
  models: AVAILABLE_MODELS.map((m) => ({
    ...m,
    state: m.id === DEFAULT_OCR_MODEL_ID ? "downloaded" : "idle",
  })),
  isLoading: true,

  installedModels: () => get().models.filter((m) => m.state === "downloaded"),

  refresh: async () => {
    set({ isLoading: true });
    try {
      const installedIds = (await getInstalledModelIds())
        .map((id) => resolveOcrModelId(id, ""))
        .filter((id) => id.length > 0);

      if (!installedIds.includes(DEFAULT_OCR_MODEL_ID)) {
        installedIds.push(DEFAULT_OCR_MODEL_ID);
      }

      set((state) => ({
        models: state.models.map((model) => {
          const isInstalled = installedIds.includes(model.id);
          if (isInstalled) {
            return { ...model, state: "downloaded" };
          }

          const activeStates = [
            "checking",
            "downloading",
            "extracting",
            "paused",
          ];
          if (activeStates.includes(model.state)) {
            return model;
          }

          return { ...model, state: "idle" };
        }),
        isLoading: false,
      }));
    } catch (error) {
      console.error("Failed to refresh model list:", error);
      set({ isLoading: false });
    }
  },

  startDownload: async (modelId: string) => {
    const modelToDownload = get().models.find((m) => m.id === modelId);
    if (
      !modelToDownload ||
      (modelToDownload.state !== "idle" && modelToDownload.state !== "paused")
    )
      return;

    set((state) => ({
      models: state.models.map((m) =>
        m.id === modelId
          ? { ...m, state: "checking", progress: m.progress ?? 0 }
          : m,
      ),
    }));

    try {
      await downloadModel(modelToDownload);
      await get().refresh();
    } catch (error) {
      console.error(`Download failed for ${modelId}:`, error);
      set((state) => ({
        models: state.models.map((m) =>
          m.id === modelId ? { ...m, state: "idle" } : m,
        ),
      }));
      throw error;
    }
  },

  cancelDownload: async (modelId: string) => {
    try {
      await commands.cancelDownloadOcrModel(modelId);
      set((state) => ({
        models: state.models.map((m) =>
          m.id === modelId ? { ...m, state: "idle", progress: 0 } : m,
        ),
      }));
    } catch (error) {
      console.error(`Failed to cancel download for ${modelId}:`, error);
    }
  },
}));

useModelsStore.getState().refresh();

listen<DownloadProgressPayload>("download-progress", (event) => {
  const { id, status } = event.payload;
  useModelsStore.setState((state) => ({
    models: state.models.map((m) => {
      if (m.id !== id) return m;

      let newState = m.state;

      if (status === "checking") newState = "checking";
      else if (status === "downloading") newState = "downloading";
      else if (status === "extracting") newState = "extracting";
      else if (status === "paused") newState = "paused";

      return {
        ...m,
        progress: resolveProgress(m.progress, event.payload),
        state: newState,
      };
    }),
  }));
});
