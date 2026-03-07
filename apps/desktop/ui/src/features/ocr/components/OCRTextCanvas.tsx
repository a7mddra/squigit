/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import styles from "./OCRTextCanvas.module.css";

interface OCRBox {
  text: string;
  box: number[][];
}

interface OCRTextCanvasProps {
  data: OCRBox[];
  size: { w: number; h: number };
  svgRef: React.RefObject<SVGSVGElement | null>;
  onTextMouseDown: (e: React.MouseEvent, boxIndex: number) => void;
}

export const OCRTextCanvas: React.FC<OCRTextCanvasProps> = ({
  data,
  size,
  svgRef,
  onTextMouseDown,
}) => {
  if (data.length === 0 || size.w === 0) return null;

  return (
    <svg
      ref={svgRef}
      className={styles.textLayer}
      data-text-layer
      viewBox={`0 0 ${size.w} ${size.h}`}
      preserveAspectRatio="xMidYMid meet"
    >
      <defs>
        <mask id="textCutoutMask">
          <rect x="0" y="0" width={size.w} height={size.h} fill="var(--c-raw-036)" />
          {data.map((item, i) => {
            const points = item.box.map((p) => `${p[0]},${p[1]}`).join(" ");
            return <polygon key={`mask-${i}`} points={points} fill="var(--c-raw-042)" />;
          })}
        </mask>
      </defs>

      <rect
        x="0"
        y="0"
        width={size.w}
        height={size.h}
        className={styles.dimOverlay}
        mask="url(#textCutoutMask)"
      />

      {data.map((item, i) => {
        const b = item.box;
        const points = b.map((p) => `${p[0]},${p[1]}`).join(" ");
        const h = Math.abs(b[3][1] - b[0][1]);
        const w = Math.abs(b[1][0] - b[0][0]);

        return (
          <g key={i}>
            <polygon className={styles.highlightBg} points={points} />
            <text
              id={`text-${i}`}
              x={b[0][0]}
              y={b[0][1] + h * 0.78}
              fontSize={h * 0.85}
              fontFamily="'Arial Narrow', Arial, sans-serif"
              textLength={w}
              lengthAdjust="spacingAndGlyphs"
              className={styles.selectableText}
              data-selectable-text
              onMouseDown={(e) => onTextMouseDown(e, i)}
            >
              {item.text}
            </text>
          </g>
        );
      })}
    </svg>
  );
};
