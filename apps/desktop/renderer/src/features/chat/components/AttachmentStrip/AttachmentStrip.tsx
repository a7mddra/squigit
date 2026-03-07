/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Attachment } from "./attachment.types";
import styles from "./AttachmentStrip.module.css";

// ─── Constants ─────────────────────────────────────────────────────────────

const MIN_THUMB_WIDTH = 30;

const EXT_COLORS: Record<string, string> = {
  // Documents
  pdf: "var(--c-raw-123)",
  doc: "var(--c-raw-069)",
  docx: "var(--c-raw-069)",
  xls: "var(--c-raw-066)",
  xlsx: "var(--c-raw-066)",
  ppt: "var(--c-raw-076)",
  pptx: "var(--c-raw-076)",
  rtf: "var(--c-raw-101)",
  // Text / Code
  txt: "var(--c-raw-117)",
  md: "var(--c-raw-098)",
  csv: "var(--c-raw-087)",
  json: "var(--c-raw-107)",
  xml: "var(--c-raw-111)",
  yaml: "var(--c-raw-063)",
  yml: "var(--c-raw-063)",
  html: "var(--c-raw-122)",
  css: "var(--c-raw-091)",
  js: "var(--c-raw-125)",
  ts: "var(--c-raw-093)",
  jsx: "var(--c-raw-074)",
  tsx: "var(--c-raw-074)",
  py: "var(--c-raw-094)",
  rs: "var(--c-raw-119)",
  go: "var(--c-raw-086)",
  java: "var(--c-raw-127)",
  c: "var(--c-raw-049)",
  cpp: "var(--c-raw-062)",
  h: "var(--c-raw-049)",
  hpp: "var(--c-raw-062)",
};

// ─── Helpers ────────────────────────────────────────────────────────────────

function getBadgeColor(ext: string): string {
  return EXT_COLORS[ext.toLowerCase()] ?? "#6b7280";
}

function shouldUseGeneratedImageName(attachment: Attachment): boolean {
  if (attachment.type !== "image") return false;
  if (attachment.sourcePath) return false;

  const name = attachment.name.trim();
  const ext = attachment.extension.toLowerCase();
  if (!name.toLowerCase().endsWith(`.${ext}`)) return false;

  const stem = name.slice(0, -(ext.length + 1)).toLowerCase();
  if (!stem) return true;
  if (stem.length >= 24 && /^[a-f0-9]+$/.test(stem)) return true;
  if (/^(image|img|capture|screenshot|clipboard|paste)[-_]?\d*$/.test(stem)) {
    return true;
  }

  return false;
}

function getThumbGeometry(
  scrollWidth: number,
  clientWidth: number,
  scrollLeft: number,
) {
  const thumbWidth = Math.max(
    (clientWidth / scrollWidth) * clientWidth,
    MIN_THUMB_WIDTH,
  );
  const scrollableTrack = clientWidth - thumbWidth;
  const scrollableContent = scrollWidth - clientWidth;
  const thumbLeft =
    scrollableContent > 0
      ? (scrollLeft / scrollableContent) * scrollableTrack
      : 0;
  return { thumbWidth, thumbLeft, scrollableTrack, scrollableContent };
}

// ─── Sub-components ─────────────────────────────────────────────────────────

const FileThumbnail: React.FC<{ attachment: Attachment }> = ({
  attachment,
}) => (
  <div className={styles.fileThumb}>
    <div
      className={styles.fileExtBadge}
      style={{ backgroundColor: getBadgeColor(attachment.extension) }}
    >
      {attachment.extension.toUpperCase().slice(0, 4)}
    </div>
    <div className={styles.fileInfo}>
      <span className={styles.fileName}>{attachment.name}</span>
    </div>
  </div>
);

const ImageThumbnail: React.FC<{ attachment: Attachment }> = ({
  attachment,
}) => {
  const src = attachment.path.startsWith("http")
    ? attachment.path
    : convertFileSrc(attachment.path);

  return (
    <div className={styles.imageThumb}>
      <img src={src} alt={attachment.name} draggable={false} />
    </div>
  );
};

const RemoveButton: React.FC<{
  name: string;
  onRemove: (e: React.MouseEvent) => void;
}> = ({ name, onRemove }) => (
  <button
    className={styles.removeBtn}
    onClick={onRemove}
    aria-label={`Remove ${name}`}
  >
    <svg
      className={styles.cancelIcon}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="3"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  </button>
);

// ─── Scrollbar thumb drag hook ───────────────────────────────────────────────

interface ScrollbarThumbProps {
  thumbWidth: number;
  thumbLeft: number;
  stripRef: React.RefObject<HTMLDivElement | null>;
  onScrollChange: () => void;
}

const ScrollbarThumb: React.FC<ScrollbarThumbProps> = ({
  thumbWidth,
  thumbLeft,
  stripRef,
  onScrollChange,
}) => {
  const isDragging = useRef(false);
  const startX = useRef(0);
  const scrollLeftStart = useRef(0);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      isDragging.current = true;
      startX.current = e.clientX;
      scrollLeftStart.current = stripRef.current?.scrollLeft ?? 0;
      e.currentTarget.setPointerCapture(e.pointerId);
      e.preventDefault(); // prevent text selection while dragging
    },
    [stripRef],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isDragging.current || !stripRef.current) return;
      const { scrollWidth, clientWidth } = stripRef.current;
      const { scrollableTrack, scrollableContent } = getThumbGeometry(
        scrollWidth,
        clientWidth,
        0,
      );

      if (scrollableTrack > 0) {
        const deltaX = e.clientX - startX.current;
        stripRef.current.scrollLeft =
          scrollLeftStart.current +
          deltaX * (scrollableContent / scrollableTrack);
        onScrollChange();
      }
    },
    [stripRef, onScrollChange],
  );

  const handlePointerUp = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      isDragging.current = false;
      e.currentTarget.releasePointerCapture(e.pointerId);
    },
    [],
  );

  return (
    <div className={styles.scrollbarTrack}>
      <div
        className={styles.scrollbarThumb}
        style={{ width: thumbWidth, transform: `translateX(${thumbLeft}px)` }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
      />
    </div>
  );
};

// ─── AttachmentStrip ─────────────────────────────────────────────────────────

export interface AttachmentStripProps {
  attachments: Attachment[];
  onRemove?: (id: string) => void;
  onClick?: (attachment: Attachment) => void;
  readOnly?: boolean;
}

interface ScrollState {
  canScroll: boolean;
  thumbWidth: number;
  thumbLeft: number;
}

const INITIAL_SCROLL_STATE: ScrollState = {
  canScroll: false,
  thumbWidth: 0,
  thumbLeft: 0,
};

export const AttachmentStrip: React.FC<AttachmentStripProps> = ({
  attachments,
  onRemove,
  onClick,
  readOnly = false,
}) => {
  const stripRef = useRef<HTMLDivElement>(null);
  const [scrollState, setScrollState] =
    useState<ScrollState>(INITIAL_SCROLL_STATE);

  const displayAttachments = useMemo(() => {
    let unnamedImageIndex = 0;
    return attachments.map((attachment) => {
      if (!shouldUseGeneratedImageName(attachment)) {
        return attachment;
      }
      unnamedImageIndex += 1;
      return {
        ...attachment,
        name: `image-${unnamedImageIndex}.${attachment.extension}`,
      };
    });
  }, [attachments]);

  const updateScroll = useCallback(() => {
    const el = stripRef.current;
    if (!el) return;

    const { scrollWidth, clientWidth, scrollLeft } = el;
    if (scrollWidth <= clientWidth) {
      setScrollState((prev) =>
        prev.canScroll === false ? prev : INITIAL_SCROLL_STATE,
      );
      return;
    }

    const { thumbWidth, thumbLeft } = getThumbGeometry(
      scrollWidth,
      clientWidth,
      scrollLeft,
    );
    setScrollState((prev) => {
      if (
        prev.canScroll === true &&
        prev.thumbWidth === thumbWidth &&
        prev.thumbLeft === thumbLeft
      ) {
        return prev;
      }
      return { canScroll: true, thumbWidth, thumbLeft };
    });
  }, []);

  // Measure synchronously after DOM mutations so the thumb is visible on first render.
  useLayoutEffect(() => {
    updateScroll();
  }, [updateScroll, attachments]);

  // Wire up resize + wheel listeners.
  useEffect(() => {
    const el = stripRef.current;
    if (!el) return;

    let rAF = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(rAF);
      rAF = requestAnimationFrame(() => {
        updateScroll();
      });
    });
    observer.observe(el);

    const handleWheel = (e: WheelEvent) => {
      const delta =
        Math.abs(e.deltaY) > Math.abs(e.deltaX) ? e.deltaY : e.deltaX;
      if (delta === 0) return;

      const prev = el.scrollLeft;
      el.scrollLeft += delta;
      if (el.scrollLeft !== prev) e.preventDefault();
      updateScroll();
    };

    window.addEventListener("resize", updateScroll);
    el.addEventListener("wheel", handleWheel, { passive: false });

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", updateScroll);
      el.removeEventListener("wheel", handleWheel);
    };
  }, [updateScroll]);

  const handleClick = useCallback(
    (attachment: Attachment) => {
      onClick?.(attachment);
    },
    [onClick],
  );

  const handleRemove = useCallback(
    (e: React.MouseEvent, id: string) => {
      e.stopPropagation();
      onRemove?.(id);
    },
    [onRemove],
  );

  if (attachments.length === 0) return null;

  return (
    <div className={styles.wrapper}>
      <div className={styles.strip} ref={stripRef} onScroll={updateScroll}>
        {displayAttachments.map((attachment) => (
          <div
            key={attachment.id}
            className={styles.thumbWrapper}
            onClick={() => handleClick(attachment)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") handleClick(attachment);
            }}
          >
            {attachment.type === "image" ? (
              <ImageThumbnail attachment={attachment} />
            ) : (
              <FileThumbnail attachment={attachment} />
            )}

            {!readOnly && onRemove && (
              <RemoveButton
                name={attachment.name}
                onRemove={(e) => handleRemove(e, attachment.id)}
              />
            )}
          </div>
        ))}
      </div>

      {scrollState.canScroll && (
        <ScrollbarThumb
          thumbWidth={scrollState.thumbWidth}
          thumbLeft={scrollState.thumbLeft}
          stripRef={stripRef}
          onScrollChange={updateScroll}
        />
      )}
    </div>
  );
};
