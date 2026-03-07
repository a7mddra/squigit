/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import styles from "./CapturePreview.module.css";

const CursorSvg = () => (
  <svg
    viewBox="0 0 255 362"
    fill="none"
    className={styles.cursorSvg}
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M0 305.971V0L221.195 221.984H91.7908L83.9476 224.353L0 305.971Z"
      fill="var(--c-raw-036)"
    />
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M173.363 318.853L104.567 348.18L15.219 136.322L85.5601 106.651L173.363 318.853Z"
      fill="var(--c-raw-036)"
    />
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M147.915 305.849L112.725 320.636L53.5669 179.754L88.6993 164.947L147.915 305.849Z"
      fill="var(--c-raw-018)"
    />
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M19.0833 45.9883V259.738L75.7417 204.982L83.9094 202.327H174.899L19.0833 45.9883Z"
      fill="var(--c-raw-018)"
    />
  </svg>
);

interface CapturePreviewProps {
  type: "rectangular" | "squiggle";
}

export const CapturePreview: React.FC<CapturePreviewProps> = ({ type }) => {
  return (
    <div className={styles.container}>
      <div className={styles.wrapper}>
        {type === "rectangular" ? (
          <>
            <div className={styles.selectionBox} />

            <div className={styles.cursorRectWrapper}>
              <div className={styles.cursorRectInner}>
                <CursorSvg />
              </div>
            </div>
          </>
        ) : (
          <>
            <svg viewBox="0 0 180 180" className={styles.squiggleSvg}>
              <path
                d="M 133,33 C 100,25 67,30 50,50 C 25,75 33,117 58,142 C 83,158 133,150 150,125 C 167,100 158,50 133,33 Z"
                className={styles.squigglePath}
              />
            </svg>
            <div className={styles.cursorSquiggle}>
              <CursorSvg />
            </div>
          </>
        )}
      </div>
    </div>
  );
};
