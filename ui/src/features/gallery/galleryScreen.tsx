/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { getImagePath, type ChatMetadata } from "@/lib";
import type { Attachment } from "@/features/chat/components/AttachmentStrip/attachment.types";
import { useAppContext } from "@/providers/AppProvider";
import { Thumbnail } from "./components/thumbnail/thumbnail";
import styles from "./galleryScreen.module.css";

const SYSTEM_GALLERY_ID = "__system_gallery";

interface GalleryCandidate {
  hash: string;
  chatId: string;
  title: string;
  updatedAt: string;
}

interface GalleryImage extends GalleryCandidate {
  path: string;
}

const toTimestamp = (value: string) => new Date(value || 0).getTime();

const selectLatestByHash = (chats: ChatMetadata[]): GalleryCandidate[] => {
  const byHash = new Map<string, GalleryCandidate>();

  for (const chat of chats) {
    const hash = chat.image_hash?.trim();
    if (!hash) continue;

    const candidate: GalleryCandidate = {
      hash,
      chatId: chat.id,
      title: chat.title || "Untitled",
      updatedAt: chat.updated_at || chat.created_at,
    };

    const existing = byHash.get(hash);
    if (!existing || toTimestamp(candidate.updatedAt) > toTimestamp(existing.updatedAt)) {
      byHash.set(hash, candidate);
    }
  }

  return Array.from(byHash.values()).sort(
    (a, b) => toTimestamp(b.updatedAt) - toTimestamp(a.updatedAt),
  );
};

export const GalleryScreen: React.FC = () => {
  const app = useAppContext();
  const [isLoading, setIsLoading] = useState(true);
  const [items, setItems] = useState<GalleryImage[]>([]);

  const candidates = useMemo(
    () => selectLatestByHash(app.chatHistory.chats),
    [app.chatHistory.chats],
  );

  useEffect(() => {
    if (app.chatHistory.activeSessionId !== SYSTEM_GALLERY_ID) return;
    void app.chatHistory.refreshChats();
  }, [
    app.chatHistory.activeSessionId,
    app.chatHistory.refreshChats,
  ]);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setIsLoading(true);

      const resolved = await Promise.all(
        candidates.map(async (candidate) => {
          try {
            const path = await getImagePath(candidate.hash);
            return { ...candidate, path };
          } catch {
            return null;
          }
        }),
      );

      if (cancelled) return;
      setItems(resolved.filter((entry): entry is GalleryImage => entry !== null));
      setIsLoading(false);
    };

    void load();
    return () => {
      cancelled = true;
    };
  }, [candidates]);

  const handleOpenImage = useCallback(
    (item: GalleryImage) => {
      const extension = item.path.split(".").pop()?.toLowerCase() || "png";
      const attachment: Attachment = {
        id: `gallery-${item.hash}`,
        type: "image",
        name: `${item.hash}.${extension}`,
        extension,
        path: item.path,
      };

      void app.openMediaViewer(attachment, {
        isGallery: true,
        chatId: item.chatId,
      });
    },
    [app],
  );

  if (isLoading) {
    return <div className={styles.state}>Loading images...</div>;
  }

  if (items.length === 0) {
    return <div className={styles.state}>No images yet.</div>;
  }

  return (
    <section className={styles.screen}>
      <header className={styles.header}>
        <h2 className={styles.title}>Images</h2>
        <p className={styles.subtitle}>{items.length} image(s)</p>
      </header>

      <div className={styles.grid}>
        {items.map((item) => (
          <Thumbnail
            key={`${item.hash}-${item.chatId}`}
            imagePath={item.path}
            title={item.title}
            onClick={() => handleOpenImage(item)}
          />
        ))}
      </div>
    </section>
  );
};
