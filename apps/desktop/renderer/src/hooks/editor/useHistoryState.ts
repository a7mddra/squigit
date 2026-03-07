/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef, useEffect, useCallback } from "react";

interface UseHistoryStateProps<T> {
  value: T;
  onChange?: (value: T) => void;
  maxHistory?: number;
}

export function useHistoryState<T>({
  value,
  onChange,
  maxHistory = 100,
}: UseHistoryStateProps<T>) {
  const historyRef = useRef<T[]>([value]);
  const historyIndexRef = useRef<number>(0);
  const isUndoRedoRef = useRef<boolean>(false);

  useEffect(() => {
    if (isUndoRedoRef.current) {
      isUndoRedoRef.current = false;
      return;
    }

    const currentHistory = historyRef.current;
    const currentIndex = historyIndexRef.current;

    if (value !== currentHistory[currentIndex]) {
      const newHistory = currentHistory.slice(0, currentIndex + 1);
      newHistory.push(value);

      if (newHistory.length > maxHistory) {
        newHistory.shift();
      }

      historyRef.current = newHistory;
      historyIndexRef.current = newHistory.length - 1;
    }
  }, [value, maxHistory]);

  const undo = useCallback(() => {
    if (historyIndexRef.current > 0) {
      historyIndexRef.current -= 1;
      isUndoRedoRef.current = true;
      const prevValue = historyRef.current[historyIndexRef.current];
      onChange?.(prevValue);
      return prevValue;
    }
    return value;
  }, [onChange, value]);

  const redo = useCallback(() => {
    if (historyIndexRef.current < historyRef.current.length - 1) {
      historyIndexRef.current += 1;
      isUndoRedoRef.current = true;
      const nextValue = historyRef.current[historyIndexRef.current];
      onChange?.(nextValue);
      return nextValue;
    }
    return value;
  }, [onChange, value]);

  const canUndo = historyIndexRef.current > 0;
  const canRedo = historyIndexRef.current < historyRef.current.length - 1;

  const push = useCallback(
    (newValue: T) => {
      const currentHistory = historyRef.current;
      const currentIndex = historyIndexRef.current;

      if (newValue !== currentHistory[currentIndex]) {
        const newHistory = currentHistory.slice(0, currentIndex + 1);
        newHistory.push(newValue);
        if (newHistory.length > maxHistory) {
          newHistory.shift();
        }
        historyRef.current = newHistory;
        historyIndexRef.current = newHistory.length - 1;
      }
    },
    [maxHistory],
  );

  return {
    undo,
    redo,
    push,
    canUndo,
    canRedo,
  };
}
