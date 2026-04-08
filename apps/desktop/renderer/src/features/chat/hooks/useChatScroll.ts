/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  useRef,
  useState,
  useEffect,
  useLayoutEffect,
  useCallback,
  RefObject,
} from "react";
import { Message } from "../chat.types";

const BOTTOM_THRESHOLD_PX = 48;
const BOTTOM_LOCK_MS_CHAT_CHANGE = 2200;
const BOTTOM_LOCK_MS_MESSAGE_APPEND = 1000;
const MAX_BOTTOM_LOCK_FRAMES = 180;

function isNearBottom(el: HTMLDivElement): boolean {
  const distanceFromBottom =
    el.scrollHeight - el.scrollTop - el.clientHeight;
  return distanceFromBottom < BOTTOM_THRESHOLD_PX;
}

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
  const bottomLockUntilRef = useRef(0);
  const bottomLockRafRef = useRef<number | null>(null);
  const bottomLockFrameCountRef = useRef(0);

  useEffect(() => {
    const el = scrollContainerRef.current;
    if (!el) return;

    const updateBottomState = () => {
      wasAtBottomRef.current = isNearBottom(el);
    };

    updateBottomState();
    el.addEventListener("scroll", updateBottomState, { passive: true });
    return () => {
      el.removeEventListener("scroll", updateBottomState);
    };
  }, [scrollContainerRef, wasAtBottomRef]);

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

  const jumpToBottom = useCallback((el: HTMLDivElement) => {
    const maxScrollTop = Math.max(0, el.scrollHeight - el.clientHeight);
    if (Math.abs(el.scrollTop - maxScrollTop) > 1) {
      el.scrollTop = maxScrollTop;
    }
  }, []);

  const isBottomLockActive = useCallback(
    () => Date.now() < bottomLockUntilRef.current,
    [],
  );

  const runBottomLockTick = useCallback(() => {
    bottomLockRafRef.current = null;

    const el = scrollContainerRef.current;
    if (!el || isSpinnerVisible || suspendAutoScroll || !isBottomLockActive()) {
      return;
    }

    jumpToBottom(el);

    bottomLockFrameCountRef.current += 1;
    if (bottomLockFrameCountRef.current < MAX_BOTTOM_LOCK_FRAMES) {
      bottomLockRafRef.current = window.requestAnimationFrame(runBottomLockTick);
    }
  }, [
    isBottomLockActive,
    isSpinnerVisible,
    jumpToBottom,
    scrollContainerRef,
    suspendAutoScroll,
  ]);

  const startBottomLock = useCallback(
    (durationMs: number) => {
      if (durationMs > 0) {
        bottomLockUntilRef.current = Math.max(
          bottomLockUntilRef.current,
          Date.now() + durationMs,
        );
      }
      bottomLockFrameCountRef.current = 0;

      if (bottomLockRafRef.current === null) {
        bottomLockRafRef.current =
          window.requestAnimationFrame(runBottomLockTick);
      }
    },
    [runBottomLockTick],
  );

  useEffect(() => {
    return () => {
      if (bottomLockRafRef.current !== null) {
        window.cancelAnimationFrame(bottomLockRafRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!isSpinnerVisible && isBottomLockActive()) {
      startBottomLock(0);
    }
  }, [isBottomLockActive, isSpinnerVisible, startBottomLock]);

  useLayoutEffect(() => {
    const el = scrollContainerRef.current;
    if (!el || isSpinnerVisible) return;

    const chatChanged = prevChatIdRef.current !== chatId;
    const messageCountChanged = messages.length !== prevMessageCountRef.current;

    if (!suspendAutoScroll) {
      if (chatChanged) {
        jumpToBottom(el);
        startBottomLock(BOTTOM_LOCK_MS_CHAT_CHANGE);
        prevChatIdRef.current = chatId;
      } else if (messageCountChanged) {
        if (prevMessageCountRef.current === 0 && messages.length > 0) {
          const isStreamCompletion =
            messages.length === 1 && messages[0]?.role === "model";
          if (!isStreamCompletion) {
            jumpToBottom(el);
            startBottomLock(BOTTOM_LOCK_MS_MESSAGE_APPEND);
          }
        } else {
          const lastMessage = messages[messages.length - 1];
          if (lastMessage?.role === "user") {
            const maxScrollTop = Math.max(0, el.scrollHeight - el.clientHeight);
            el.scrollTo({ top: maxScrollTop, behavior: "smooth" });
            startBottomLock(BOTTOM_LOCK_MS_MESSAGE_APPEND);
          }
        }
      }
    }

    if (suspendAutoScroll) {
      prevChatIdRef.current = chatId;
    }

    prevMessageCountRef.current = messages.length;
  }, [
    chatId,
    isSpinnerVisible,
    jumpToBottom,
    messages,
    scrollContainerRef,
    startBottomLock,
    suspendAutoScroll,
  ]);

  useLayoutEffect(() => {
    const scrollEl = scrollContainerRef.current;
    if (!scrollEl) return;
    if (isSpinnerVisible) return;

    const isGrowing = inputHeight > previousInputHeightRef.current;

    if (
      !isGrowing &&
      (wasAtBottomRef.current || isBottomLockActive()) &&
      !suspendAutoScroll
    ) {
      jumpToBottom(scrollEl);
    }
    previousInputHeightRef.current = inputHeight;
  }, [
    inputHeight,
    isBottomLockActive,
    isSpinnerVisible,
    jumpToBottom,
    scrollContainerRef,
    suspendAutoScroll,
    wasAtBottomRef,
  ]);

  useLayoutEffect(() => {
    const scrollEl = scrollContainerRef.current;
    const contentEl = scrollEl?.firstElementChild;
    if (!scrollEl || !contentEl || isSpinnerVisible) return;

    const observer = new ResizeObserver(() => {
      if (suspendAutoScroll) return;
      if (!wasAtBottomRef.current && !isBottomLockActive()) return;
      jumpToBottom(scrollEl);
    });

    observer.observe(contentEl);
    return () => observer.disconnect();
  }, [
    isBottomLockActive,
    isSpinnerVisible,
    jumpToBottom,
    scrollContainerRef,
    suspendAutoScroll,
    wasAtBottomRef,
  ]);

  return { isSpinnerVisible };
}
