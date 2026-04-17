/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export const ABSOLUTE_CAS_PATH_RE =
  /(?:^|[\\/])chats[\\/]objects[\\/][0-9a-f]{2}[\\/][0-9a-f]{64}\.[^/\\>\n]+$/iu;

export function getBaseName(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

export function unwrapMarkdownLinkDestination(destination: string): string {
  const value = destination.trim();
  if (value.startsWith("<") && value.endsWith(">")) {
    return value.slice(1, -1).trim();
  }
  return value;
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

export function isAbsoluteCasPath(path: string): boolean {
  const value = unwrapMarkdownLinkDestination(path);
  if (!value) return false;

  const isAbsolute =
    value.startsWith("/") ||
    value.startsWith("\\\\") ||
    /^[a-zA-Z]:[\\/]/.test(value);

  return isAbsolute && ABSOLUTE_CAS_PATH_RE.test(value);
}
