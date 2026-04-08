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
  useEffect,
  useState,
} from "react";
import { Terminal } from "lucide-react";
import { useCodeHighlighter, useTextContextMenu, useKeyDown } from "@/hooks";
import { TextContextMenu } from "@/layout";
import styles from "./CodeBlock.shared.module.css";

export interface CodeBlockEditorProps {
  language: string;
  value: string;
  onChange?: (value: string) => void;
  onLanguageChange?: (language: string) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  placeholder?: string;
  actionLabel?: string;
  actionTitle?: string;
  actionIcon?: React.ReactNode;
  onAction?: () => void;
  actionDisabled?: boolean;
  fillHeight?: boolean;
  style?: React.CSSProperties;
}

export const CodeBlockEditor = forwardRef<
  HTMLTextAreaElement,
  CodeBlockEditorProps
>(
  (
    {
      language,
      value,
      onChange,
      onLanguageChange,
      onKeyDown,
      placeholder,
      actionLabel,
      actionTitle,
      actionIcon,
      onAction,
      actionDisabled = false,
      fillHeight = false,
      style,
    },
    ref,
  ) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const languageInputRef = useRef<HTMLInputElement>(null);
  const isCancellingRenameRef = useRef(false);
  const {
    data: contextMenuData,
    handleContextMenu,
    handleClose: handleCloseContextMenu,
  } = useTextContextMenu();
  const displayLanguage = language?.trim() || "text";
  const [isRenamingLanguage, setIsRenamingLanguage] = useState(false);
  const [renameValue, setRenameValue] = useState(displayLanguage);

  // Expose the textarea ref to parent
  useImperativeHandle(ref, () => textareaRef.current as HTMLTextAreaElement);

  // Only highlight if a valid language is provided (not "text")
  const shouldHighlight = Boolean(
    displayLanguage && displayLanguage.toLowerCase() !== "text",
  );
  const { highlightedHtml } = useCodeHighlighter(
    shouldHighlight ? value : "",
    displayLanguage,
  );

  const handleBeginRename = useCallback(() => {
    if (!onLanguageChange || isRenamingLanguage) return;
    isCancellingRenameRef.current = false;
    setIsRenamingLanguage(true);
  }, [isRenamingLanguage, onLanguageChange]);

  useEffect(() => {
    if (isRenamingLanguage && languageInputRef.current) {
      const input = languageInputRef.current;
      const caretPosition = input.value.length;
      input.focus();
      input.setSelectionRange(caretPosition, caretPosition);
    }
  }, [isRenamingLanguage]);

  useEffect(() => {
    if (!isRenamingLanguage) {
      setRenameValue(displayLanguage);
    }
  }, [displayLanguage, isRenamingLanguage]);

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

  const handleRenameSubmit = useCallback(() => {
    isCancellingRenameRef.current = false;
    const nextLanguage = renameValue.trim() || "text";
    if (nextLanguage !== displayLanguage) {
      onLanguageChange?.(nextLanguage);
    }
    setRenameValue(nextLanguage);
    setIsRenamingLanguage(false);
  }, [displayLanguage, onLanguageChange, renameValue]);

  const handleRenameCancel = useCallback(() => {
    isCancellingRenameRef.current = true;
    setRenameValue(displayLanguage);
    setIsRenamingLanguage(false);
  }, [displayLanguage]);

  const handleRenameBlur = useCallback(() => {
    if (isCancellingRenameRef.current) {
      isCancellingRenameRef.current = false;
      setRenameValue(displayLanguage);
      setIsRenamingLanguage(false);
      return;
    }

    handleRenameSubmit();
  }, [displayLanguage, handleRenameSubmit]);

  const handleRenameKeyDown = useKeyDown(
    {
      Enter: handleRenameSubmit,
      Escape: handleRenameCancel,
    },
    { stopPropagation: true },
  );

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
      className={`${styles.wrapper} ${fillHeight ? styles.fillHeight : ""}`}
      role="region"
      aria-label="Editor code block"
    >
      <div
        className={`${styles.header} ${
          actionLabel && onAction ? styles.headerWithAction : ""
        }`}
      >
        <div
          className={`${styles.langLabel} ${
            onLanguageChange ? styles.headerEditableZone : ""
          }`}
          onClick={
            onLanguageChange
              ? (e) => {
                  e.stopPropagation();
                  handleBeginRename();
                }
              : undefined
          }
        >
          <Terminal size={14} />
          {isRenamingLanguage ? (
            <input
              ref={languageInputRef}
              className={styles.langInput}
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={handleRenameBlur}
              onKeyDown={handleRenameKeyDown}
              onClick={(e) => e.stopPropagation()}
              spellCheck={false}
              aria-label="Code language"
            />
          ) : (
            <span
              className={`${styles.langName} ${
                onLanguageChange ? styles.renameableLangName : ""
              }`}
            >
              {displayLanguage}
            </span>
          )}
        </div>
      </div>
      {actionLabel && onAction && (
        <button
          type="button"
          onClick={onAction}
          className={`${styles.copyButton} ${styles.stickyButton}`}
          title={actionTitle || actionLabel}
          aria-label={actionTitle || actionLabel}
          disabled={actionDisabled}
        >
          {actionIcon && (
            <span className={styles.iconWrapper}>{actionIcon}</span>
          )}
          <span>{actionLabel}</span>
        </button>
      )}
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
},
);

CodeBlockEditor.displayName = "CodeBlockEditor";
