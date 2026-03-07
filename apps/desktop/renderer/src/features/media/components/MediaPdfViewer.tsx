/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getDocument } from "pdfjs-dist";
import styles from "./MediaPdfViewer.module.css";

interface MediaPdfViewerProps {
  filePath: string;
  isOpen: boolean;
}

export const MediaPdfViewer: React.FC<MediaPdfViewerProps> = ({
  filePath,
  isOpen,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const pagesRef = useRef<HTMLDivElement>(null);

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pageCount, setPageCount] = useState(0);

  useEffect(() => {
    if (!isOpen || !containerRef.current || !pagesRef.current) return;

    let cancelled = false;
    const pagesEl = pagesRef.current;
    let loadingTask: ReturnType<typeof getDocument> | null = null;

    const renderPdf = async () => {
      setIsLoading(true);
      setError(null);
      setPageCount(0);
      pagesEl.innerHTML = "";

      try {
        loadingTask = getDocument({
          url: convertFileSrc(filePath),
          isEvalSupported: false,
        });

        const pdf = await loadingTask.promise;
        if (cancelled) return;

        const targetWidth = Math.max(
          containerRef.current!.clientWidth - 40,
          320,
        );
        setPageCount(pdf.numPages);

        for (let pageIndex = 1; pageIndex <= pdf.numPages; pageIndex += 1) {
          if (cancelled) return;

          const page = await pdf.getPage(pageIndex);
          const viewportAtOne = page.getViewport({ scale: 1 });
          const scale = targetWidth / viewportAtOne.width;
          const viewport = page.getViewport({ scale });

          const pageWrap = document.createElement("div");
          pageWrap.className = styles.pdfPage;

          const canvas = document.createElement("canvas");
          canvas.className = styles.pdfCanvas;

          const dpr = window.devicePixelRatio || 1;
          canvas.width = Math.max(1, Math.floor(viewport.width * dpr));
          canvas.height = Math.max(1, Math.floor(viewport.height * dpr));
          canvas.style.width = `${Math.floor(viewport.width)}px`;
          canvas.style.height = `${Math.floor(viewport.height)}px`;

          const ctx = canvas.getContext("2d");
          if (!ctx) continue;

          pageWrap.appendChild(canvas);
          pagesEl.appendChild(pageWrap);

          await page.render({
            canvas,
            canvasContext: ctx,
            viewport,
            transform: dpr === 1 ? undefined : [dpr, 0, 0, dpr, 0, 0],
          }).promise;
        }
      } catch (pdfErr) {
        if (!cancelled) {
          console.error("[MediaOverlay] Failed to render PDF:", pdfErr);
          setError("Failed to load PDF preview.");
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };

    // Allow overlay layout to settle before measuring width.
    const rAF = requestAnimationFrame(() => {
      renderPdf();
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(rAF);
      if (loadingTask) loadingTask.destroy();
      pagesEl.innerHTML = "";
    };
  }, [filePath, isOpen]);

  return (
    <div className={styles.pdfViewer} ref={containerRef}>
      {isLoading && <div className={styles.loadingText}>Loading PDF...</div>}
      {error && <div className={styles.errorText}>{error}</div>}
      {!isLoading && !error && pageCount > 0 && (
        <div className={styles.metaText}>{pageCount} pages</div>
      )}
      <div ref={pagesRef} className={styles.pdfPages} />
    </div>
  );
};
