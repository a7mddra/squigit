/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { StreamSegment } from "./parser";

export interface StreamToken {
  segmentIndex: number;
  text: string;
  isFirst: boolean;
  isLast: boolean;
  delay: number;
}

const DELAYS = {
  text: 16,
  bold: 16,
  italic: 16,
  heading: 18,
  listItem: 18,
  link: 16,
  blockquote: 18,
  code: 15,
  codeblock: 3,
  math: 0,
  mathblock: 0,
  paragraph: 0,
  break: 0,
} as const;

function chunkText(
  content: string,
  options: { maxWords: number; maxChars: number },
): string[] {
  const units = content.match(/[^\s]+\s*|\s+/g) || [content];
  const chunks: string[] = [];
  let buffer = "";
  let words = 0;

  const flush = () => {
    if (buffer.length > 0) {
      chunks.push(buffer);
      buffer = "";
      words = 0;
    }
  };

  for (const unit of units) {
    const isWord = /\S/.test(unit);
    const wouldExceed =
      buffer.length > 0 &&
      ((isWord && words + 1 > options.maxWords) ||
        buffer.length + unit.length > options.maxChars);

    if (wouldExceed) {
      flush();
    }

    buffer += unit;
    if (isWord) {
      words += 1;
    }

    if (unit.includes("\n")) {
      flush();
    }
  }

  flush();
  return chunks.length > 0 ? chunks : [content];
}

export function tokenizeSegments(segments: StreamSegment[]): StreamToken[] {
  const tokens: StreamToken[] = [];

  segments.forEach((segment, segmentIndex) => {
    const baseDelay = DELAYS[segment.type] ?? 25;

    if (segment.type === "math" || segment.type === "mathblock") {
      if (segment.content.trim()) {
        tokens.push({
          segmentIndex,
          text: segment.content,
          isFirst: true,
          isLast: true,
          delay: 0,
        });
      }
      return;
    }

    if (segment.type === "codeblock") {
      const lines = segment.content.split("\n");
      lines.forEach((line, i) => {
        tokens.push({
          segmentIndex,
          text: i < lines.length - 1 ? line + "\n" : line,
          isFirst: i === 0,
          isLast: i === lines.length - 1,
          delay: baseDelay,
        });
      });
    } else if (segment.type === "break") {
      tokens.push({
        segmentIndex,
        text: "\n",
        isFirst: true,
        isLast: true,
        delay: 0,
      });
    } else if (segment.content.trim() === "") {
      if (segment.content) {
        tokens.push({
          segmentIndex,
          text: segment.content,
          isFirst: true,
          isLast: true,
          delay: 0,
        });
      }
    } else {
      const chunks =
        segment.type === "heading" ||
        segment.type === "listItem" ||
        segment.type === "blockquote"
          ? [segment.content]
          : chunkText(segment.content, { maxWords: 3, maxChars: 26 });

      chunks.forEach((chunk, i) => {
        tokens.push({
          segmentIndex,
          text: chunk,
          isFirst: i === 0,
          isLast: i === chunks.length - 1,
          delay: chunk.trim() ? baseDelay : 0,
        });
      });
    }
  });

  return tokens;
}
