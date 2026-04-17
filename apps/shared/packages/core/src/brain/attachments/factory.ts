/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { getBaseName } from "./paths.ts";
import { getExtension, isImageExtension } from "./extensions.ts";
import type { Attachment } from "./types.ts";

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
