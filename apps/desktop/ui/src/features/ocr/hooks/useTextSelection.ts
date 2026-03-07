/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef, useCallback } from "react";

interface OCRBox {
  text: string;
  box: number[][];
}

interface UseTextSelectionParams {
  data: OCRBox[];
  onSelectionComplete?: (selection: Selection) => void;
}

export const useTextSelection = ({
  data,
  onSelectionComplete,
}: UseTextSelectionParams) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const isCustomSelectingRef = useRef(false);
  const selectionModeRef = useRef<"char" | "word" | "line">("char");
  const selectionAnchorRef = useRef<{
    boxIndex: number;
    charIndex: number;
  } | null>(null);

  const getMouseInSvg = useCallback((e: MouseEvent | React.MouseEvent) => {
    const svg = svgRef.current;
    if (!svg) return null;
    const pt = svg.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    return pt.matrixTransform(svg.getScreenCTM()?.inverse());
  }, []);

  const getClosestTextIndex = useCallback(
    (
      point: DOMPoint,
      boxIndex: number,
    ): { boxIndex: number; charIndex: number } | null => {
      const item = data[boxIndex];
      if (!item) return null;

      const b = item.box;
      const x0 = b[0][0];
      const y0 = b[0][1];
      const x1 = b[1][0];
      const y1 = b[1][1];

      const dx = x1 - x0;
      const dy = y1 - y0;
      const lenSq = dx * dx + dy * dy;

      let t = ((point.x - x0) * dx + (point.y - y0) * dy) / lenSq;
      t = Math.max(0, Math.min(1, t));

      const totalChars = item.text.length;
      let charIndex = Math.floor(t * totalChars);
      if (charIndex < 0) charIndex = 0;
      if (charIndex > totalChars) charIndex = totalChars;

      return { boxIndex, charIndex };
    },
    [data],
  );

  const getWordRange = useCallback((text: string, index: number) => {
    if (!/[a-zA-Z0-9]/.test(text[index])) {
      return { start: index, end: index + 1 };
    }

    let start = index;
    while (start > 0 && /[a-zA-Z0-9]/.test(text[start - 1])) {
      start--;
    }
    let end = index;
    while (end < text.length && /[a-zA-Z0-9]/.test(text[end])) {
      end++;
    }
    return { start, end };
  }, []);

  const updateNativeSelection = useCallback(
    (
      anchor: { boxIndex: number; charIndex: number },
      focus: { boxIndex: number; charIndex: number },
    ) => {
      const selection = window.getSelection();
      if (!selection) return;

      const getTextNode = (bIdx: number) => {
        const textEl = document.getElementById(`text-${bIdx}`);
        return textEl?.firstChild || null;
      };

      const anchorNode = getTextNode(anchor.boxIndex);
      const focusNode = getTextNode(focus.boxIndex);

      if (anchorNode && focusNode) {
        try {
          if (
            selection.anchorNode !== anchorNode ||
            selection.focusNode !== focusNode ||
            selection.anchorOffset !== anchor.charIndex ||
            selection.focusOffset !== focus.charIndex
          ) {
            selection.setBaseAndExtent(
              anchorNode,
              anchor.charIndex,
              focusNode,
              focus.charIndex,
            );
          }
        } catch {}
      }
    },
    [],
  );

  const handleSelectionMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isCustomSelectingRef.current || !selectionAnchorRef.current) return;

      const pt = getMouseInSvg(e);
      if (!pt) return;

      let closestBoxIndex = -1;
      let minDist = Infinity;

      data.forEach((item, idx) => {
        const cx = (item.box[0][0] + item.box[1][0]) / 2;
        const cy = (item.box[0][1] + item.box[3][1]) / 2;
        const dist = Math.sqrt(Math.pow(pt.x - cx, 2) + Math.pow(pt.y - cy, 2));
        if (dist < minDist) {
          minDist = dist;
          closestBoxIndex = idx;
        }
      });

      if (closestBoxIndex === -1) return;

      let focus = getClosestTextIndex(pt, closestBoxIndex);
      if (!focus) return;

      if (selectionModeRef.current === "word") {
        const text = data[closestBoxIndex].text;
        const { start, end } = getWordRange(text, focus.charIndex);
        if (closestBoxIndex === selectionAnchorRef.current.boxIndex) {
          if (focus.charIndex < selectionAnchorRef.current.charIndex) {
            focus.charIndex = start;
          } else {
            focus.charIndex = end;
          }
        } else {
          focus.charIndex = end;
        }
      } else if (selectionModeRef.current === "line") {
        const text = data[closestBoxIndex].text;
        if (closestBoxIndex > selectionAnchorRef.current.boxIndex) {
          focus.charIndex = text.length;
        } else if (closestBoxIndex < selectionAnchorRef.current.boxIndex) {
          focus.charIndex = 0;
        } else {
          if (focus.charIndex < selectionAnchorRef.current.charIndex)
            focus.charIndex = 0;
          else focus.charIndex = text.length;
        }
      }

      updateNativeSelection(selectionAnchorRef.current, focus);
    },
    [
      data,
      getMouseInSvg,
      getClosestTextIndex,
      getWordRange,
      updateNativeSelection,
    ],
  );

  const handleGlobalMouseUp = useCallback(() => {
    isCustomSelectingRef.current = false;
    document.removeEventListener("mousemove", handleSelectionMouseMove);
    document.removeEventListener("mouseup", handleGlobalMouseUp);

    const sel = window.getSelection();
    if (
      sel &&
      !sel.isCollapsed &&
      sel.toString().trim() &&
      onSelectionComplete
    ) {
      onSelectionComplete(sel);
    }
  }, [handleSelectionMouseMove, onSelectionComplete]);

  const handleTextMouseDown = useCallback(
    (e: React.MouseEvent, boxIndex: number) => {
      if (e.button !== 0) return;
      e.preventDefault();

      const pt = getMouseInSvg(e);
      if (!pt) return;

      const target = getClosestTextIndex(pt, boxIndex);
      if (!target) return;

      isCustomSelectingRef.current = true;

      const clicks = e.detail;
      if (clicks >= 2) {
        selectionModeRef.current = clicks % 2 === 0 ? "word" : "line";
      } else {
        selectionModeRef.current = "char";
      }

      if (selectionModeRef.current === "word") {
        const text = data[boxIndex].text;
        const { start, end } = getWordRange(text, target.charIndex);
        const anchor = { boxIndex, charIndex: start };
        const focus = { boxIndex, charIndex: end };
        selectionAnchorRef.current = anchor;
        updateNativeSelection(anchor, focus);
      } else if (selectionModeRef.current === "line") {
        const text = data[boxIndex].text;
        const anchor = { boxIndex, charIndex: 0 };
        const focus = { boxIndex, charIndex: text.length };
        selectionAnchorRef.current = anchor;
        updateNativeSelection(anchor, focus);
      } else {
        selectionAnchorRef.current = target;
        window.getSelection()?.removeAllRanges();
      }

      document.addEventListener("mousemove", handleSelectionMouseMove);
      document.addEventListener("mouseup", handleGlobalMouseUp);
    },
    [
      data,
      getMouseInSvg,
      getClosestTextIndex,
      getWordRange,
      updateNativeSelection,
      handleSelectionMouseMove,
      handleGlobalMouseUp,
    ],
  );

  return {
    svgRef,
    handleTextMouseDown,
  };
};
