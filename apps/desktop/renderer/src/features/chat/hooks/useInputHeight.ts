/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useRef, useEffect } from "react";

export function useInputHeight() {
  const inputContainerRef = useRef<HTMLDivElement>(null);
  const [inputHeight, setInputHeight] = useState(0);
  const lastHeightRef = useRef(0);

  useEffect(() => {
    const el = inputContainerRef.current;
    if (!el) return;

    const commitHeight = (rawHeight: number) => {
      const viewportHeight = window.innerHeight || rawHeight;
      const cappedHeight = Math.min(rawHeight, viewportHeight * 0.75);
      const nextHeight = Math.max(0, Math.round(cappedHeight));
      if (nextHeight === lastHeightRef.current) {
        return;
      }

      lastHeightRef.current = nextHeight;
      setInputHeight(nextHeight);
    };

    const measureAndCommit = () => {
      const measured = Math.max(
        el.offsetHeight,
        el.getBoundingClientRect().height,
      );
      commitHeight(measured);
    };

    measureAndCommit();

    const observer = new ResizeObserver(() => {
      measureAndCommit();
    });

    observer.observe(el);

    return () => {
      observer.disconnect();
    };
  }, []);

  return { inputContainerRef, inputHeight };
}
