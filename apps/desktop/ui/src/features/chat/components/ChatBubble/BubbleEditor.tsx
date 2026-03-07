/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useEffect,
  useRef,
  useLayoutEffect,
  useImperativeHandle,
} from "react";
import styles from "./ChatBubble.module.css";

interface BubbleEditorProps {
  value: string;
  onChange: (value: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
  width?: number;
}

const AutoResizeTextarea = React.forwardRef<
  HTMLTextAreaElement,
  {
    value: string;
    onChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
    onKeyDown?: (e: React.KeyboardEvent) => void;
    placeholder?: string;
  }
>(({ value, onChange, onKeyDown, placeholder }, ref) => {
  const innerRef = useRef<HTMLTextAreaElement>(null);

  useImperativeHandle(ref, () => innerRef.current as HTMLTextAreaElement);

  useLayoutEffect(() => {
    if (innerRef.current) {
      innerRef.current.style.height = "auto";
      innerRef.current.style.height = `${innerRef.current.scrollHeight}px`;
    }
  }, [value]);

  return (
    <textarea
      ref={innerRef}
      value={value}
      onChange={onChange}
      onKeyDown={onKeyDown}
      className={styles.editorSegmentText}
      placeholder={placeholder}
      rows={1}
    />
  );
});

AutoResizeTextarea.displayName = "AutoResizeTextarea";

export const BubbleEditor: React.FC<BubbleEditorProps> = ({
  value,
  onChange,
  onConfirm,
  onCancel,
  width,
}) => {
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && e.shiftKey) {
      e.stopPropagation();
      return;
    }

    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      onConfirm();
    }
    if (e.key === "Escape") {
      onCancel();
    }
  };

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textareaRef.current) {
      const el = textareaRef.current;
      el.focus({ preventScroll: true });
      const len = el.value.length;
      el.setSelectionRange(len, len);
      el.scrollTop = el.scrollHeight;
    }
  }, []);

  return (
    <div
      className={styles.messageEditor}
      style={{ width: width ? `${width}px` : "100%" }}
    >
      <AutoResizeTextarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
      />
      <div className={styles.editActions}>
        <button onClick={onCancel} className={styles.cancelButton}>
          Cancel
        </button>
        <button onClick={onConfirm} className={styles.saveButton}>
          Save
        </button>
      </div>
    </div>
  );
};
