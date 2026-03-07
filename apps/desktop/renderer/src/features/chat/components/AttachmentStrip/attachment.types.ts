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

/** Parse attachment paths from the {{...}} mention format */
export function parseAttachmentPaths(text: string): string[] {
  const matches = text.match(/\{\{([^}]+)\}\}/g) || [];
  return matches.map((m) => m.slice(2, -2));
}

/** Strip {{...}} tokens from message text for display */
export function stripAttachmentMentions(text: string): string {
  return text.replace(/\{\{[^}]+\}\}/g, "").trim();
}

/** Build a {{path}} mention token from a path */
export function buildAttachmentMention(path: string): string {
  return `{{${path}}}`;
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
