/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export const validateImage = (file: File) => {
  const MAX = 20 * 1024 * 1024; // 20MB
  if (file.size > MAX) return "File too large (Max 20MB)";
  if (!["image/jpeg", "image/png", "image/webp"].includes(file.type)) {
    return "Unsupported format";
  }
  return null;
};

const IMAGE_EXTENSION_VALUES = [
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "bmp",
  "svg",
] as const;

const ACCEPTED_EXTENSION_VALUES = [
  ...IMAGE_EXTENSION_VALUES,
  "txt",
  "md",
  "csv",
  "json",
  "xml",
  "yaml",
  "yml",
  "toml",
  "ini",
  "cfg",
  "conf",
  "html",
  "css",
  "js",
  "ts",
  "jsx",
  "tsx",
  "sh",
  "bash",
  "zsh",
  "fish",
  "py",
  "rs",
  "go",
  "java",
  "c",
  "cpp",
  "h",
  "hpp",
  "sql",
  "log",
  "pdf",
  "docx",
  "doc",
  "xlsx",
  "xls",
  "pptx",
  "ppt",
  "rtf",
] as const;

export type ImageExtension = (typeof IMAGE_EXTENSION_VALUES)[number];
export type AcceptedExtension = (typeof ACCEPTED_EXTENSION_VALUES)[number];

export const IMAGE_EXTENSIONS: ImageExtension[] = [...IMAGE_EXTENSION_VALUES];
export const ACCEPTED_EXTENSIONS: AcceptedExtension[] = [
  ...ACCEPTED_EXTENSION_VALUES,
];
const IMAGE_EXTENSION_SET = new Set<string>(IMAGE_EXTENSION_VALUES);
const ACCEPTED_EXTENSION_SET = new Set<string>(ACCEPTED_EXTENSION_VALUES);

export interface Attachment {
  id: string;
  type: "image" | "file";
  name: string;
  extension: string;
  path: string;
  sourcePath?: string;
  isTemp?: boolean;
}

const LEGACY_ATTACHMENT_MENTION_RE = /\{\{([^}]+)\}\}/g;
const LINK_ATTACHMENT_MENTION_RE = /\[([^\]\n]+)\]\((<[^>\n]+>|[^)\n]+)\)/g;
const ABSOLUTE_CAS_PATH_RE =
  /(?:^|[\\/])chats[\\/]objects[\\/][0-9a-f]{2}[\\/][0-9a-f]{64}\.[^/\\>\n]+$/iu;

export function isImageExtension(ext: string): boolean {
  return IMAGE_EXTENSION_SET.has(ext.toLowerCase());
}

export function isAcceptedExtension(ext: string): ext is AcceptedExtension {
  return ACCEPTED_EXTENSION_SET.has(ext.toLowerCase());
}

export function getExtension(name: string): string {
  const lastDot = name.lastIndexOf(".");
  if (lastDot <= 0) return "file";
  return name.slice(lastDot + 1).toLowerCase();
}

function getBaseName(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

function sanitizeAttachmentLabel(label: string): string {
  return label.replace(/[\[\]\n\r]/g, " ").trim();
}

export function unwrapMarkdownLinkDestination(destination: string): string {
  const value = destination.trim();
  if (value.startsWith("<") && value.endsWith(">")) {
    return value.slice(1, -1).trim();
  }
  return value;
}

export function formatAttachmentLinkDestination(path: string): string {
  const value = unwrapMarkdownLinkDestination(path);
  return isAttachmentPath(value) ? `<${value}>` : value;
}

export function isAbsoluteCasPath(path: string): boolean {
  const value = unwrapMarkdownLinkDestination(path);
  if (!value) return false;

  const isAbsolute =
    value.startsWith("/") ||
    value.startsWith("\\\\") ||
    /^[a-zA-Z]:[\\/]/.test(value);

  return isAbsolute && ABSOLUTE_CAS_PATH_RE.test(value);
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

export function isAttachmentPath(path: string): boolean {
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

      const attachment = attachmentFromPath(path, undefined, label || undefined);
      return attachment.type === "image"
        ? ""
        : buildAttachmentMention(path, label);
    },
  );

  return withoutImageLinks.replace(/\n{3,}/g, "\n\n").trim();
}

export function buildAttachmentMention(path: string, displayName?: string): string {
  const fallbackName = getBaseName(path);
  const label = sanitizeAttachmentLabel(displayName || fallbackName || "attachment");
  const destination = formatAttachmentLinkDestination(path);
  return `[${label}](${destination})`;
}

export function attachmentFromPath(
  path: string,
  id?: string,
  originalName?: string,
  sourcePath?: string,
): Attachment {
  const name = originalName || getBaseName(path);
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
