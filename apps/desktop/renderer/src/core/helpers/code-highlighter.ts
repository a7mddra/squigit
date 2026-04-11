/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { createHighlighter, type Highlighter } from "shiki";

let highlighterPromise: Promise<Highlighter> | null = null;

export const SYNTAX_THEMES = {
  dark: "vesper",
  light: "github-light",
} as const;

export const DEFAULT_LANGUAGE = "bash";

export const getHighlighter = async (): Promise<Highlighter> => {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: [SYNTAX_THEMES.dark, SYNTAX_THEMES.light],
      langs: [],
    });
  }
  return highlighterPromise;
};

export const highlightCode = async (
  code: string,
  language: string,
): Promise<string> => {
  const highlighter = await getHighlighter();

  if (
    language &&
    language !== "text" &&
    !highlighter.getLoadedLanguages().includes(language)
  ) {
    try {
      await highlighter.loadLanguage(language as any);
    } catch (e) {
      console.warn(`[syntax] Failed to load language: ${language}`);
    }
  }

  const lang = highlighter.getLoadedLanguages().includes(language)
    ? language
    : DEFAULT_LANGUAGE;

  return highlighter.codeToHtml(code, {
    lang,
    themes: {
      light: SYNTAX_THEMES.light,
      dark: SYNTAX_THEMES.dark,
    },
    defaultColor: false,
  });
};
