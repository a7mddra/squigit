/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useRef } from "react";
import { CornerDownLeft, Eraser, FolderOpen } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import {
  CodeBlock,
  WidgetOverlay,
  WidgetOverlayIconButton,
} from "@/components";
import { ACCEPTED_EXTENSIONS, getExtension, isImageExtension } from "@/core";
import styles from "./CodeEditor.module.css";

const NON_CODE_IMPORT_EXTENSIONS = new Set([
  "pdf",
  "docx",
  "doc",
  "xlsx",
  "xls",
  "pptx",
  "ppt",
  "rtf",
]);

const CODE_IMPORT_EXTENSIONS = ACCEPTED_EXTENSIONS.filter(
  (ext) => !isImageExtension(ext) && !NON_CODE_IMPORT_EXTENSIONS.has(ext),
);

const EDITOR_PLACEHOLDER =
  "Replace 'text' with a language for syntax highlighting.";

const EDITOR_STYLE: React.CSSProperties = {
  height: "100%",
  maxHeight: "none",
};

interface CodeEditorProps {
  isOpen: boolean;
  onClose: () => void;
  value: string;
  language: string;
  onValueChange: (value: string) => void;
  onLanguageChange: (language: string) => void;
  onInsert: (language: string, value: string) => void;
}

const normalizeCode = (value: string) => value.replace(/\r\n?/g, "\n");

const appendImportedCode = (currentValue: string, importedValue: string) => {
  if (!currentValue) {
    return importedValue;
  }

  return currentValue.endsWith("\n")
    ? `${currentValue}${importedValue}`
    : `${currentValue}\n${importedValue}`;
};

export const CodeEditor: React.FC<CodeEditorProps> = ({
  isOpen,
  onClose,
  value,
  language,
  onValueChange,
  onLanguageChange,
  onInsert,
}) => {
  const editorRef = useRef<HTMLTextAreaElement>(null);

  const focusEditorAtEnd = useCallback((nextValue?: string) => {
    window.setTimeout(() => {
      const textarea = editorRef.current;
      if (!textarea) return;

      const cursorPos = nextValue?.length ?? textarea.value.length;
      textarea.focus();
      textarea.setSelectionRange(cursorPos, cursorPos);
    }, 0);
  }, []);

  useEffect(() => {
    if (!isOpen) return;
    focusEditorAtEnd();
  }, [focusEditorAtEnd, isOpen]);

  const handleImportCode = useCallback(async () => {
    try {
      const result = await open({
        multiple: false,
        filters: [
          {
            name: "Text & Code Files",
            extensions: CODE_IMPORT_EXTENSIONS,
          },
          {
            name: "All Files",
            extensions: ["*"],
          },
        ],
      });

      const selectedPath = Array.isArray(result) ? result[0] : result;
      if (!selectedPath) {
        return;
      }

      const isText = await invoke<boolean>("validate_text_file", {
        path: selectedPath,
      });
      if (!isText) {
        console.warn(
          "[CodeEditor] Selected file is not a text file:",
          selectedPath,
        );
        return;
      }

      const importedText = normalizeCode(await readTextFile(selectedPath));
      const nextValue = appendImportedCode(value, importedText);
      onValueChange(nextValue);

      const importedExtension = getExtension(selectedPath);
      if (
        language.trim().toLowerCase() === "text" &&
        importedExtension &&
        importedExtension !== "file"
      ) {
        onLanguageChange(importedExtension);
      }

      focusEditorAtEnd(nextValue);
    } catch (error) {
      console.error("[CodeEditor] Failed to import code:", error);
    }
  }, [focusEditorAtEnd, language, onLanguageChange, onValueChange, value]);

  const handleInsert = useCallback(() => {
    onInsert(language.trim() || "text", value);
  }, [language, onInsert, value]);

  const handleClearCode = useCallback(() => {
    onLanguageChange("text");
    onValueChange("");
    focusEditorAtEnd("");
  }, [focusEditorAtEnd, onLanguageChange, onValueChange]);

  return (
    <WidgetOverlay
      isOpen={isOpen}
      onClose={onClose}
      sectionContentClassName={styles.sectionContent}
      sidebarBottom={
        <>
          <WidgetOverlayIconButton
            icon={<Eraser size={20} />}
            label="clear code"
            onClick={handleClearCode}
          />
          <WidgetOverlayIconButton
            icon={<FolderOpen size={22} />}
            label="import code"
            onClick={handleImportCode}
          />
        </>
      }
    >
      <div className={styles.editorRoot}>
        <CodeBlock
          ref={editorRef}
          language={language || "text"}
          value={value}
          isEditor
          onChange={onValueChange}
          onLanguageChange={onLanguageChange}
          placeholder={EDITOR_PLACEHOLDER}
          actionLabel="insert"
          actionTitle="insert code"
          actionIcon={<CornerDownLeft size={14} />}
          onAction={handleInsert}
          fillHeight
          style={EDITOR_STYLE}
        />
      </div>
    </WidgetOverlay>
  );
};
