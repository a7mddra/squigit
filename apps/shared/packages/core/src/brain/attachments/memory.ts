/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

const LEGACY_ATTACHMENT_MENTION_RE = /\{\{([^}]+)\}\}/g;
const LINK_ATTACHMENT_MENTION_RE = /\[([^\]\n]+)\]\((<[^>\n]+>|[^)\n]+)\)/g;

type AttachmentMention = {
  path: string;
  label?: string;
};

function basename(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

function unwrapMarkdownLinkDestination(destination: string): string {
  const value = destination.trim();
  if (value.startsWith("<") && value.endsWith(">")) {
    return value.slice(1, -1).trim();
  }
  return value;
}

function isLikelyAttachmentPath(path: string): boolean {
  const value = unwrapMarkdownLinkDestination(path);
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
    const path = unwrapMarkdownLinkDestination(String(match[2] || ""));
    if (!isLikelyAttachmentPath(path) || seen.has(path)) continue;
    seen.add(path);
    out.push({
      path,
      label: label || undefined,
    });
  }

  return out;
}

function stripAttachmentMentionsForHistory(text: string): string {
  const withoutLegacy = text.replace(LEGACY_ATTACHMENT_MENTION_RE, "");
  const withoutLinks = withoutLegacy.replace(
    LINK_ATTACHMENT_MENTION_RE,
    (...args: unknown[]) => {
      const full = String(args[0] || "");
      const path = unwrapMarkdownLinkDestination(String(args[2] || ""));
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

function formatAttachmentReferences(mentions: AttachmentMention[]): string {
  if (mentions.length === 0) return "";
  const lines = mentions.map((item) => {
    const name = item.label?.trim() || basename(item.path);
    return `- \`${name}\`: \`${item.path}\``;
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
