/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";
import {
  UserPreferences,
  loadPreferences,
  savePreferences,
  commands,
  DEFAULT_OCR_MODEL_ID,
  resolveOcrModelId,
} from "@/lib";
import { useTheme } from "@/hooks";

export const useSystemPreferences = () => {
  const { theme, resolvedTheme, setTheme } = useTheme();

  useEffect(() => {
    const updateNativeBg = async () => {
      const color = resolvedTheme === "dark" ? "#0a0a0a" : "#ffffff";
      try {
        await commands.setBackgroundColor(color);
      } catch (e) {
        console.error("Failed to set native background color", e);
      }
    };
    updateNativeBg();
  }, [resolvedTheme]);

  const [activePrompt, setActivePrompt] = useState<string>("");
  const [editingPrompt, setEditingPrompt] = useState<string>("");
  const [startupModel, setStartupModel] = useState<string>("");
  const [editingModel, setEditingModel] = useState<string>("");
  const [sessionModel, setSessionModel] = useState<string>("");
  const [autoExpandOCR, setAutoExpandOCR] = useState<boolean>(true);
  const [ocrEnabled, setOcrEnabled] = useState<boolean>(true);
  const [captureType, setCaptureType] = useState<"rectangular" | "squiggle">(
    "rectangular",
  );
  const [startupOcrLanguage, setStartupOcrLanguage] =
    useState<string>(DEFAULT_OCR_MODEL_ID);
  const [sessionOcrLanguage, setSessionOcrLanguage] =
    useState<string>(DEFAULT_OCR_MODEL_ID);

  const updatePreferences = async (updates: Partial<UserPreferences>) => {
    const normalizedUpdatedOcrLanguage =
      updates.ocrLanguage !== undefined
        ? resolveOcrModelId(updates.ocrLanguage)
        : undefined;

    if (updates.model !== undefined) {
      setStartupModel(updates.model);
      setEditingModel(updates.model);
      setSessionModel(updates.model);
    }
    if (updates.prompt !== undefined) {
      setActivePrompt(updates.prompt);
      setEditingPrompt(updates.prompt);
    }
    if (updates.autoExpandOCR !== undefined) {
      setAutoExpandOCR(updates.autoExpandOCR);
    }
    if (updates.ocrEnabled !== undefined) {
      setOcrEnabled(updates.ocrEnabled);
      if (!updates.ocrEnabled) {
        setSessionOcrLanguage("");
      } else {
        setSessionOcrLanguage(
          normalizedUpdatedOcrLanguage ?? resolveOcrModelId(startupOcrLanguage),
        );
      }
    }
    if (updates.captureType !== undefined) {
      setCaptureType(updates.captureType);
    }
    if (normalizedUpdatedOcrLanguage !== undefined) {
      setStartupOcrLanguage(normalizedUpdatedOcrLanguage);
      if (ocrEnabled && updates.ocrEnabled === undefined) {
        setSessionOcrLanguage(normalizedUpdatedOcrLanguage);
      }
    }
    if (updates.theme !== undefined) {
      setTheme(updates.theme);
    }

    try {
      const currentPrefs = await loadPreferences();
      await savePreferences({ ...currentPrefs, ...updates });
    } catch (e) {
      console.error("Failed to save preferences:", e);
    }
  };

  return {
    theme,
    resolvedTheme,
    setTheme,
    isDarkMode: resolvedTheme === "dark",

    prompt: activePrompt,
    setActivePrompt,
    editingPrompt,
    setEditingPrompt,

    startupModel,
    setStartupModel,
    editingModel,
    setEditingModel,
    sessionModel,
    setSessionModel,

    autoExpandOCR,
    setAutoExpandOCR,
    ocrEnabled,
    setOcrEnabled,
    captureType,
    setCaptureType,
    startupOcrLanguage,
    setStartupOcrLanguage,
    sessionOcrLanguage,
    setSessionOcrLanguage,

    updatePreferences,
  };
};
