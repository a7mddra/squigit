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
      const nextHeight = Math.max(0, Math.round(rawHeight));
      if (nextHeight === lastHeightRef.current) {
        return;
      }

      lastHeightRef.current = nextHeight;
      setInputHeight(nextHeight);
    };

    const measureAndCommit = () => {
      commitHeight(el.getBoundingClientRect().height);
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
