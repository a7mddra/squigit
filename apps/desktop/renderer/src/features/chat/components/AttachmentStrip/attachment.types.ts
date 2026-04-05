/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

/** Image extensions — used to determine thumbnail style */
export const IMAGE_EXTENSIONS = [
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "bmp",
  "svg",
];

/** All accepted extensions for the file dialog */
export const ACCEPTED_EXTENSIONS = [
  ...IMAGE_EXTENSIONS,
  "txt",
  "md",
  "csv",
  "json",
  "xml",
  "yaml",
  "yml",
  "html",
  "css",
  "js",
  "ts",
  "jsx",
  "tsx",
  "py",
  "rs",
  "go",
  "java",
  "c",
  "cpp",
  "h",
  "hpp",
  "pdf",
  "docx",
  "doc",
  "xlsx",
  "xls",
  "pptx",
  "ppt",
  "rtf",
];

export interface Attachment {
  /** Unique ID per instance */
  id: string;
  /** "image" or "file" */
  type: "image" | "file";
  /** Display name (filename) */
  name: string;
  /** Extension without leading dot, e.g. "png", "docx" */
  extension: string;
  /** Absolute path on disk (CAS path or /tmp path) */
  path: string;
  /** Original source path on disk (if selected/imported by user) */
  sourcePath?: string;
  /** True for capture-to-input temporary files in /tmp */
  isTemp?: boolean;
}

/** Determine if a file extension is an image */
export function isImageExtension(ext: string): boolean {
  return IMAGE_EXTENSIONS.includes(ext.toLowerCase());
}

export function getExtension(name: string): string {
  const lastDot = name.lastIndexOf(".");
  if (lastDot <= 0) return "file";
  return name.slice(lastDot + 1).toLowerCase();
}

const LEGACY_ATTACHMENT_MENTION_RE = /\{\{([^}]+)\}\}/g;
const LINK_ATTACHMENT_MENTION_RE = /\[([^\]\n]+)\]\(([^)\n]+)\)/g;

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

/** Parse attachment paths from both legacy `{{path}}` and link-style `[name](path)` mentions. */
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
    const path = (match[2] || "").trim();
    if (!isLikelyAttachmentPath(path) || seen.has(path)) continue;
    seen.add(path);
    out.push(path);
  }

  return out;
}

/** Strip attachment mention tokens from message text for display */
export function stripAttachmentMentions(text: string): string {
  const withoutLegacy = text.replace(LEGACY_ATTACHMENT_MENTION_RE, "");
  const withoutLinks = withoutLegacy.replace(
    LINK_ATTACHMENT_MENTION_RE,
    (full, _label, path: string) => {
      return isLikelyAttachmentPath(path) ? "" : full;
    },
  );
  return withoutLinks.replace(/\n{3,}/g, "\n\n").trim();
}

/** Build a user-friendly attachment mention token that still carries the real file path */
export function buildAttachmentMention(path: string, displayName?: string): string {
  const fallbackName = (() => {
    const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
  })();
  const label = (displayName || fallbackName || "attachment")
    .replace(/[\[\]\n\r]/g, " ")
    .trim();
  return `[${label}](${path})`;
}

/** Build an Attachment object from a file system path */
export function attachmentFromPath(
  path: string,
  id?: string,
  originalName?: string,
  sourcePath?: string,
): Attachment {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  const name =
    originalName || (lastSlash >= 0 ? path.slice(lastSlash + 1) : path);
  const ext = getExtension(name);
  const type = isImageExtension(ext) ? "image" : "file";
  return {
    id: id ?? `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    type,
    name,
    extension: ext,
    path,
    sourcePath,
    isTemp: path.startsWith("/tmp/"),
  };
}
