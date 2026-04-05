/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type MediaViewerKind = "image" | "pdf" | "text";

export interface MediaGalleryItem {
  path: string;
  sourcePath?: string;
  name: string;
  extension: string;
}

export interface MediaViewerItem {
  kind: MediaViewerKind;
  path: string;
  sourcePath?: string;
  name: string;
  extension: string;
  textContent?: string;
  isGallery?: boolean;
  galleryChatId?: string;
  galleryItems?: MediaGalleryItem[];
  galleryIndex?: number;
  openedFromChat?: boolean;
}
