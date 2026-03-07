/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";
import { highlightCode } from "@/lib";

export const useCodeHighlighter = (
  code: string,
  language: string,
  enabled = true,
) => {
  const [highlightedHtml, setHighlightedHtml] = useState("");
  const [isLoading, setIsLoading] = useState(enabled && language !== "text");

  useEffect(() => {
    if (!enabled || language === "text") {
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    let isMounted = true;

    const doHighlight = async () => {
      try {
        const html = await highlightCode(code, language);

        if (isMounted) {
          setHighlightedHtml(html);
          setIsLoading(false);
        }
      } catch (error) {
        console.error("[syntax] Highlighting error:", error);
        if (isMounted) setIsLoading(false);
      }
    };

    doHighlight();

    return () => {
      isMounted = false;
    };
  }, [code, language, enabled]);

  return { highlightedHtml, isLoading };
};
