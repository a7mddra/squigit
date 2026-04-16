const INCOMPLETE_LINK_PATTERNS: RegExp[] = [
  /!?\[[^\]]*$/u,
  /!?\[[^\]]*\]\([^\)]*$/u,
];

export const STREAM_PRIME_DELAY_MS = 140;
export const STREAM_PLAYBACK_INTERVAL_MS = 70;

function hideIncompleteCodeBlocks(
  text: string,
): { text: string; isWritingCode: boolean } {
  const lines = text.split("\n");
  let inCodeBlock = false;
  let codeBlockMarker = "";
  let startLineIndex = -1;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const match = line.match(/^\s*(`{3,})/u);

    if (!inCodeBlock) {
      if (match) {
        inCodeBlock = true;
        codeBlockMarker = match[1];
        startLineIndex = i;
      }
      continue;
    }

    if (line.trim().startsWith(codeBlockMarker)) {
      inCodeBlock = false;
      codeBlockMarker = "";
    }
  }

  if (!inCodeBlock) {
    return { text, isWritingCode: false };
  }

  return {
    text: lines.slice(0, startLineIndex).join("\n"),
    isWritingCode: true,
  };
}

function hideDanglingMarkdown(text: string): string {
  let next = text;
  let changed = true;

  while (changed) {
    changed = false;
    for (const pattern of INCOMPLETE_LINK_PATTERNS) {
      const stripped = next.replace(pattern, "");
      if (stripped !== next) {
        next = stripped;
        changed = true;
      }
    }
  }

  return next;
}

export function getRenderableStreamingText(
  rawText: string,
): { text: string; isWritingCode: boolean } {
  const { text, isWritingCode } = hideIncompleteCodeBlocks(rawText);
  return {
    text: hideDanglingMarkdown(text),
    isWritingCode,
  };
}

export function countRemainingStreamWords(
  rawText: string,
  cursor: number,
): number {
  const remaining = rawText.slice(cursor).trim();
  if (!remaining) return 0;
  return remaining.split(/\s+/u).filter(Boolean).length;
}

export function getStreamBatchSize(backlogWords: number): number {
  if (backlogWords >= 100) return 10;
  if (backlogWords >= 40) return 7;
  return 5;
}

export function advanceStreamCursorByWords(
  rawText: string,
  cursor: number,
  wordsToReveal: number,
): number {
  if (cursor >= rawText.length) return rawText.length;

  const remaining = rawText.slice(cursor);
  const wordRegex = /\S+/gu;
  let match: RegExpExecArray | null = null;
  let revealed = 0;
  let nextCursor = rawText.length;

  while (revealed < wordsToReveal) {
    match = wordRegex.exec(remaining);
    if (!match) break;
    revealed += 1;
    nextCursor = cursor + match.index + match[0].length;
  }

  while (nextCursor < rawText.length && /\s/u.test(rawText[nextCursor])) {
    nextCursor += 1;
  }

  return nextCursor;
}
