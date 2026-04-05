/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { invoke } from "@tauri-apps/api/core";

const LEGACY_ATTACHMENT_MENTION_RE = /\{\{([^}]+)\}\}/g;
const LINK_ATTACHMENT_MENTION_RE = /\[([^\]\n]+)\]\(([^)\n]+)\)/g;
const ATTACHMENT_MEMORY_TIMEOUT_MS = 1600;

type AttachmentMention = {
  path: string;
  label?: string;
};

function basename(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

function isLikelyAttachmentPath(path: string): boolean {
  const value = path.trim();
  if (!value) return false;

  const lower = value.toLowerCase();
  if (
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("mailto:") ||
    lower.startsWith("tel:")
  ) {
    return false;
  }

  if (value.startsWith("/") || value.startsWith("\\\\")) return true;
  if (/^[a-zA-Z]:[\\/]/.test(value)) return true;

  return (
    value.startsWith("objects/") ||
    value.startsWith("./objects/") ||
    value.startsWith("../objects/") ||
    value.startsWith("tmp/") ||
    value.startsWith("/tmp/")
  );
}

function extractAttachmentMentions(text: string): AttachmentMention[] {
  const seen = new Set<string>();
  const out: AttachmentMention[] = [];

  for (const match of text.matchAll(LEGACY_ATTACHMENT_MENTION_RE)) {
    const path = (match[1] || "").trim();
    if (!path || seen.has(path)) continue;
    seen.add(path);
    out.push({ path });
  }

  for (const match of text.matchAll(LINK_ATTACHMENT_MENTION_RE)) {
    const label = (match[1] || "").trim();
    const path = (match[2] || "").trim();
    if (!isLikelyAttachmentPath(path) || seen.has(path)) continue;
    seen.add(path);
    out.push({
      path,
      label: label || undefined,
    });
  }

  return out;
}

export function extractAttachmentPaths(text: string): string[] {
  return extractAttachmentMentions(text).map((item) => item.path);
}

export function stripAttachmentMentionsForHistory(text: string): string {
  const withoutLegacy = text.replace(LEGACY_ATTACHMENT_MENTION_RE, "");
  const withoutLinks = withoutLegacy.replace(
    LINK_ATTACHMENT_MENTION_RE,
    (...args: unknown[]) => {
      const full = String(args[0] || "");
      const path = String(args[2] || "");
      return isLikelyAttachmentPath(path) ? "" : full;
    },
  );
  return withoutLinks.replace(/\n{3,}/g, "\n\n").trim();
}

function hasAttachmentContextBlock(text: string): boolean {
  return (
    /\[Attachment Context\]/i.test(text) ||
    /\[Attachment References\]/i.test(text)
  );
}

export function formatAttachmentReferences(mentions: AttachmentMention[]): string {
  if (mentions.length === 0) return "";
  const lines = mentions.map((item) => {
    const name = item.label?.trim() || basename(item.path);
    return `- \`${name}\``;
  });
  return `[Attachment References]\n${lines.join("\n")}`;
}

export function normalizeMessageForHistory(text: string): string {
  const mentions = extractAttachmentMentions(text);
  if (mentions.length === 0 || hasAttachmentContextBlock(text)) {
    return text;
  }

  const stripped = stripAttachmentMentionsForHistory(text);
  const refs = formatAttachmentReferences(mentions);
  if (!refs) return stripped || text;
  return stripped ? `${stripped}\n\n${refs}` : refs;
}

export async function buildRichUserHistoryContent(text: string): Promise<string> {
  const mentions = extractAttachmentMentions(text);
  if (mentions.length === 0 || hasAttachmentContextBlock(text)) {
    return text;
  }

  const stripped = stripAttachmentMentionsForHistory(text);

  try {
    const context = await Promise.race<string>([
      invoke<string>("build_attachment_memory_context", {
        attachments: mentions.map((item) => ({
          path: item.path,
          displayName: item.label ?? null,
        })),
      }),
      new Promise<string>((_, reject) => {
        setTimeout(
          () => reject(new Error("attachment-memory-timeout")),
          ATTACHMENT_MEMORY_TIMEOUT_MS,
        );
      }),
    ]);
    const normalizedContext = (context || "").trim();

    if (normalizedContext) {
      return stripped ? `${stripped}\n\n${normalizedContext}` : normalizedContext;
    }
  } catch (error) {
    console.warn("[AttachmentMemory] Failed to build rich attachment context:", error);
  }

  return normalizeMessageForHistory(text);
}
