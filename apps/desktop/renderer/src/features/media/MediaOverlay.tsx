/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WidgetOverlay } from "@/components";
import { AppContextMenu } from "@/layout";
import { GlobalWorkerOptions } from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { MediaSidebar } from "./components/MediaSidebar";
import { MediaImageViewer } from "./components/MediaImageViewer";
import { MediaPdfViewer } from "./components/MediaPdfViewer";
import { MediaTextViewer } from "./components/MediaTextViewer";
import type { MediaViewerItem } from "./media.types";
import styles from "./MediaOverlay.module.css";

GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

interface MediaOverlayProps {
  isOpen: boolean;
  onClose: () => void;
  item: MediaViewerItem | null;
  onRevealInChat?: (chatId: string) => void;
}

export const MediaOverlay: React.FC<MediaOverlayProps> = ({
  isOpen,
  onClose,
  item,
  onRevealInChat,
}) => {
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    hasSelection: boolean;
  } | null>(null);
  const isChatOpenedImage =
    item?.kind === "image" && item?.openedFromChat === true;

  const activePath = useMemo(
    () => item?.sourcePath || item?.path || "",
    [item?.sourcePath, item?.path],
  );

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const selection = window.getSelection();
    const hasSelection = !!selection && selection.toString().length > 0;

    setContextMenu({ x: e.clientX, y: e.clientY, hasSelection });
  };

  const handleCopySelection = () => {
    const selection = window.getSelection();
    if (selection) {
      navigator.clipboard.writeText(selection.toString());
    }
    setContextMenu(null);
  };

  const handleCopyPath = () => {
    if (!activePath) return;
    navigator.clipboard.writeText(activePath);
  };

  const handleReveal = async () => {
    if (!activePath) return;
    try {
      await invoke("reveal_in_file_manager", { path: activePath });
    } catch (error) {
      if (item && activePath !== item.path) {
        try {
          await invoke("reveal_in_file_manager", { path: item.path });
          return;
        } catch {
          // Continue to log original error.
        }
      }
      console.error("[MediaOverlay] Failed to reveal file:", error);
    }
  };

  const handleRevealInChat = () => {
    const chatId = item?.galleryChatId;
    if (!chatId || !onRevealInChat) return;
    onRevealInChat(chatId);
  };

  const renderViewer = () => {
    if (!item) return <div className={styles.emptyText}>No file selected.</div>;

    if (item.kind === "image") {
      return (
        <MediaImageViewer
          filePath={item.path}
          name={item.name}
          isGallery={item.isGallery === true}
          galleryItems={item.galleryItems}
          initialIndex={item.galleryIndex}
        />
      );
    }

    if (item.kind === "pdf") {
      return <MediaPdfViewer filePath={item.path} isOpen={isOpen} />;
    }

    return (
      <MediaTextViewer
        extension={item.extension}
        textContent={item.textContent ?? ""}
      />
    );
  };

  return (
    <>
      <WidgetOverlay
        isOpen={isOpen}
        onClose={onClose}
        onContextMenu={handleContextMenu}
        sectionContentClassName={styles.sectionContent}
        sidebarBottom={
          <MediaSidebar
            onReveal={handleReveal}
            onCopyPath={handleCopyPath}
            onRevealInChat={
              item?.isGallery && item.galleryChatId && !isChatOpenedImage
                ? handleRevealInChat
                : undefined
            }
          />
        }
      >
        <div className={styles.viewerRoot}>
          {!isChatOpenedImage && (
            <div className={styles.viewerHeader}>
              <h3 className={styles.viewerTitle}>{item?.name || "Viewer"}</h3>
            </div>
          )}
          <div className={styles.viewerBody}>{renderViewer()}</div>
        </div>
      </WidgetOverlay>
      {contextMenu && (
        <AppContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          onCopy={handleCopySelection}
          hasSelection={contextMenu.hasSelection}
        />
      )}
    </>
  );
};
