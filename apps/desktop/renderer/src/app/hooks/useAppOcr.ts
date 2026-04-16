/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { type OcrFrame, saveImgbbUrl, saveOcrData } from "@squigit/core/config";

export const useAppOcr = (activeSessionId: string | null) => {
  const [sessionLensUrl, setSessionLensUrl] = useState<string | null>(null);
  const [ocrData, setOcrData] = useState<OcrFrame>({});
  const [isOcrScanning, setIsOcrScanning] = useState(false);
  const activeSessionIdRef = useRef(activeSessionId);

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  const handleUpdateLensUrl = useCallback(
    (url: string | null) => {
      setSessionLensUrl(url);
      if (activeSessionId && url) {
        saveImgbbUrl(activeSessionId, url).catch((e) =>
          console.error("Failed to save ImgBB URL", e),
        );
      }
    },
    [activeSessionId],
  );

  const handleUpdateOCRData = useCallback(
    (
      targetChatId: string | null,
      modelId: string,
      data: { text: string; box: number[][] }[],
    ) => {
      const regions = data.map((d) => ({
        text: d.text,
        bbox: d.box,
      }));
      const chatIdForWrite = targetChatId || activeSessionIdRef.current;

      if (chatIdForWrite && modelId) {
        saveOcrData(chatIdForWrite, modelId, regions).catch((e) =>
          console.error("Failed to save OCR", e),
        );
      }

      if (
        !chatIdForWrite ||
        chatIdForWrite !== activeSessionIdRef.current ||
        !modelId
      ) {
        return;
      }

      console.log(`[useApp] Updating OCR data for model: ${modelId}`);
      setOcrData((prev) => {
        const newState = {
          ...prev,
          [modelId]: regions,
        };
        console.log(
          `[useApp] New OCR Data keys: ${Object.keys(newState).join(", ")}`,
        );
        return newState;
      });
    },
    [],
  );

  return {
    sessionLensUrl,
    setSessionLensUrl,
    handleUpdateLensUrl,
    ocrData,
    setOcrData,
    handleUpdateOCRData,
    isOcrScanning,
    setIsOcrScanning,
  };
};
