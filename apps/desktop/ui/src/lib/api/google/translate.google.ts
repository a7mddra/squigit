/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { google } from "@/lib";

/**
 * Generates a Google Translate URL for the given text.
 * @param text - The text to translate
 * @param targetLang - Target language code (default: "en")
 * @param sourceLang - Source language code (default: "auto")
 * @returns The full Google Translate URL
 */
export function generateTranslateUrl(
  text: string,
  targetLang: string = "en",
  sourceLang: string = "auto",
): string {
  const params = new URLSearchParams({
    text,
    sl: sourceLang,
    tl: targetLang,
    op: "translate",
  });

  return `${google.translate}/?${params.toString()}`;
}
