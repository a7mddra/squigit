/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useRef, useState, useEffect, useCallback } from "react";
import { useHistoryState } from "./useHistoryState";
import { useClipboard, useKeyDown } from "@/hooks";

interface UseTextEditorProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: () => void;
  preventNewLine?: boolean;
}

export function useTextEditor({
  value,
  onChange,
  onSubmit,
  preventNewLine = false,
}: UseTextEditorProps) {
  const ref = useRef<HTMLTextAreaElement | HTMLInputElement | null>(null);
  const [hasSelection, setHasSelection] = useState(false);

  const { undo, redo } = useHistoryState({
    value,
    onChange,
  });

  const { readText } = useClipboard();

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const handleSelectionChange = () => {
      if (document.activeElement === el) {
        const start = el.selectionStart;
        const end = el.selectionEnd;
        setHasSelection(start !== end);
      }
    };

    document.addEventListener("selectionchange", handleSelectionChange);
    return () => {
      document.removeEventListener("selectionchange", handleSelectionChange);
    };
  }, []);

  const handleSelectAll = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    el.select();
    setHasSelection(true);
  }, []);

  const handleCopy = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const selectedText = el.value.substring(
      el.selectionStart || 0,
      el.selectionEnd || 0,
    );
    if (selectedText) {
      navigator.clipboard.writeText(selectedText);
    }
  }, []);

  const handleCut = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const start = el.selectionStart || 0;
    const end = el.selectionEnd || 0;
    const selectedText = el.value.substring(start, end);

    if (selectedText) {
      navigator.clipboard.writeText(selectedText);
      const newValue = el.value.substring(0, start) + el.value.substring(end);
      onChange(newValue);

      setTimeout(() => {
        if (ref.current) {
          ref.current.setSelectionRange(start, start);
          ref.current.focus();
        }
      }, 0);
    }
  }, [onChange]);

  const handlePaste = useCallback(async () => {
    const el = ref.current;
    if (!el) return;

    el.focus();

    try {
      const text = await readText();
      if (!text) return;

      const start = el.selectionStart || 0;
      const end = el.selectionEnd || 0;
      const newValue =
        el.value.substring(0, start) + text + el.value.substring(end);

      onChange(newValue);
      const newCursorPos = start + text.length;

      setTimeout(() => {
        if (ref.current) {
          ref.current.setSelectionRange(newCursorPos, newCursorPos);
          ref.current.focus();
        }
      }, 0);
    } catch (err) {
      console.error("Failed to read clipboard:", err);
    }
  }, [onChange, readText]);

  const handleKeyDown = useKeyDown(
    {
      "Mod+z": (e) => {
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
      },
      "Mod+Shift+z": () => redo(),
      "Mod+y": () => redo(),
      "Mod+a": () => handleSelectAll(),
      Enter: (e) => {
        if (preventNewLine) {
          e.preventDefault();
          if (onSubmit) onSubmit();
        } else if (!e.shiftKey && onSubmit) {
          e.preventDefault();
          if (value.trim().length > 0) onSubmit();
        }
      },
    },
    { preventDefault: true },
  );

  return {
    ref,
    hasSelection,
    handleCopy,
    handleCut,
    handlePaste,
    handleSelectAll,
    handleKeyDown,
    undo,
    redo,
  };
}
