/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { getImagePath, type ChatMetadata, type Attachment } from "@/core";
import { useMediaContext } from "@/app/context/AppMedia";
import { Thumbnail } from "./components/Thumbnail";
import styles from "./Gallery.module.css";

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
    if (
      !existing ||
      toTimestamp(candidate.updatedAt) > toTimestamp(existing.updatedAt)
    ) {
      byHash.set(hash, candidate);
    }
  }

  return Array.from(byHash.values()).sort(
    (a, b) => toTimestamp(b.updatedAt) - toTimestamp(a.updatedAt),
  );
};

interface GalleryProps {
  chats: ChatMetadata[];
  activeSessionId: string | null;
  refreshChats: () => Promise<void> | void;
}

export const Gallery: React.FC<GalleryProps> = ({
  chats,
  activeSessionId,
  refreshChats,
}) => {
  const { openMediaViewer } = useMediaContext();
  const [isLoading, setIsLoading] = useState(true);
  const [items, setItems] = useState<GalleryImage[]>([]);

  const candidates = useMemo(() => selectLatestByHash(chats), [chats]);

  useEffect(() => {
    if (activeSessionId !== SYSTEM_GALLERY_ID) return;
    void refreshChats();
  }, [activeSessionId, refreshChats]);

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
      setItems(
        resolved.filter((entry): entry is GalleryImage => entry !== null),
      );
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
      const viewerName = item.title.trim() || "Image";
      const attachment: Attachment = {
        id: `gallery-${item.hash}`,
        type: "image",
        name: viewerName,
        extension,
        path: item.path,
      };

      void openMediaViewer(attachment, {
        isGallery: true,
        chatId: item.chatId,
      });
    },
    [openMediaViewer],
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
