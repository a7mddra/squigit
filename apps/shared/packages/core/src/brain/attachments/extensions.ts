/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export const IMAGE_EXTENSION_VALUES = [
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "bmp",
  "svg",
] as const;

export const ACCEPTED_EXTENSION_VALUES = [
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
