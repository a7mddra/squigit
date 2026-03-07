/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef, useState, useEffect, useLayoutEffect, RefObject } from "react";
import { Message } from "../chat.types";

export function useChatScroll({
  messages,
  chatId,
  isNavigating,
  inputHeight,
  scrollContainerRef,
  wasAtBottomRef,
  suspendAutoScroll = false,
}: {
  messages: Message[];
  chatId: string | null;
  isNavigating: boolean;
  inputHeight: number;
  scrollContainerRef: RefObject<HTMLDivElement | null>;
  wasAtBottomRef: React.MutableRefObject<boolean>;
  suspendAutoScroll?: boolean;
}) {
  const [showSpinner, setShowSpinner] = useState(false);
  const navigationStartTimeRef = useRef<number>(0);
  const prevChatIdRef = useRef<string | null>(null);
  const prevMessageCountRef = useRef(messages.length);
  const MIN_SPINNER_DURATION = 400;
  const previousInputHeightRef = useRef(0);

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

  useLayoutEffect(() => {
    const el = scrollContainerRef.current;
    if (!el || isSpinnerVisible) return;

    let rafId: number | null = null;
    const timeoutIds: number[] = [];
    const chatChanged = prevChatIdRef.current !== chatId;
    const messageCountChanged = messages.length !== prevMessageCountRef.current;

    const scrollToBottom = () => {
      el.scrollTo({ top: el.scrollHeight, behavior: "instant" });
    };

    if (!suspendAutoScroll) {
      if (chatChanged) {
        scrollToBottom();
        rafId = requestAnimationFrame(scrollToBottom);
        timeoutIds.push(window.setTimeout(scrollToBottom, 50));
        timeoutIds.push(window.setTimeout(scrollToBottom, 150));
        prevChatIdRef.current = chatId;
      } else if (messageCountChanged) {
        if (prevMessageCountRef.current === 0 && messages.length > 0) {
          const isStreamCompletion =
            messages.length === 1 && messages[0]?.role === "model";
          if (!isStreamCompletion) {
            scrollToBottom();
          }
        } else {
          const lastMessage = messages[messages.length - 1];
          if (lastMessage?.role === "user") {
            el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
          }
        }
      }
    }

    if (suspendAutoScroll) {
      prevChatIdRef.current = chatId;
    }

    prevMessageCountRef.current = messages.length;

    return () => {
      if (rafId !== null) {
        window.cancelAnimationFrame(rafId);
      }
      for (const timeoutId of timeoutIds) {
        window.clearTimeout(timeoutId);
      }
    };
  }, [
    chatId,
    isSpinnerVisible,
    messages,
    scrollContainerRef,
    suspendAutoScroll,
  ]);

  useLayoutEffect(() => {
    const scrollEl = scrollContainerRef.current;
    if (!scrollEl) return;
    if (isSpinnerVisible) return;

    const isGrowing = inputHeight > previousInputHeightRef.current;

    if (!isGrowing && wasAtBottomRef.current) {
      scrollEl.scrollTop = scrollEl.scrollHeight;
    }
    previousInputHeightRef.current = inputHeight;
  }, [inputHeight, isSpinnerVisible, scrollContainerRef, wasAtBottomRef]);

  return { isSpinnerVisible };
}
