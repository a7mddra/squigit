/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type Attachment,
  getExtension,
  isImageExtension,
  unwrapMarkdownLinkDestination,
} from "@squigit/core/brain/session/attachments";
import { type MediaGalleryItem, type MediaViewerItem } from "@/features/media";

export type MediaViewerOpenOptions = {
  isGallery?: boolean;
  chatId?: string;
  galleryAttachments?: Attachment[];
  initialIndex?: number;
  openedFromChat?: boolean;
};

const UNSUPPORTED_PREVIEW_EXTENSIONS = new Set([
  "doc",
  "docx",
  "xls",
  "xlsx",
  "ppt",
  "pptx",
  "rtf",
  "odt",
  "ods",
  "odp",
  "pages",
  "numbers",
  "key",
]);

const ATTACHMENT_SOURCE_MAP_STORAGE_KEY = "squigit:attachment-source-map:v1";
const ATTACHMENT_SOURCE_MAP_MAX_ENTRIES = 2048;

function normalizeAttachmentPath(path: string): string {
  const unwrapped = unwrapMarkdownLinkDestination(path);

  try {
    return decodeURI(unwrapped);
  } catch {
    return unwrapped;
  }
}

function readAttachmentSourceMap(): Map<string, string> {
  if (typeof window === "undefined") return new Map();

  try {
    const raw = localStorage.getItem(ATTACHMENT_SOURCE_MAP_STORAGE_KEY);
    if (!raw) return new Map();

    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return new Map();
    }

    const map = new Map<string, string>();
    for (const [casPath, sourcePath] of Object.entries(parsed)) {
      if (typeof sourcePath === "string" && sourcePath.length > 0) {
        map.set(casPath, sourcePath);
      }
    }

    return map;
  } catch {
    return new Map();
  }
}

function persistAttachmentSourceMap(map: Map<string, string>) {
  if (typeof window === "undefined") return;

  try {
    const entries = Array.from(map.entries());
    const trimmedEntries =
      entries.length > ATTACHMENT_SOURCE_MAP_MAX_ENTRIES
        ? entries.slice(entries.length - ATTACHMENT_SOURCE_MAP_MAX_ENTRIES)
        : entries;

    localStorage.setItem(
      ATTACHMENT_SOURCE_MAP_STORAGE_KEY,
      JSON.stringify(Object.fromEntries(trimmedEntries)),
    );
  } catch {
    // Ignore storage quota / JSON errors and keep in-memory map.
  }
}

export const useAppMedia = ({ attachments }: { attachments: Attachment[] }) => {
  const attachmentSourceMapRef = useRef<Map<string, string>>(
    readAttachmentSourceMap(),
  );
  const [mediaViewer, setMediaViewer] = useState<{
    isOpen: boolean;
    item: MediaViewerItem | null;
  }>({
    isOpen: false,
    item: null,
  });

  const rememberAttachmentSourcePath = useCallback(
    (casPath: string, sourcePath: string) => {
      if (!casPath || !sourcePath) return;
      attachmentSourceMapRef.current.set(casPath, sourcePath);
      persistAttachmentSourceMap(attachmentSourceMapRef.current);
    },
    [],
  );

  useEffect(() => {
    attachments.forEach((attachment) => {
      if (attachment.sourcePath) {
        rememberAttachmentSourcePath(attachment.path, attachment.sourcePath);
      }
    });
  }, [attachments, rememberAttachmentSourcePath]);

  const getAttachmentSourcePath = useCallback((path: string) => {
    const directMatch = attachmentSourceMapRef.current.get(path);
    if (directMatch) return directMatch;

    const fileName = path.split(/[/\\]/).pop();
    if (!fileName) return null;

    for (const [casPath, sourcePath] of attachmentSourceMapRef.current) {
      if (
        casPath.endsWith(`/${fileName}`) ||
        casPath.endsWith(`\\${fileName}`)
      ) {
        return sourcePath;
      }
    }

    return null;
  }, []);

  const closeMediaViewer = useCallback(() => {
    setMediaViewer((prev) => ({ ...prev, isOpen: false }));
  }, []);

  const revealInFileManager = useCallback(async (path: string) => {
    try {
      await invoke("reveal_in_file_manager", { path });
    } catch (error) {
      console.error("[media] Failed to reveal in file manager:", error);
      throw error;
    }
  }, []);

  const openMediaViewer = useCallback(
    async (attachment: Attachment, options?: MediaViewerOpenOptions) => {
      const normalizedAttachmentPath = normalizeAttachmentPath(attachment.path);
      const pathExtension = getExtension(
        normalizedAttachmentPath,
      ).toLowerCase();
      const extension =
        pathExtension && pathExtension !== "file"
          ? pathExtension
          : attachment.extension.toLowerCase();
      const sourcePath =
        attachment.sourcePath ||
        getAttachmentSourcePath(normalizedAttachmentPath) ||
        getAttachmentSourcePath(attachment.path) ||
        undefined;

      let resolvedPath = normalizedAttachmentPath;
      try {
        resolvedPath = await invoke<string>("resolve_attachment_path", {
          path: normalizedAttachmentPath,
        });
      } catch (error) {
        console.warn("[media] Could not resolve attachment path:", error);
      }

      const revealPath = sourcePath || resolvedPath;

      if (UNSUPPORTED_PREVIEW_EXTENSIONS.has(extension)) {
        try {
          await revealInFileManager(revealPath);
        } catch {
          if (revealPath !== resolvedPath) {
            await revealInFileManager(resolvedPath);
          }
        }
        return;
      }

      if (attachment.type === "image" || isImageExtension(extension)) {
        const galleryAttachments =
          options?.galleryAttachments?.filter(
            (entry) =>
              entry.type === "image" || isImageExtension(entry.extension),
          ) ?? [];
        let galleryItems: MediaGalleryItem[] | undefined;
        let galleryIndex: number | undefined;

        if (galleryAttachments.length > 0) {
          const resolvedGallery = await Promise.all(
            galleryAttachments.map(async (galleryAttachment) => {
              const gallerySourcePath =
                galleryAttachment.sourcePath ||
                getAttachmentSourcePath(galleryAttachment.path) ||
                undefined;

              let galleryResolvedPath = galleryAttachment.path;
              try {
                galleryResolvedPath = await invoke<string>(
                  "resolve_attachment_path",
                  {
                    path: galleryAttachment.path,
                  },
                );
              } catch (error) {
                console.warn("[media] Could not resolve gallery path:", error);
              }

              return {
                path: galleryResolvedPath,
                sourcePath: gallerySourcePath,
                name: galleryAttachment.name,
                extension: galleryAttachment.extension.toLowerCase(),
              };
            }),
          );

          if (resolvedGallery.length > 0) {
            galleryItems = resolvedGallery;
            const fallbackIndex = galleryAttachments.findIndex(
              (entry) =>
                entry.id === attachment.id || entry.path === attachment.path,
            );
            const initialIndex =
              typeof options?.initialIndex === "number"
                ? options.initialIndex
                : fallbackIndex >= 0
                  ? fallbackIndex
                  : 0;
            galleryIndex = Math.max(
              0,
              Math.min(initialIndex, resolvedGallery.length - 1),
            );
          }
        }

        const activeGalleryItem =
          galleryItems && typeof galleryIndex === "number"
            ? galleryItems[galleryIndex]
            : undefined;

        setMediaViewer({
          isOpen: true,
          item: {
            kind: "image",
            path: activeGalleryItem?.path || resolvedPath,
            sourcePath: activeGalleryItem?.sourcePath || sourcePath,
            name: activeGalleryItem?.name || attachment.name,
            extension: activeGalleryItem?.extension || extension,
            isGallery:
              options?.isGallery === true || (galleryItems?.length ?? 0) > 1,
            galleryChatId: options?.chatId,
            galleryItems,
            galleryIndex,
            openedFromChat: options?.openedFromChat === true,
          },
        });
        return;
      }

      if (extension === "pdf") {
        setMediaViewer({
          isOpen: true,
          item: {
            kind: "pdf",
            path: resolvedPath,
            sourcePath,
            name: attachment.name,
            extension,
          },
        });
        return;
      }

      try {
        const textContent = await invoke<string>("read_attachment_text", {
          path: resolvedPath,
        });

        setMediaViewer({
          isOpen: true,
          item: {
            kind: "text",
            path: resolvedPath,
            sourcePath,
            name: attachment.name,
            extension,
            textContent,
          },
        });
      } catch (error) {
        console.warn("[media] Falling back to file-manager reveal:", error);
        try {
          await revealInFileManager(revealPath);
        } catch {
          if (revealPath !== resolvedPath) {
            await revealInFileManager(resolvedPath);
          }
        }
      }
    },
    [getAttachmentSourcePath, revealInFileManager],
  );

  return {
    mediaViewer,
    rememberAttachmentSourcePath,
    getAttachmentSourcePath,
    openMediaViewer,
    closeMediaViewer,
  };
};
