/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useRef, useState } from "react";
import { ArrowDown } from "lucide-react";
import { useTextEditor, useTextContextMenu } from "@/hooks";
import { TextContextMenu } from "@/layout";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  attachmentFromPath,
  buildAttachmentMention,
  getExtension,
  isAcceptedExtension,
  isImageExtension,
} from "@/lib";
import { InputTextarea } from "./InputTextarea";
import { InputActions } from "./InputActions";
import { ImageStrip } from "./ImageStrip";
import { CodeEditor } from "./CodeEditor";
import type { ChatInputProps } from "./chat-input.types";
import styles from "./ChatInput.module.css";

export const ChatInput: React.FC<ChatInputProps> = React.memo(({
  startupImage,
  forceVisible = false,
  isNavigating = false,
  input: value,
  onInputChange: onChange,
  onSend,
  isLoading,
  isAiTyping = false,
  isStoppable = false,
  onStopGeneration,
  placeholder: customPlaceholder,
  variant = "default",
  selectedModel,
  onModelChange,
  attachments,
  onAttachmentsChange,
  onCaptureToInput,
  onPreviewAttachment,
  rememberAttachmentSourcePath,
  showScrollToBottomButton = false,
  onScrollToBottom,
}) => {
  if (!startupImage && !forceVisible && !isNavigating) return null;

  const placeholder = customPlaceholder || "Ask anything";
  const disabled = isLoading && !isAiTyping;

  const [isExpanded, setIsExpanded] = useState(false);
  const [isCodeEditorOpen, setIsCodeEditorOpen] = useState(false);
  const [codeEditorValue, setCodeEditorValue] = useState("");
  const [codeEditorLanguage, setCodeEditorLanguage] = useState("text");

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const shadowRef = useRef<HTMLTextAreaElement>(null);

  const handleSubmit = () => {
    if (!disabled && !isLoading && (value.trim().length > 0 || attachments.length > 0)) {
      onSend();
    }
  };

  const {
    ref: editorRef,
    handleKeyDown: editorKeyDown,
    hasSelection,
    handleCopy,
    handleCut,
    handlePaste,
    handleSelectAll,
    undo,
    redo,
  } = useTextEditor({
    value,
    onChange: (newValue) => {
      onChange(newValue);
    },
    onSubmit: handleSubmit,
    preventNewLine: false,
  });

  const {
    data: contextMenuData,
    handleContextMenu,
    handleClose: handleCloseContextMenu,
  } = useTextContextMenu();

  const handleKeyDown = async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Intercept Ctrl+V for images
    if ((e.ctrlKey || e.metaKey) && e.key === "v") {
      try {
        const result = await invoke<{ hash: string; path: string }>(
          "read_clipboard_image",
        );
        if (result && result.path) {
          onAttachmentsChange([
            ...latestAttachmentsRef.current,
            attachmentFromPath(result.path),
          ]);
        }
      } catch (err) {
        // Not an image or clipboard error, allow default paste behavior
      }
    }
    editorKeyDown(e);
  };

  const isButtonActive =
    !disabled &&
    !isLoading &&
    (value.trim().length > 0 || attachments.length > 0);

  const containerRef = useRef<HTMLDivElement>(null);
  const latestValueRef = useRef(value);
  const latestAttachmentsRef = useRef(attachments);

  useEffect(() => {
    latestValueRef.current = value;
  }, [value]);

  useEffect(() => {
    latestAttachmentsRef.current = attachments;
  }, [attachments]);

  const appendCodeBlockToInput = useCallback(
    (language: string, code: string) => {
      const normalizedLanguage = language.trim() || "text";
      const normalizedCode = code.replace(/\r\n?/g, "\n").replace(/\n+$/u, "");
      const currentValue = latestValueRef.current;
      const prefix =
        currentValue.length > 0 && !currentValue.endsWith("\n") ? "\n" : "";
      const nextValue =
        currentValue +
        `${prefix}\`\`\`${normalizedLanguage}\n${normalizedCode}\n\`\`\`\n`;

      onChange(nextValue);
      window.setTimeout(() => {
        const nextTextarea = textareaRef.current;
        if (!nextTextarea) return;
        nextTextarea.focus();
        nextTextarea.setSelectionRange(nextValue.length, nextValue.length);
      }, 0);
    },
    [onChange],
  );

  const insertTextAtCaret = useCallback(
    (textToInsert: string) => {
      if (!textToInsert) return;

      const textarea = textareaRef.current;
      if (!textarea) {
        onChange(latestValueRef.current + textToInsert);
        return;
      }

      const currentValue = latestValueRef.current;
      const start = textarea.selectionStart ?? currentValue.length;
      const end = textarea.selectionEnd ?? start;
      const nextValue =
        currentValue.slice(0, start) +
        textToInsert +
        currentValue.slice(end);
      const cursorPos = start + textToInsert.length;

      onChange(nextValue);
      window.setTimeout(() => {
        const nextTextarea = textareaRef.current;
        if (!nextTextarea) return;
        nextTextarea.focus();
        nextTextarea.setSelectionRange(cursorPos, cursorPos);
      }, 0);
    },
    [onChange],
  );

  const insertInlineFileLinks = useCallback(
    (links: string[]) => {
      if (links.length === 0) return;

      insertTextAtCaret(`${links.join(" ")} `);
    },
    [insertTextAtCaret],
  );

  const handleFilePaths = useCallback(
    async (paths: string[]) => {
      const currentAttachments = latestAttachmentsRef.current;
      const nextAttachments = [...currentAttachments];
      const inlineLinks: string[] = [];

      for (const filePath of paths) {
        const originalName = filePath.split(/[/\\]/).pop() || filePath;
        const ext = getExtension(filePath);
        const isAllowed = isAcceptedExtension(ext);

        if (isImageExtension(ext)) {
          try {
            const result = await invoke<{ hash: string; path: string }>(
              "store_image_from_path",
              { path: filePath },
            );
            rememberAttachmentSourcePath?.(result.path, filePath);
            nextAttachments.push(
              attachmentFromPath(
                result.path,
                undefined,
                originalName,
                filePath,
              ),
            );
          } catch (err) {
            console.error("Failed to store image:", err);
          }
          continue;
        }

        if (isAllowed) {
          try {
            const result = await invoke<{ hash: string; path: string }>(
              "store_file_from_path",
              { path: filePath },
            );
            rememberAttachmentSourcePath?.(result.path, filePath);
            inlineLinks.push(buildAttachmentMention(result.path, originalName));
          } catch (err) {
            console.error("Failed to store file:", err);
          }
          continue;
        }

        try {
          const isText = await invoke<boolean>("validate_text_file", {
            path: filePath,
          });
          if (!isText) {
            console.warn("Selected file is binary and not supported:", filePath);
            continue;
          }

          const result = await invoke<{ hash: string; path: string }>(
            "store_file_from_path",
            { path: filePath },
          );
          rememberAttachmentSourcePath?.(result.path, filePath);
          inlineLinks.push(buildAttachmentMention(result.path, originalName));
        } catch (err) {
          console.error("Failed to validate selected text file:", err);
        }
      }

      if (nextAttachments.length > currentAttachments.length) {
        onAttachmentsChange(nextAttachments);
      }
      if (inlineLinks.length > 0) {
        insertInlineFileLinks(inlineLinks);
      }
    },
    [
      insertInlineFileLinks,
      onAttachmentsChange,
      rememberAttachmentSourcePath,
    ],
  );

  useEffect(() => {
    const unlistenDrop = listen<{ paths: string[] }>(
      "tauri://drag-drop",
      async (event) => {
        const paths = Array.from(event.payload.paths || []);
        if (paths.length === 0) return;
        await handleFilePaths(paths);
      },
    );

    return () => {
      unlistenDrop.then((fn) => fn());
    };
  }, [handleFilePaths]);

  const containerContent = (
    <div
      className={`${styles.container} ${disabled && !isStoppable ? styles.disabled : ""} ${
        variant === "transparent" ? styles.transparentVariant : ""
      }`}
      ref={containerRef}
    >
      <ImageStrip
        attachments={attachments}
        onClick={onPreviewAttachment}
        onRemove={(id) =>
          onAttachmentsChange(attachments.filter((a) => a.id !== id))
        }
      />

      <InputTextarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder={placeholder}
        editorRef={editorRef}
        shadowRef={shadowRef}
        textareaRef={textareaRef}
        onContextMenu={handleContextMenu}
        isExpanded={isExpanded}
        setIsExpanded={setIsExpanded}
      />
      {contextMenuData.isOpen && (
        <TextContextMenu
          x={contextMenuData.x}
          y={contextMenuData.y}
          onClose={handleCloseContextMenu}
          onCopy={handleCopy}
          onCut={handleCut}
          onPaste={async () => {
            try {
              const result = await invoke<{ hash: string; path: string }>(
                "read_clipboard_image",
              );
              if (result && result.path) {
                onAttachmentsChange([
                  ...latestAttachmentsRef.current,
                  attachmentFromPath(result.path),
                ]);
                return;
              }
            } catch (err) {
              // Not an image, fall through to text paste
            }
            handlePaste();
          }}
          onSelectAll={handleSelectAll}
          onUndo={undo}
          onRedo={redo}
          hasSelection={hasSelection}
        />
      )}

      <InputActions
        onSubmit={handleSubmit}
        onStop={() => onStopGeneration?.()}
        isButtonActive={isButtonActive}
        isAiTyping={isAiTyping}
        isStoppable={isStoppable}
        disabled={disabled}
        selectedModel={selectedModel}
        onModelChange={onModelChange}
        onTranscript={(text, isFinal) => {
          if (isFinal) {
            onChange((value + " " + text).trim());
          }
        }}
        onCaptureToInput={onCaptureToInput}
        onFilePaths={async (paths: string[]) => {
          await handleFilePaths(paths);
        }}
        onOpenCodeEditor={() => setIsCodeEditorOpen(true)}
      />
    </div>
  );

  const codeEditorOverlay = (
    <CodeEditor
      isOpen={isCodeEditorOpen}
      onClose={() => setIsCodeEditorOpen(false)}
      value={codeEditorValue}
      language={codeEditorLanguage}
      onValueChange={setCodeEditorValue}
      onLanguageChange={setCodeEditorLanguage}
      onInsert={(language, code) => {
        appendCodeBlockToInput(language, code);
        setIsCodeEditorOpen(false);
      }}
    />
  );

  if (variant === "transparent") {
    return (
      <>
        {containerContent}
        {codeEditorOverlay}
      </>
    );
  }

  return (
    <>
      <footer className={styles.footer}>
        <div className={styles.inputWrapper}>
          {showScrollToBottomButton && onScrollToBottom && (
            <button
              type="button"
              className={`${styles.iconButton} ${styles.scrollToBottomButton}`}
              onClick={onScrollToBottom}
              aria-label="Scroll to bottom"
            >
              <ArrowDown size={14} />
            </button>
          )}
          {containerContent}
        </div>
      </footer>
      {codeEditorOverlay}
    </>
  );
});

ChatInput.displayName = "ChatInput";

export default ChatInput;
