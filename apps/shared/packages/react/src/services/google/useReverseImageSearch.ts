/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect, useRef } from "react";
import { google } from "@squigit/core/services/google";
import { getSystemPort } from "@squigit/core/ports";
import { openExternalUrl } from "@squigit/core/services/system";

/**
 * Uploads an image file path to ImgBB and returns the hosted URL.
 * @param imagePath - Absolute local image path
 * @param apiKey - The ImgBB API key
 * @returns The public URL of the uploaded image
 */
export async function uploadToImgBB(
  imagePath: string,
  apiKey: string,
): Promise<string> {
  return getSystemPort().uploadImageToImgBB(imagePath, apiKey);
}

/**
 * Generates a Google Lens search URL for the given image URL.
 * @param imageUrl - The public URL of the image
 * @returns The full Google Lens URL
 */
export function generateLensUrl(imageUrl: string): string {
  const params = new URLSearchParams();
  params.append("url", imageUrl);
  params.append("ep", "subb");
  params.append("re", "df");
  params.append("s", "4");
  params.append("hl", "en");
  params.append("gl", "US");

  return `${google.lens}/uploadbyurl?${params.toString()}`;
}

export const useReverseImageSearch = (
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
        await openExternalUrl(finalUrl);
        return;
      }

      const publicUrl = await uploadToImgBB(imagePath, key);
      const lensUrl = generateLensUrl(publicUrl);
      const finalUrl = appendQuery(lensUrl, searchQuery);

      setCachedUrl(lensUrl);
      await openExternalUrl(finalUrl);
    } finally {
      setIsLensLoading(false);
    }
  };

  useEffect(() => {
    if (!waitingForKey) return;

    const unlistenKeyPromise = getSystemPort().listenToSystemEvent<{
      provider: string;
      key: string;
    }>(
      "clipboard-text",
      async (payload) => {
        const { provider, key } = payload;
        if (provider === "imgbb") {
          setWaitingForKey(false);
          await getSystemPort().closeImgbbWindow();
          if (imageRef.current) {
            await runLensSearch(imageRef.current.path, key);
          }
        }
      },
    );

    const unlistenClosePromise = getSystemPort().listenToSystemEvent<void>(
      "imgbb-popup-closed",
      () => {
        console.log("Setup window closed without key");
        setWaitingForKey(false);
      },
    );

    return () => {
      void unlistenKeyPromise.then((f) => f());
      void unlistenClosePromise.then((f) => f());
    };
  }, [waitingForKey]);

  const triggerLens = async (searchQuery?: string) => {
    if (!startupImage) return;
    if (isLensLoading || waitingForKey) return;

    if (cachedUrl) {
      const finalUrl = appendQuery(cachedUrl, searchQuery);
      await openExternalUrl(finalUrl);
      return;
    }

    try {
      setIsLensLoading(true);
      console.log(
        "[ReverseImageSearch] triggerLens called with activeProfileId:",
        activeProfileId,
      );

      if (!activeProfileId) {
        console.error("[ReverseImageSearch] No active profile ID!");
        setShowAuthDialog(true);
        setIsLensLoading(false);
        return;
      }

      const apiKey = await getSystemPort().getApiKey("imgbb", activeProfileId);

      console.log(
        "[ReverseImageSearch] Retrieved API key for imgbb:",
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
