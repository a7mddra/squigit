/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { uploadToImgBB, generateLensUrl } from "./lens.google";

export const useGoogleLens = (
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
  } | null,
  cachedUrl: string | null,
  setCachedUrl: (url: string) => void,
  activeProfileId: string | null,
) => {
  const [isLensLoading, setIsLensLoading] = useState(false);
  const [waitingForKey, setWaitingForKey] = useState(false);
  const [showAuthDialog, setShowAuthDialog] = useState(false);

  const imageRef = useRef(startupImage);
  useEffect(() => {
    imageRef.current = startupImage;
  }, [startupImage]);

  const appendQuery = (url: string, query?: string) => {
    if (!query || !query.trim()) return url;
    const encodedQuery = encodeURIComponent(query.trim());
    return `${url}&q=${encodedQuery}`;
  };

  const runLensSearch = async (
    imagePath: string,
    key: string,
    searchQuery?: string,
  ) => {
    try {
      setIsLensLoading(true);
      if (cachedUrl) {
        const finalUrl = appendQuery(cachedUrl, searchQuery);
        await invoke("open_external_url", { url: finalUrl });
        return;
      }

      const publicUrl = await uploadToImgBB(imagePath, key);
      const lensUrl = generateLensUrl(publicUrl);
      const finalUrl = appendQuery(lensUrl, searchQuery);

      setCachedUrl(lensUrl);
      await invoke("open_external_url", { url: finalUrl });
    } finally {
      setIsLensLoading(false);
    }
  };

  useEffect(() => {
    if (!waitingForKey) return;

    const unlistenKeyPromise = listen<{ provider: string; key: string }>(
      "clipboard-text",
      async (event) => {
        const { provider, key } = event.payload;
        if (provider === "imgbb") {
          setWaitingForKey(false);
          invoke("close_imgbb_window");
          if (imageRef.current) {
            await runLensSearch(imageRef.current.path, key);
          }
        }
      },
    );

    const unlistenClosePromise = listen<void>("imgbb-popup-closed", () => {
      console.log("Setup window closed without key");
      setWaitingForKey(false);
    });

    return () => {
      unlistenKeyPromise.then((f) => f());
      unlistenClosePromise.then((f) => f());
    };
  }, [waitingForKey]);

  const triggerLens = async (searchQuery?: string) => {
    if (!startupImage) return;
    if (isLensLoading || waitingForKey) return;

    if (cachedUrl) {
      const finalUrl = appendQuery(cachedUrl, searchQuery);
      await invoke("open_external_url", { url: finalUrl });
      return;
    }

    try {
      setIsLensLoading(true);
      console.log(
        "[useGoogleLens] triggerLens called with activeProfileId:",
        activeProfileId,
      );

      if (!activeProfileId) {
        console.error("[useGoogleLens] No active profile ID!");
        setShowAuthDialog(true);
        setIsLensLoading(false);
        return;
      }

      const apiKey = await invoke<string>("get_api_key", {
        provider: "imgbb",
        profileId: activeProfileId,
      });

      console.log(
        "[useGoogleLens] Retrieved API key for imgbb:",
        apiKey ? "FOUND" : "EMPTY",
      );

      if (apiKey) {
        await runLensSearch(startupImage.path, apiKey, searchQuery);
      } else {
        setShowAuthDialog(true);
        setIsLensLoading(false);
      }
    } catch (error) {
      console.error("Lens Trigger Error:", error);
      setShowAuthDialog(true);
      setIsLensLoading(false);
      setWaitingForKey(false);
    }
  };

  return {
    isLensLoading: isLensLoading || waitingForKey,
    triggerLens,
    showAuthDialog,
    setShowAuthDialog,
  };
};
