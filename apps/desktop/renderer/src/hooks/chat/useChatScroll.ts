/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef, useState, useEffect, RefObject } from "react";

export function useChatScroll({
  isNavigating,
  scrollContainerRef,
  bottomAnchorRef,
  wasAtBottomRef,
}: {
  isNavigating: boolean;
  scrollContainerRef: RefObject<HTMLDivElement | null>;
  bottomAnchorRef: RefObject<HTMLDivElement | null>;
  wasAtBottomRef: React.MutableRefObject<boolean>;
}) {
  const [showSpinner, setShowSpinner] = useState(false);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const navigationStartTimeRef = useRef<number>(0);
  const MIN_SPINNER_DURATION = 400;

  useEffect(() => {
    const container = scrollContainerRef.current;
    const anchor = bottomAnchorRef.current;
    if (!container || !anchor) {
      wasAtBottomRef.current = true;
      setIsAtBottom(true);
      return;
    }

    const observer = new IntersectionObserver(
      ([entry]) => {
        const nextIsAtBottom = entry?.isIntersecting ?? false;
        wasAtBottomRef.current = nextIsAtBottom;
        setIsAtBottom(nextIsAtBottom);
      },
      {
        root: container,
        threshold: 0.01,
      },
    );

    observer.observe(anchor);
    return () => observer.disconnect();
  }, [bottomAnchorRef, scrollContainerRef, wasAtBottomRef]);

  useEffect(() => {
    if (isNavigating) {
      setShowSpinner(true);
      navigationStartTimeRef.current = Date.now();
    } else {
      const elapsed = Date.now() - navigationStartTimeRef.current;
      const remaining = Math.max(0, MIN_SPINNER_DURATION - elapsed);
      const t = setTimeout(() => {
        setShowSpinner(false);
      }, remaining);
      return () => clearTimeout(t);
    }
  }, [isNavigating]);

  const isSpinnerVisible = isNavigating || showSpinner;

  return { isSpinnerVisible, isAtBottom };
}
