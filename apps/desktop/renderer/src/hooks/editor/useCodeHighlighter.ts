/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";
import { highlightCode } from "@/core";

const CACHE_MAX = 64;
const highlightCache = new Map<string, string>();
const highlightPromiseCache = new Map<string, Promise<string>>();

function cacheKey(code: string, language: string): string {
  return `${language}\0${code}`;
}

function cacheSet(key: string, html: string): void {
  if (highlightCache.size >= CACHE_MAX) {
    const firstKey = highlightCache.keys().next().value;
    if (firstKey !== undefined) {
      highlightCache.delete(firstKey);
    }
  }
  highlightCache.set(key, html);
}

function getHighlightPromise(
  key: string,
  code: string,
  language: string,
): Promise<string> {
  const existingPromise = highlightPromiseCache.get(key);
  if (existingPromise) {
    return existingPromise;
  }

  const nextPromise = highlightCode(code, language)
    .then((html) => {
      cacheSet(key, html);
      highlightPromiseCache.delete(key);
      return html;
    })
    .catch((error) => {
      highlightPromiseCache.delete(key);
      throw error;
    });

  highlightPromiseCache.set(key, nextPromise);
  return nextPromise;
}

export const useCodeHighlighter = (
  code: string,
  language: string,
  enabled = true,
) => {
  const key = enabled && language !== "text" ? cacheKey(code, language) : "";
  const cached = key ? highlightCache.get(key) : undefined;

  const [highlightedHtml, setHighlightedHtml] = useState(cached ?? "");
  const [isLoading, setIsLoading] = useState(
    enabled && language !== "text" && !cached,
  );

  useEffect(() => {
    if (!enabled || language === "text") {
      setIsLoading(false);
      return;
    }

    const k = cacheKey(code, language);
    const hit = highlightCache.get(k);
    if (hit) {
      setHighlightedHtml(hit);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    let isMounted = true;

    const doHighlight = async () => {
      try {
        const html = await getHighlightPromise(k, code, language);

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
