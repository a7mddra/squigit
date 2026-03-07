/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import type { Root, RootContent, PhrasingContent } from "mdast";

export interface StreamSegment {
  type:
    | "text"
    | "bold"
    | "italic"
    | "code"
    | "codeblock"
    | "math"
    | "mathblock"
    | "heading"
    | "listItem"
    | "link"
    | "paragraph"
    | "blockquote"
    | "break";
  content: string;
  meta?: {
    language?: string;
    level?: number;
    href?: string;
    ordered?: boolean;
    listIndex?: number;
  };
}

export function parseMarkdownToSegments(markdown: string): StreamSegment[] {
  const segments: StreamSegment[] = [];
  const lines = markdown.split("\n");

  let inCodeBlock = false;
  let codeBlockDepth = 0;
  let codeBlockFence = "";
  let codeBlockLang = "";
  let codeBlockContent: string[] = [];

  let inMathBlock = false;
  let mathBlockContent: string[] = [];

  let textBuffer: string[] = [];

  const flushTextBuffer = () => {
    if (textBuffer.length > 0) {
      const text = textBuffer.join("\n") + "\n";
      const textSegments = parseMarkdownText(text);
      segments.push(...textSegments);
      textBuffer = [];
    }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (!inCodeBlock) {
      const trimmed = line.trim();
      if (trimmed.startsWith("$$")) {
        if (!inMathBlock) {
          flushTextBuffer();
          const rest = trimmed.slice(2);
          if (rest.endsWith("$$") && rest.length > 0) {
            segments.push({
              type: "mathblock",
              content: rest.slice(0, -2),
            });
          } else {
            inMathBlock = true;
            mathBlockContent = [];
            if (rest.trim()) mathBlockContent.push(rest);
          }
          continue;
        } else {
          const contentBefore = trimmed.slice(2).trim();
          if (contentBefore) mathBlockContent.push(contentBefore);
          segments.push({
            type: "mathblock",
            content: mathBlockContent.join("\n"),
          });
          inMathBlock = false;
          mathBlockContent = [];
          continue;
        }
      }

      if (inMathBlock) {
        mathBlockContent.push(line);
        continue;
      }
    }

    const fenceMatch = line.match(/^(\s*)(`{3,})(.*)$/);

    if (fenceMatch) {
      const fence = fenceMatch[2];
      const lang = fenceMatch[3].trim();

      if (!inCodeBlock) {
        flushTextBuffer();
        inCodeBlock = true;
        codeBlockDepth = 1;
        codeBlockFence = fence;
        codeBlockLang = lang;
        codeBlockContent = [];
      } else {
        if (lang !== "") {
          codeBlockDepth++;
          codeBlockContent.push(line);
        } else {
          if (fence.length >= codeBlockFence.length) {
            codeBlockDepth--;
          } else {
            codeBlockContent.push(line);
            continue;
          }

          if (codeBlockDepth === 0) {
            segments.push({
              type: "codeblock",
              content: codeBlockContent.join("\n"),
              meta: { language: codeBlockLang },
            });
            inCodeBlock = false;
            codeBlockFence = "";
            codeBlockLang = "";
            codeBlockDepth = 0;
            codeBlockContent = [];
          } else {
            codeBlockContent.push(line);
          }
        }
      }
    } else {
      if (inCodeBlock) {
        codeBlockContent.push(line);
      } else {
        textBuffer.push(line);
      }
    }
  }

  if (inMathBlock) {
    segments.push({
      type: "mathblock",
      content: mathBlockContent.join("\n"),
    });
  } else if (inCodeBlock) {
    segments.push({
      type: "codeblock",
      content: codeBlockContent.join("\n"),
      meta: { language: codeBlockLang },
    });
  } else {
    if (textBuffer.length > 0) {
      const text = textBuffer.join("\n");
      const textSegments = parseMarkdownText(text);
      segments.push(...textSegments);
    }
  }

  return segments;
}

function parseMarkdownText(markdown: string): StreamSegment[] {
  const processor = unified().use(remarkParse).use(remarkGfm).use(remarkMath);
  const tree = processor.parse(markdown) as Root;
  const segments: StreamSegment[] = [];

  processNode(tree, segments);
  return segments;
}

function processNode(
  node: Root | RootContent | PhrasingContent,
  segments: StreamSegment[],
  inheritedStyle?: "bold" | "italic",
  listMeta?: { ordered: boolean; index: number },
): void {
  switch (node.type) {
    case "root":
      for (const child of node.children) {
        processNode(child, segments);
      }
      break;

    case "paragraph":
      for (const child of node.children) {
        processNode(child, segments);
      }

      segments.push({ type: "text", content: "\n\n" });
      break;

    case "heading":
      segments.push({
        type: "heading",
        content: extractText(node),
        meta: { level: node.depth },
      });
      break;

    case "text":
      if (node.value.trim() || node.value === " ") {
        segments.push({
          type: inheritedStyle || "text",
          content: node.value,
        });
      }
      break;

    case "strong":
      for (const child of node.children) {
        processNode(child, segments, "bold");
      }
      break;

    case "emphasis":
      for (const child of node.children) {
        processNode(child, segments, "italic");
      }
      break;

    case "inlineCode":
      segments.push({
        type: "code",
        content: node.value,
      });
      break;

    case "code":
      segments.push({
        type: "codeblock",
        content: node.value,
        meta: { language: node.lang || "" },
      });
      break;

    case "link":
      segments.push({
        type: "link",
        content: extractText(node),
        meta: { href: node.url },
      });
      break;

    case "list":
      {
        const ordered = Boolean(node.ordered);
        let index = node.start ?? 1;

        for (const child of node.children) {
          processNode(child, segments, undefined, { ordered, index });
          index += 1;
        }

        segments.push({ type: "text", content: "\n" });
      }
      break;

    case "listItem":
      segments.push({
        type: "listItem",
        content: extractText(node),
        meta: {
          ordered: listMeta?.ordered ?? false,
          listIndex: listMeta?.index,
        },
      });
      segments.push({ type: "text", content: "\n" });
      break;

    case "blockquote":
      segments.push({
        type: "blockquote",
        content: extractText(node),
      });
      segments.push({ type: "text", content: "\n\n" });
      break;

    case "break":
    case "thematicBreak":
      segments.push({ type: "break", content: "" });
      break;

    case "math":
      if ("value" in node && typeof node.value === "string") {
        segments.push({ type: "mathblock", content: node.value });
        segments.push({ type: "text", content: "\n\n" });
      }
      break;

    case "inlineMath":
      if ("value" in node && typeof node.value === "string") {
        segments.push({ type: "math", content: node.value });
      }
      break;

    default:
      if ("children" in node && Array.isArray(node.children)) {
        for (const child of node.children) {
          processNode(child as RootContent | PhrasingContent, segments);
        }
      } else if ("value" in node && typeof node.value === "string") {
        segments.push({ type: "text", content: node.value });
      }
  }
}

export function preprocessMarkdown(
  markdown: string,
  options: { doubleNewlines?: boolean } = {},
): string {
  const lines = markdown.split("\n");
  const resultLines: string[] = [];

  let codeBlockDepth = 0;
  let inCodeBlock = false;
  let codeBlockFence = "";
  let codeBlockLang = "";
  let codeBlockContent: string[] = [];
  let textBuffer: string[] = [];

  const flushTextBuffer = () => {
    if (textBuffer.length > 0) {
      let text = textBuffer.join("\n");
      if (options.doubleNewlines) {
        text = text.replace(/\n/g, "\n\n");
      }
      resultLines.push(text);
      textBuffer = [];
    }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    const fenceMatch = line.match(/^(\s*)(`{3,})(.*)$/);

    if (fenceMatch) {
      const indent = fenceMatch[1];
      const fence = fenceMatch[2];
      const lang = fenceMatch[3].trim();

      if (!inCodeBlock) {
        flushTextBuffer();
        inCodeBlock = true;
        codeBlockDepth = 1;
        codeBlockFence = fence;
        codeBlockLang = lang;
        codeBlockContent = [];
      } else {
        if (lang !== "") {
          codeBlockDepth++;
          codeBlockContent.push(line);
        } else {
          if (fence.length >= codeBlockFence.length) {
            codeBlockDepth--;
          } else {
            codeBlockContent.push(line);
            continue;
          }

          if (codeBlockDepth === 0) {
            const content = codeBlockContent.join("\n");

            const innerFences = content.match(/`{3,}/g);
            let maxFenceLength = 0;
            if (innerFences) {
              maxFenceLength = Math.max(...innerFences.map((f) => f.length));
            }

            let outerFence = codeBlockFence;
            if (maxFenceLength >= outerFence.length) {
              outerFence = "`".repeat(maxFenceLength + 1);
            }

            resultLines.push(`${indent}${outerFence}${codeBlockLang}`);
            resultLines.push(content);
            resultLines.push(`${indent}${outerFence}`);

            inCodeBlock = false;
            codeBlockFence = "";
            codeBlockLang = "";
            codeBlockDepth = 0;
            codeBlockContent = [];
          } else {
            codeBlockContent.push(line);
          }
        }
      }
    } else {
      if (inCodeBlock) {
        codeBlockContent.push(line);
      } else {
        textBuffer.push(line);
      }
    }
  }

  if (inCodeBlock) {
    const content = codeBlockContent.join("\n");
    const innerFences = content.match(/`{3,}/g);
    let maxFenceLength = 0;
    if (innerFences) {
      maxFenceLength = Math.max(...innerFences.map((f) => f.length));
    }
    let outerFence = codeBlockFence;
    if (maxFenceLength >= outerFence.length) {
      outerFence = "`".repeat(maxFenceLength + 1);
    }

    resultLines.push(`${outerFence}${codeBlockLang}`);
    resultLines.push(content);
    resultLines.push(`${outerFence}`);
  } else {
    if (textBuffer.length > 0) {
      let text = textBuffer.join("\n");
      if (options.doubleNewlines) {
        text = text.replace(/\n/g, "\n\n");
      }
      resultLines.push(text);
    }
  }

  return resultLines.join("\n");
}

function extractText(node: any): string {
  if (node.type === "text") {
    return node.value;
  }
  if (node.type === "inlineMath") {
    return `$${node.value}$`;
  }
  if (node.type === "math") {
    return `$$\n${node.value}\n$$`;
  }
  if (node.type === "inlineCode") {
    return node.value;
  }
  if (node.type === "break") {
    return "\n";
  }
  if (node.type === "thematicBreak") {
    return "\n---\n";
  }
  if (node.children) {
    const joiner = node.type === "list" || node.type === "listItem" ? "\n" : "";
    return node.children.map(extractText).join(joiner);
  }
  return "";
}
