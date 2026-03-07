/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  forwardRef,
  useRef,
  useCallback,
  useImperativeHandle,
  useLayoutEffect,
} from "react";
import { Terminal } from "lucide-react";
import { useCodeHighlighter, useTextContextMenu, useKeyDown } from "@/hooks";
import { TextContextMenu } from "@/layout";
import styles from "./CodeBlock.shared.module.css";

export interface CodeBlockEditorProps {
  language: string;
  value: string;
  onChange?: (value: string) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  placeholder?: string;
  style?: React.CSSProperties;
}

export const CodeBlockEditor = forwardRef<
  HTMLTextAreaElement,
  CodeBlockEditorProps
>(({ language, value, onChange, onKeyDown, placeholder, style }, ref) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const {
    data: contextMenuData,
    handleContextMenu,
    handleClose: handleCloseContextMenu,
  } = useTextContextMenu();

  // Expose the textarea ref to parent
  useImperativeHandle(ref, () => textareaRef.current as HTMLTextAreaElement);

  // Only highlight if a valid language is provided (not "text")
  const shouldHighlight = language && language !== "text";
  const { highlightedHtml } = useCodeHighlighter(
    shouldHighlight ? value : "",
    language,
  );

  // Sync scroll between textarea and highlight overlay
  const handleScroll = useCallback(() => {
    if (textareaRef.current && highlightRef.current) {
      highlightRef.current.scrollTop = textareaRef.current.scrollTop;
      highlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
    }
  }, []);

  // Force sync on render updates (e.g. when highlighting loads or content changes)
  useLayoutEffect(() => {
    handleScroll();
  });

  // Ensure trailing newline so cursor position matches
  const displayValue = value.endsWith("\n") ? value + " " : value;

  const handleCopy = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    if (textarea.selectionStart !== textarea.selectionEnd) {
      const selectedText = textarea.value.substring(
        textarea.selectionStart,
        textarea.selectionEnd,
      );
      navigator.clipboard.writeText(selectedText);
    }
  }, []);

  const handleCut = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea || !onChange) return;

    if (textarea.selectionStart !== textarea.selectionEnd) {
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const selectedText = textarea.value.substring(start, end);
      navigator.clipboard.writeText(selectedText);

      const newValue =
        textarea.value.substring(0, start) + textarea.value.substring(end);
      onChange(newValue);

      setTimeout(() => {
        textarea.selectionStart = textarea.selectionEnd = start;
      }, 0);
    }
  }, [onChange, value]);

  const handlePaste = useCallback(async () => {
    const textarea = textareaRef.current;
    if (!textarea || !onChange) return;

    try {
      const text = await navigator.clipboard.readText();
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const newValue = value.substring(0, start) + text + value.substring(end);
      onChange(newValue);

      setTimeout(() => {
        textarea.selectionStart = textarea.selectionEnd = start + text.length;
      }, 0);
    } catch (err) {
      console.error("Failed to read clipboard:", err);
    }
  }, [onChange, value]);

  const handleSelectAll = useCallback(() => {
    textareaRef.current?.select();
  }, []);

  const handleInternalKeyDown = useKeyDown(
    {
      Tab: (e) => {
        e.preventDefault();
        const textarea = textareaRef.current;
        if (!textarea || !onChange) return;

        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;

        // Insert 2 spaces
        const spaces = "  ";

        const newValue =
          value.substring(0, start) + spaces + value.substring(end);
        onChange(newValue);

        setTimeout(() => {
          textarea.selectionStart = textarea.selectionEnd =
            start + spaces.length;
        }, 0);
      },
    },
    { preventDefault: false },
  );

  return (
    <div
      className={styles.wrapper}
      role="region"
      aria-label="Editor code block"
    >
      <div className={styles.header}>
        <div className={styles.langLabel}>
          <Terminal size={14} />
          <span className={styles.langName}>{language || "text"}</span>
        </div>
      </div>
      <div className={styles.EditorContainer} style={style}>
        {/* Sizer to drive auto-height since other layers are absolute */}
        <div className={styles.sizer} aria-hidden="true">
          {displayValue || placeholder}
        </div>

        {/* Highlighted code layer (behind) */}
        {shouldHighlight && highlightedHtml ? (
          <div
            ref={highlightRef}
            className={`${styles.highlightLayer} shiki-dual-theme`}
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
            aria-hidden="true"
          />
        ) : (
          <pre
            ref={highlightRef as any}
            className={styles.highlightLayer}
            aria-hidden="true"
          >
            {displayValue || placeholder}
          </pre>
        )}
        {/* Transparent textarea (on top) */}
        <textarea
          ref={textareaRef}
          className={styles.textareaOverlay}
          value={value}
          onChange={(e) => onChange?.(e.target.value)}
          onKeyDown={(e) => {
            handleInternalKeyDown(e);
            onKeyDown?.(e);
          }}
          onScroll={handleScroll}
          placeholder={placeholder}
          spellCheck={false}
          aria-label="code editor"
          onContextMenu={handleContextMenu}
        />
        {contextMenuData.isOpen && (
          <TextContextMenu
            x={contextMenuData.x}
            y={contextMenuData.y}
            onClose={handleCloseContextMenu}
            onCopy={handleCopy}
            onCut={handleCut}
            onPaste={handlePaste}
            onSelectAll={handleSelectAll}
            hasSelection={
              textareaRef.current
                ? textareaRef.current.selectionStart !==
                  textareaRef.current.selectionEnd
                : false
            }
          />
        )}
      </div>
    </div>
  );
});

CodeBlockEditor.displayName = "CodeBlockEditor";
