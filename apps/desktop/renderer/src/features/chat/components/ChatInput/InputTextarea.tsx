/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useLayoutEffect, useCallback, useState } from "react";
import { CollapseTextareaIcon, ExpandTextareaIcon } from "@/assets";
import styles from "./ChatInput.module.css";

interface InputTextareaProps {
  value: string;
  onChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  disabled: boolean;
  placeholder: string;
  editorRef: React.MutableRefObject<
    HTMLTextAreaElement | HTMLInputElement | null
  >;
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

    setShowExpandButton(scrollHeight > lineHeight * 10);

    if (
      document.activeElement === textarea &&
      textarea.selectionStart >= textarea.value.length
    ) {
      if (textarea.scrollHeight > textarea.clientHeight) {
        textarea.scrollTop = textarea.scrollHeight;
      }
    }
  }, [value, isExpanded, textareaRef, shadowRef]);

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
            {isExpanded ? (
              <CollapseTextareaIcon size={14} />
            ) : (
              <ExpandTextareaIcon size={14} />
            )}
          </button>
        )}
      </div>

      <div className={styles.inputArea}>
        {value.length === 0 && (
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
          onContextMenu={onContextMenu}
        />
      </div>
    </>
  );
};
