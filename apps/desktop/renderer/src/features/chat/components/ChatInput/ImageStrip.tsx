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
import { CloseCrossIcon } from "@/components/icons";
import type { Attachment } from "@squigit/core/brain/session/attachments";
import styles from "./ImageStrip.module.css";

const MIN_THUMB_WIDTH = 30;

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
    <CloseCrossIcon size={24} className={styles.cancelIcon} ariaHidden />
  </button>
);

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
      e.preventDefault();
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

interface ImageStripProps {
  attachments: Attachment[];
  onRemove?: (id: string) => void;
  onClick?: (attachment: Attachment) => void;
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

export const ImageStrip: React.FC<ImageStripProps> = ({
  attachments,
  onRemove,
  onClick,
}) => {
  const stripRef = useRef<HTMLDivElement>(null);
  const [scrollState, setScrollState] =
    useState<ScrollState>(INITIAL_SCROLL_STATE);

  const imageAttachments = useMemo(
    () => attachments.filter((attachment) => attachment.type === "image"),
    [attachments],
  );

  const displayAttachments = useMemo(() => {
    let unnamedImageIndex = 0;
    return imageAttachments.map((attachment) => {
      if (!shouldUseGeneratedImageName(attachment)) {
        return attachment;
      }

      unnamedImageIndex += 1;
      return {
        ...attachment,
        name: `image-${unnamedImageIndex}.${attachment.extension}`,
      };
    });
  }, [imageAttachments]);

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

  useLayoutEffect(() => {
    updateScroll();
  }, [updateScroll, displayAttachments]);

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

  if (displayAttachments.length === 0) return null;

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
            <ImageThumbnail attachment={attachment} />

            {onRemove && (
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
