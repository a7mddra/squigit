/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useLayoutEffect, useCallback, useState } from "react";
import styles from "./ChatInput.module.css";

const ExpandIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M4 10V4h6" />
    <path d="M20 14v6h-6" />
  </svg>
);

const CollapseIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M8 2v6H2" />
    <path d="M16 22v-6h6" />
  </svg>
);

interface InputTextareaProps {
  value: string;
  onChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  disabled: boolean;
  isCodeBlockActive: boolean;
  placeholder: string;
  editorRef: React.MutableRefObject<HTMLTextAreaElement | HTMLInputElement | null>;
  shadowRef: React.MutableRefObject<HTMLTextAreaElement | null>;
  textareaRef: React.MutableRefObject<HTMLTextAreaElement | null>;
  onContextMenu: (e: React.MouseEvent<HTMLTextAreaElement>) => void;
  isExpanded: boolean;
  setIsExpanded: React.Dispatch<React.SetStateAction<boolean>>;
}

export const InputTextarea: React.FC<InputTextareaProps> = ({
  value,
  onChange,
  onKeyDown,
  disabled,
  isCodeBlockActive,
  placeholder,
  editorRef,
  shadowRef,
  textareaRef,
  onContextMenu,
  isExpanded,
  setIsExpanded,
}) => {
  const [showExpandButton, setShowExpandButton] = useState(false);

  const resizeTextarea = useCallback(() => {
    const textarea = textareaRef.current;
    const shadow = shadowRef.current;
    if (!textarea || !shadow) return;

    shadow.value = value;

    const lineHeight = 24;
    const maxLines = isExpanded ? 15 : 10;
    const maxHeight = lineHeight * maxLines;

    const width =
      textarea.getBoundingClientRect().width || textarea.clientWidth;
    shadow.style.width = `${width}px`;

    shadow.style.height = "0px";
    const scrollHeight = shadow.scrollHeight;

    const minHeight = 32;
    const newHeight = Math.max(Math.min(scrollHeight, maxHeight), minHeight);
    textarea.style.height = `${newHeight}px`;

    setShowExpandButton(isCodeBlockActive || scrollHeight > lineHeight * 10);

    if (
      document.activeElement === textarea &&
      textarea.selectionStart >= textarea.value.length
    ) {
      if (textarea.scrollHeight > textarea.clientHeight) {
        textarea.scrollTop = textarea.scrollHeight;
      }
    }
  }, [value, isExpanded, isCodeBlockActive, textareaRef, shadowRef]);

  useLayoutEffect(() => {
    const raf = requestAnimationFrame(() => resizeTextarea());
    return () => cancelAnimationFrame(raf);
  }, [value, isExpanded, resizeTextarea]);

  return (
    <>
      <div className={styles.topRow}>
        {showExpandButton && (
          <button
            className={styles.expandButton}
            onClick={() => setIsExpanded(!isExpanded)}
          >
            {isExpanded ? <CollapseIcon /> : <ExpandIcon />}
          </button>
        )}
      </div>

      <div className={styles.inputArea}>
        {!isCodeBlockActive && value.length === 0 && (
          <div className={styles.customPlaceholder}>{placeholder}</div>
        )}
        <textarea
          ref={shadowRef}
          className={`${styles.textarea} ${styles.shadow}`}
          value={value}
          readOnly
          aria-hidden="true"
          tabIndex={-1}
          rows={1}
        />

        <textarea
          ref={(el) => {
            editorRef.current = el;
            textareaRef.current = el;
          }}
          className={styles.textarea}
          value={value}
          onChange={onChange}
          onKeyDown={onKeyDown}
          disabled={disabled}
          rows={1}
          style={{ display: isCodeBlockActive ? "none" : undefined }}
          onContextMenu={onContextMenu}
        />
      </div>
    </>
  );
};
