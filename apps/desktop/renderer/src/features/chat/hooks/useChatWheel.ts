/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useEffect, useState, RefObject } from "react";

export function useChatWheel(
  headerRef: RefObject<HTMLDivElement | null>,
  activeSessionId?: string | null,
) {
  const [isImageExpanded, setIsImageExpanded] = useState(false);

  useEffect(() => {
    setIsImageExpanded(false);
  }, [activeSessionId]);

  useEffect(() => {
    const header = headerRef.current;
    if (!header) return;

    const handleWheel = (e: WheelEvent) => {
      if (Math.abs(e.deltaY) < 5) return;

      if (e.deltaY < 0) {
        if (!isImageExpanded) {
          setIsImageExpanded(true);
          e.preventDefault();
        }
      } else {
        if (isImageExpanded) {
          setIsImageExpanded(false);
          e.preventDefault();
        }
      }
    };

    header.addEventListener("wheel", handleWheel, { passive: false });
    return () => {
      header.removeEventListener("wheel", handleWheel);
    };
  }, [isImageExpanded, headerRef]);

  return { isImageExpanded, setIsImageExpanded };
}
