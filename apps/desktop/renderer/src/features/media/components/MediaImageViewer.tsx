/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ChevronLeft, ChevronRight, Minus, Plus, RotateCcw } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { MediaGalleryItem } from "../media.types";
import styles from "./MediaImageViewer.module.css";

interface MediaImageViewerProps {
  filePath: string;
  name: string;
  isGallery?: boolean;
  galleryItems?: MediaGalleryItem[];
  initialIndex?: number;
}

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 0.2;

interface ResolvedGalleryItem extends MediaGalleryItem {
  src: string;
}

const resolveMediaSource = (path: string) =>
  /^(?:https?:\/\/|data:)/iu.test(path) ? path : convertFileSrc(path);

const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

const getWrappedIndex = (index: number, total: number) =>
  ((index % total) + total) % total;

export const MediaImageViewer: React.FC<MediaImageViewerProps> = ({
  filePath,
  name,
  isGallery = false,
  galleryItems,
  initialIndex,
}) => {
  const resolvedGalleryItems = useMemo<ResolvedGalleryItem[]>(() => {
    const sourceItems =
      galleryItems && galleryItems.length > 0
        ? galleryItems
        : [
            {
              path: filePath,
              name,
              extension: "",
            },
          ];

    return sourceItems.map((item) => ({
      ...item,
      src: resolveMediaSource(item.path),
    }));
  }, [filePath, galleryItems, name]);

  const [activeIndex, setActiveIndex] = useState(() =>
    clamp(
      typeof initialIndex === "number" ? initialIndex : 0,
      0,
      Math.max(0, resolvedGalleryItems.length - 1),
    ),
  );

  const [zoom, setZoom] = useState(1);
  const [isDragging, setIsDragging] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const canUseGalleryNavigation =
    (isGallery || resolvedGalleryItems.length > 1) &&
    resolvedGalleryItems.length > 1;

  const activeItem = resolvedGalleryItems[activeIndex];
  const src = activeItem?.src || resolveMediaSource(filePath);
  const activeName = activeItem?.name || name;

  const dragRef = useRef<{
    active: boolean;
    x: number;
    y: number;
    scrollLeft: number;
    scrollTop: number;
  }>({
    active: false,
    x: 0,
    y: 0,
    scrollLeft: 0,
    scrollTop: 0,
  });

  useEffect(() => {
    const nextIndex = clamp(
      typeof initialIndex === "number" ? initialIndex : 0,
      0,
      Math.max(0, resolvedGalleryItems.length - 1),
    );
    setActiveIndex(nextIndex);
  }, [filePath, initialIndex, resolvedGalleryItems.length]);

  useEffect(() => {
    setZoom(1);
    setIsDragging(false);
    dragRef.current.active = false;
    if (scrollRef.current) {
      scrollRef.current.scrollLeft = 0;
      scrollRef.current.scrollTop = 0;
    }
  }, [src]);

  const goToIndex = useCallback(
    (nextIndex: number) => {
      if (!canUseGalleryNavigation) return;
      setActiveIndex((prevIndex) => {
        const current = clamp(
          prevIndex,
          0,
          Math.max(0, resolvedGalleryItems.length - 1),
        );
        if (nextIndex === current) return current;
        return getWrappedIndex(nextIndex, resolvedGalleryItems.length);
      });
    },
    [canUseGalleryNavigation, resolvedGalleryItems.length],
  );

  const goToPrevious = useCallback(() => {
    if (!canUseGalleryNavigation) return;
    setActiveIndex((prevIndex) =>
      getWrappedIndex(prevIndex - 1, resolvedGalleryItems.length),
    );
  }, [canUseGalleryNavigation, resolvedGalleryItems.length]);

  const goToNext = useCallback(() => {
    if (!canUseGalleryNavigation) return;
    setActiveIndex((prevIndex) =>
      getWrappedIndex(prevIndex + 1, resolvedGalleryItems.length),
    );
  }, [canUseGalleryNavigation, resolvedGalleryItems.length]);

  useEffect(() => {
    if (!canUseGalleryNavigation) return;

    const handleGlobalKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isTypingTarget =
        !!target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable);
      if (isTypingTarget) return;

      if (event.key === "ArrowLeft") {
        event.preventDefault();
        goToPrevious();
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        goToNext();
      }
    };

    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  }, [canUseGalleryNavigation, goToNext, goToPrevious]);

  const clearSelection = () => {
    const selection = window.getSelection();
    if (selection && selection.rangeCount > 0) {
      selection.removeAllRanges();
    }
  };

  const clampZoom = (value: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, value));

  const updateZoom = (next: number) => {
    setZoom(clampZoom(next));
  };

  const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!scrollRef.current) return;
    e.preventDefault();
    clearSelection();
    dragRef.current = {
      active: true,
      x: e.clientX,
      y: e.clientY,
      scrollLeft: scrollRef.current.scrollLeft,
      scrollTop: scrollRef.current.scrollTop,
    };
    setIsDragging(true);
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current.active || !scrollRef.current) return;

    const deltaX = e.clientX - dragRef.current.x;
    const deltaY = e.clientY - dragRef.current.y;

    scrollRef.current.scrollLeft = dragRef.current.scrollLeft - deltaX;
    scrollRef.current.scrollTop = dragRef.current.scrollTop - deltaY;
  };

  const handlePointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    dragRef.current.active = false;
    setIsDragging(false);
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
  };

  const handleWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    const delta = e.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP;
    setZoom((prev) => clampZoom(prev + delta));
  };

  const handleDragStart = (e: React.DragEvent<HTMLElement>) => {
    e.preventDefault();
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      clearSelection();
    }
  };

  return (
    <div
      className={styles.imageViewerRoot}
      data-gallery={canUseGalleryNavigation ? "true" : "false"}
    >
      <div className={styles.imageTools}>
        <button
          className={styles.toolButton}
          onClick={() => updateZoom(zoom - ZOOM_STEP)}
          title="Zoom out"
          aria-label="Zoom out"
        >
          <Minus size={14} />
        </button>
        <button
          className={styles.toolButton}
          onClick={() => setZoom(1)}
          title="Reset zoom"
          aria-label="Reset zoom"
        >
          <RotateCcw size={14} />
        </button>
        <button
          className={styles.toolButton}
          onClick={() => updateZoom(zoom + ZOOM_STEP)}
          title="Zoom in"
          aria-label="Zoom in"
        >
          <Plus size={14} />
        </button>
        <span className={styles.zoomLabel}>{Math.round(zoom * 100)}%</span>
        {canUseGalleryNavigation && (
          <span className={styles.galleryCounter}>
            {activeIndex + 1} / {resolvedGalleryItems.length}
          </span>
        )}
      </div>

      <div className={styles.viewerStage}>
        <div
          ref={scrollRef}
          className={styles.imageScrollArea}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
          onWheel={handleWheel}
          onDragStart={handleDragStart}
          onKeyDown={handleKeyDown}
          tabIndex={0}
          style={{ cursor: isDragging ? "grabbing" : "grab" }}
        >
          <div
            className={styles.imageTransformLayer}
            style={{ transform: `scale(${zoom})` }}
          >
            <img
              src={src}
              alt={activeName}
              className={styles.previewImage}
              draggable={false}
            />
          </div>
        </div>
        {canUseGalleryNavigation && (
          <>
            <button
              type="button"
              className={`${styles.navButton} ${styles.navButtonLeft}`}
              onClick={goToPrevious}
              aria-label="Previous image"
            >
              <ChevronLeft size={18} />
            </button>
            <button
              type="button"
              className={`${styles.navButton} ${styles.navButtonRight}`}
              onClick={goToNext}
              aria-label="Next image"
            >
              <ChevronRight size={18} />
            </button>
          </>
        )}
      </div>

      {canUseGalleryNavigation && (
        <div className={styles.thumbnailStrip} role="tablist" aria-label="Gallery thumbnails">
          {resolvedGalleryItems.map((item, index) => (
            <button
              key={`${item.path}-${index}`}
              type="button"
              className={`${styles.thumbnailButton} ${
                index === activeIndex ? styles.thumbnailButtonActive : ""
              }`}
              onClick={() => goToIndex(index)}
              aria-label={`Open image ${index + 1}`}
              aria-current={index === activeIndex ? "true" : undefined}
            >
              <img
                src={item.src}
                alt={item.name || `Image ${index + 1}`}
                className={styles.thumbnailImage}
                draggable={false}
              />
            </button>
          ))}
        </div>
      )}
    </div>
  );
};
