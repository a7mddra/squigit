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
