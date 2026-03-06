/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useRef, useState } from "react";
import { Minus, Plus, RotateCcw } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import styles from "./MediaImageViewer.module.css";

interface MediaImageViewerProps {
  filePath: string;
  name: string;
  isGallery?: boolean;
}

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 0.2;

export const MediaImageViewer: React.FC<MediaImageViewerProps> = ({
  filePath,
  name,
  isGallery = false,
}) => {
  const src = useMemo(() => convertFileSrc(filePath), [filePath]);

  const [zoom, setZoom] = useState(1);
  const [isDragging, setIsDragging] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

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

  const clampZoom = (value: number) =>
    Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, value));

  const updateZoom = (next: number) => {
    setZoom(clampZoom(next));
  };

  const clearSelection = () => {
    const selection = window.getSelection();
    if (selection && selection.rangeCount > 0) {
      selection.removeAllRanges();
    }
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
    updateZoom(zoom + delta);
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
      data-gallery={isGallery ? "true" : "false"}
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
      </div>

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
            alt={name}
            className={styles.previewImage}
            draggable={false}
          />
        </div>
      </div>
    </div>
  );
};
