/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef, useState, useEffect } from "react";
import { createPortal } from "react-dom";
import styles from "./Tooltip.module.css";

const EDGE_PADDING = 8;

interface TooltipProps {
  text: string;
  parentRef: React.RefObject<HTMLElement | null>;
  show: boolean;
  offset?: number;
  above?: boolean;
}

export const Tooltip: React.FC<TooltipProps> = ({
  text,
  parentRef,
  show,
  offset = 8,
  above = false,
}) => {
  const [style, setStyle] = useState<React.CSSProperties>({
    opacity: 0,
    visibility: "hidden",
  });
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!show || !parentRef.current) {
      setStyle({ opacity: 0, visibility: "hidden" });
      return;
    }

    const update = () => {
      if (!parentRef.current || !ref.current) return;
      const parentRect = parentRef.current.getBoundingClientRect();
      const tooltipRect = ref.current.getBoundingClientRect();

      let left: number;
      let top: number;

      if (above) {
        left = parentRect.left + parentRect.width / 2 - tooltipRect.width / 2;
        top = parentRect.top - tooltipRect.height - offset;
      } else {
        const windowWidth = window.innerWidth;
        left = parentRect.right + offset;
        top = parentRect.top + parentRect.height / 2 - tooltipRect.height / 2;

        if (left + tooltipRect.width > windowWidth - EDGE_PADDING) {
          const leftCandidate = parentRect.left - offset - tooltipRect.width;
          if (leftCandidate > EDGE_PADDING) {
            left = leftCandidate;
          } else {
            left = parentRect.left - offset - tooltipRect.width;
          }
        }
      }

      setStyle({
        position: "fixed",
        top: top,
        left: left,
        opacity: 1,
        visibility: "visible",
        zIndex: 9999,
      });
    };

    update();
    window.addEventListener("scroll", update, true);
    window.addEventListener("resize", update);
    return () => {
      window.removeEventListener("scroll", update, true);
      window.removeEventListener("resize", update);
    };
  }, [show, text, offset, above]);

  if (!show)
    return createPortal(
      <div ref={ref} className={styles.tooltip} style={{ opacity: 0 }}>
        {text}
        {above && <div className={styles.arrow} />}
      </div>,
      document.body,
    );

  return createPortal(
    <div ref={ref} className={styles.tooltip} style={style}>
      {text}
      {above && <div className={styles.arrow} />}
    </div>,
    document.body,
  );
};
