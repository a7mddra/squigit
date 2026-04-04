/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import styles from "./shared.module.css";

const EDGE_PADDING = 8;

interface CitationTipProps {
  parentRef: React.RefObject<HTMLElement | null>;
  show: boolean;
  headerUrl: string;
  headerIconUrl?: string;
  body: string;
  offset?: number;
  above?: boolean;
}

export const CitationTip: React.FC<CitationTipProps> = ({
  parentRef,
  show,
  headerUrl,
  headerIconUrl,
  body,
  offset = 10,
  above = true,
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
      const tipRect = ref.current.getBoundingClientRect();

      let left = parentRect.left + parentRect.width / 2 - tipRect.width / 2;
      left = Math.max(EDGE_PADDING, left);
      left = Math.min(left, window.innerWidth - tipRect.width - EDGE_PADDING);

      let top = above
        ? parentRect.top - tipRect.height - offset
        : parentRect.bottom + offset;

      if (top < EDGE_PADDING) {
        top = EDGE_PADDING;
      }
      if (top + tipRect.height > window.innerHeight - EDGE_PADDING) {
        top = Math.max(
          EDGE_PADDING,
          window.innerHeight - tipRect.height - EDGE_PADDING,
        );
      }

      setStyle({
        position: "fixed",
        top,
        left,
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
  }, [show, parentRef, headerUrl, body, offset, above]);

  if (!show) return null;

  return createPortal(
    <div ref={ref} className={styles.citationTip} style={style}>
      <div className={styles.citationHeader}>
        {headerIconUrl && (
          <img
            src={headerIconUrl}
            alt=""
            className={styles.citationHeaderIcon}
            onError={(e) => {
              e.currentTarget.style.display = "none";
            }}
          />
        )}
        <span className={styles.citationHeaderUrl}>{headerUrl}</span>
      </div>
      <div className={styles.citationBody}>{body}</div>
      {above && <div className={styles.arrow} />}
    </div>,
    document.body,
  );
};
