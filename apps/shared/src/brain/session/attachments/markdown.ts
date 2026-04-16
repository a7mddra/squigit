/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { attachmentFromPath } from "./factory.ts";
import {
  getBaseName,
  isAttachmentPath,
  unwrapMarkdownLinkDestination,
  isAbsoluteCasPath,
} from "./paths.ts";

export const LEGACY_ATTACHMENT_MENTION_RE = /\{\{([^}]+)\}\}/g;
export const LINK_ATTACHMENT_MENTION_RE = /\[([^\]\n]+)\]\((<[^>\n]+>|[^)\n]+)\)/g;

function sanitizeAttachmentLabel(label: string): string {
  return label.replace(/[\[\]\n\r]/g, " ").trim();
}

export function formatAttachmentLinkDestination(path: string): string {
  const value = unwrapMarkdownLinkDestination(path);
  return isAttachmentPath(value) ? `<${value}>` : value;
}

export function isAbsoluteCasAttachmentMarkdownLink(
  label: string,
  destination: string,
): boolean {
  return Boolean(label.trim()) && isAbsoluteCasPath(destination);
}

export function normalizeAttachmentMarkdownLinks(text: string): string {
  return text.replace(
    LINK_ATTACHMENT_MENTION_RE,
    (full, label: string, rawDestination: string) => {
      const path = unwrapMarkdownLinkDestination(rawDestination);
      if (!isAttachmentPath(path)) {
        return full;
      }
      return buildAttachmentMention(path, label);
    },
  );
}

export function parseAttachmentPaths(text: string): string[] {
  const out: string[] = [];
  const seen = new Set<string>();

  for (const match of text.matchAll(LEGACY_ATTACHMENT_MENTION_RE)) {
    const path = (match[1] || "").trim();
    if (!path || seen.has(path)) continue;
    seen.add(path);
    out.push(path);
  }

  for (const match of text.matchAll(LINK_ATTACHMENT_MENTION_RE)) {
    const path = unwrapMarkdownLinkDestination(String(match[2] || ""));
    if (!isAttachmentPath(path) || seen.has(path)) continue;
    seen.add(path);
    out.push(path);
  }

  return out;
}

export function stripAttachmentMentions(text: string): string {
  const withoutLegacy = text.replace(LEGACY_ATTACHMENT_MENTION_RE, "");
  const withoutLinks = withoutLegacy.replace(
    LINK_ATTACHMENT_MENTION_RE,
    (full, _label, rawPath: string) => {
      const path = unwrapMarkdownLinkDestination(rawPath);
      return isAttachmentPath(path) ? "" : full;
    },
  );
  return withoutLinks.replace(/\n{3,}/g, "\n\n").trim();
}

export function stripImageAttachmentMentions(text: string): string {
  const withoutLegacyImages = text.replace(
    LEGACY_ATTACHMENT_MENTION_RE,
    (full, rawPath: string) => {
      const path = rawPath.trim();
      if (!path) return full;

      const attachment = attachmentFromPath(path);
      if (attachment.type === "image") {
        return "";
      }

      return buildAttachmentMention(path, attachment.name);
    },
  );

  const withoutImageLinks = withoutLegacyImages.replace(
    LINK_ATTACHMENT_MENTION_RE,
    (full, label: string, rawPath: string) => {
      const path = unwrapMarkdownLinkDestination(rawPath);
      if (!isAttachmentPath(path)) {
        return full;
      }

      const attachment = attachmentFromPath(
        path,
        undefined,
        label || undefined,
      );
      return attachment.type === "image"
        ? ""
        : buildAttachmentMention(path, label);
    },
  );

  return withoutImageLinks.replace(/\n{3,}/g, "\n\n").trim();
}

export function buildAttachmentMention(
  path: string,
  displayName?: string,
): string {
  const fallbackName = getBaseName(path);
  const label = sanitizeAttachmentLabel(
    displayName || fallbackName || "attachment",
  );
  const destination = formatAttachmentLinkDestination(path);
  return `[${label}](${destination})`;
}
