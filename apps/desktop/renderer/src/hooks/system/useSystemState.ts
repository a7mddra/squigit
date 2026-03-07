/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from "react";
import { SettingsSection } from "@/features";

export const useSystemState = () => {
  const [appName, setAppName] = useState<string>("SnapLLM");
  const [startupImage, setStartupImage] = useState<{
    path: string;
    mimeType: string;
    imageId: string;
    fromHistory?: boolean;
  } | null>(null);

  const [sessionChatTitle, setSessionChatTitle] = useState<string | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] =
    useState<SettingsSection>("general");

  const [systemError, setSystemError] = useState<string | null>(null);
  const clearSystemError = () => setSystemError(null);

  const [prefsLoaded, setPrefsLoaded] = useState(false);
  const [hasAgreed, setHasAgreed] = useState<boolean | null>(null);

  const openSettings = (section: SettingsSection) => {
    setSettingsSection(section);
    setIsSettingsOpen(true);
  };

  return {
    appName,
    setAppName,
    startupImage,
    setStartupImage,
    sessionChatTitle,
    setSessionChatTitle,
    isSettingsOpen,
    setIsSettingsOpen,
    settingsSection,
    setSettingsSection,
    systemError,
    setSystemError,
    clearSystemError,
    prefsLoaded,
    setPrefsLoaded,
    hasAgreed,
    setHasAgreed,
    openSettings,
  };
};
