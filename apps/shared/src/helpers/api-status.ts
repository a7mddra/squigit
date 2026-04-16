/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export interface AttachmentAnalysisCounts {
  imageCount: number;
  fileCount: number;
}

export const ATTACHMENT_ANALYSIS_STATUS_DELAY_MS = 1800;
export const HIGH_DEMAND_RETRY_DELAYS_MS = [1000, 2000] as const;
export const HIGH_DEMAND_RETRY_ATTEMPTS = HIGH_DEMAND_RETRY_DELAYS_MS.length;
const ANALYZED_STATUS_PREFIX = "Analyzed ";

export const API_STATUS_TEXT = {
  ANALYZING_IMAGE: "Analyzing your image",
  WRAPPING_UP: "Wrapping up with what I have so far",
  SEARCHING_RELEVANT_SOURCES: "Searching for relevant sources",
  SEARCHED_WEB: "Searched web",
  TRYING_ANOTHER_RELIABLE_SOURCE: "Trying another reliable source",
  TRYING_ANOTHER_SOURCE: "Trying another source",
  SEARCH_UNAVAILABLE_CONTEXT:
    "Search is unavailable right now, continuing with available context",
  SEARCH_UNAVAILABLE_NOW: "Search is unavailable right now.",
  FETCHING_SOURCE_FAILED: "Couldn't open one source, trying another",
  MODEL_BUSY_RETRYING:
    "Things are a bit slow right now. Reconnecting",
  QUICK_ANSWER_BUTTON: "Quick Answer",
  WRITING_CODE: "Writing code",
} as const;

export const getAttachmentAnalysisStatusText = (
  counts: AttachmentAnalysisCounts | null | undefined,
): string => {
  if (!counts) {
    return "";
  }

  const parts: string[] = [];

  if (counts.imageCount > 0) {
    parts.push(
      `${counts.imageCount} image${counts.imageCount === 1 ? "" : "s"}`,
    );
  }

  if (counts.fileCount > 0) {
    parts.push(`${counts.fileCount} file${counts.fileCount === 1 ? "" : "s"}`);
  }

  return parts.length > 0
    ? `${ANALYZED_STATUS_PREFIX}${parts.join(", ")}`
    : "";
};

export const getHighDemandRetryStatusText = (
  attempt: number,
  total = HIGH_DEMAND_RETRY_ATTEMPTS,
): string => {
  return `${API_STATUS_TEXT.MODEL_BUSY_RETRYING} ${attempt}/${total}`;
};

export const isQuickAnswerSuppressedProgressText = (
  text: string | null | undefined,
): boolean => {
  const trimmed = text?.trim();
  if (!trimmed) {
    return false;
  }

  return (
    trimmed.startsWith(ANALYZED_STATUS_PREFIX) ||
    trimmed.startsWith(API_STATUS_TEXT.MODEL_BUSY_RETRYING)
  );
};

type ToolStatusMapResult =
  | { type: "set"; text: string }
  | { type: "clear" }
  | { type: "skip" };

const SKIP_TOOL_STATUS_PATTERNS: RegExp[] = [
  /^Calling web search tool/i,
  /^Unsupported tool requested by model/i,
  /: attempt \d+\/\d+/i,
  /: retrying after \d+ms/i,
  /: failed \[[^\]]+\]/i,
  /^Fetched page successfully\.?$/i,
  /^Page fetch failed \[[^\]]+\], returning snippet fallback\.?$/i,
];

export const mapToolStatusText = (
  message: string | null | undefined,
): ToolStatusMapResult => {
  if (message == null) {
    return { type: "clear" };
  }

  const trimmed = message.trim();
  if (!trimmed) {
    return { type: "clear" };
  }

  if (SKIP_TOOL_STATUS_PATTERNS.some((pattern) => pattern.test(trimmed))) {
    return { type: "skip" };
  }

  if (
    /^Quick Answer requested/i.test(trimmed) ||
    /^Tool call limit reached/i.test(trimmed)
  ) {
    return { type: "set", text: API_STATUS_TEXT.WRAPPING_UP };
  }

  if (
    /^Searching the web for /i.test(trimmed) ||
    /^Searching (DuckDuckGo|Mojeek) for /i.test(trimmed) ||
    /^Searching for relevant sources\.?$/i.test(trimmed)
  ) {
    return { type: "set", text: API_STATUS_TEXT.SEARCHING_RELEVANT_SOURCES };
  }

  const searchedMatch = trimmed.match(
    /^[^:]+:\s*found\s+(\d+)\s+(?:results?|sites?|sources?)\.?$/i,
  );
  if (searchedMatch) {
    const count = Number.parseInt(searchedMatch[1], 10);
    if (Number.isFinite(count) && count > 0) {
      return {
        type: "set",
        text: `${API_STATUS_TEXT.SEARCHED_WEB} - found ${count} sources`,
      };
    }
    return { type: "set", text: API_STATUS_TEXT.SEARCHED_WEB };
  }

  if (
    /^Primary search blocked/i.test(trimmed) ||
    /^Trying trusted source:/i.test(trimmed) ||
    /^Trusted local fallback failed/i.test(trimmed) ||
    /^Trying model-assisted trusted source:/i.test(trimmed) ||
    /^Trying another reliable source\.?$/i.test(trimmed)
  ) {
    return { type: "set", text: API_STATUS_TEXT.TRYING_ANOTHER_RELIABLE_SOURCE };
  }

  if (/^Web search temporarily unavailable/i.test(trimmed)) {
    return { type: "set", text: API_STATUS_TEXT.SEARCH_UNAVAILABLE_CONTEXT };
  }

  if (
    /^Search unavailable across all engines\.?$/i.test(trimmed) ||
    /^Search is unavailable right now\.?$/i.test(trimmed)
  ) {
    return { type: "set", text: API_STATUS_TEXT.SEARCH_UNAVAILABLE_NOW };
  }

  if (/^Trying another source\.?$/i.test(trimmed)) {
    return { type: "set", text: API_STATUS_TEXT.TRYING_ANOTHER_SOURCE };
  }

  if (
    /^Couldn't open one source, trying another\.?$/i.test(trimmed) ||
    /^Couldn’t open one source, trying another\.?$/i.test(trimmed)
  ) {
    return { type: "set", text: API_STATUS_TEXT.FETCHING_SOURCE_FAILED };
  }

  // Catch CAS hash filenames leaking through in tool status (e.g.
  // "Reading local context from 00f8e1ec68a9...rs"). If the name after
  // "from" is a long hex hash, show extension-aware fallback instead.
  const readingMatch = trimmed.match(
    /^Reading local context from ([a-f0-9]{32,})(\.\w+)?$/i,
  );
  if (readingMatch) {
    const ext = readingMatch[2] || "";
    return {
      type: "set",
      text: ext ? `Reading ${ext} file` : "Reading attached file",
    };
  }

  return { type: "set", text: trimmed };
};

export const getProgressStatusText = (params: {
  toolStatus?: string | null;
  isAnalyzing?: boolean;
  isRetrying?: boolean;
}) => {
  if (params.toolStatus) {
    return params.toolStatus;
  }
  if (params.isAnalyzing) {
    return API_STATUS_TEXT.ANALYZING_IMAGE;
  }
  return "";
};
