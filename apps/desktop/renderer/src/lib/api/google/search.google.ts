/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { google } from "@/lib";

/**
 * Generates a Google Search URL for the given query.
 * @param query - The search query text
 * @returns The full Google Search URL
 */
export function generateSearchUrl(query: string): string {
  const params = new URLSearchParams({ q: query });
  return `${google.search}/search?${params.toString()}`;
}
