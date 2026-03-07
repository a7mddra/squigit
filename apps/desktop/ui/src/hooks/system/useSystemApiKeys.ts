/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from "react";
import { commands, initializeGemini } from "@/lib";

export const useSystemApiKeys = (activeProfileId?: string) => {
  const [apiKey, setApiKey] = useState<string>("");
  const [imgbbKey, setImgbbKey] = useState<string>("");

  const handleSetAPIKey = async (
    provider: "google ai studio" | "imgbb",
    key: string,
  ) => {
    if (!activeProfileId) {
      console.error(
        "[useSystemApiKeys] No active profile - cannot save API key. Profile state:",
        activeProfileId,
      );
      return false;
    }
    try {
      console.log(
        `[useSystemApiKeys] Saving ${provider} key for profile ${activeProfileId}`,
      );
      await commands.setApiKey(provider, key, activeProfileId);
      if (provider === "google ai studio") {
        setApiKey(key);
        initializeGemini(key);
      } else {
        setImgbbKey(key);
      }
      return true;
    } catch (e) {
      console.error(`[useSystemApiKeys] Failed to set ${provider} API key:`, e);
      return false;
    }
  };

  return {
    apiKey,
    setApiKey,
    imgbbKey,
    setImgbbKey,
    handleSetAPIKey,
  };
};
