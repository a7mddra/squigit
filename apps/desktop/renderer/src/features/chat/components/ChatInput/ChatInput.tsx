/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useRef, useState } from "react";
import { useTextEditor, useTextContextMenu } from "@/hooks";
import { TextContextMenu } from "@/layout";
import { useAppContext } from "@/providers/AppProvider";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  AttachmentStrip,
  isImageExtension,
  getExtension,
  attachmentFromPath,
  ACCEPTED_EXTENSIONS,
} from "@/features";
import { InputTextarea } from "./InputTextarea";
import { InputCodeEditor, useCodeEditor } from "./InputCodeEditor";
import { InputActions } from "./InputActions";
import type { ChatInputProps } from "./chat-input.types";
import styles from "./ChatInput.module.css";

export const ChatInput: React.FC<ChatInputProps> = ({
  startupImage,
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
}) => {
  const app = useAppContext();
  if (!startupImage) return null;

  const placeholder = customPlaceholder || "Ask anything";
  const disabled = isLoading && !isAiTyping;

  const [isExpanded, setIsExpanded] = useState(false);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const shadowRef = useRef<HTMLTextAreaElement>(null);
  const codeTaRef = useRef<HTMLTextAreaElement>(null);

  const {
    isCodeBlockActive,
    codeLanguage,
    codeValue,
    setCodeValue,
    handleChange,
    handleCodeKeyDown,
    resetCodeEditor,
  } = useCodeEditor({
    value,
    onChange,
    textareaRef,
  });

  const handleSubmit = () => {
    if (
      !disabled &&
      !isLoading &&
      (value.trim().length > 0 ||
        codeValue.trim().length > 0 ||
        attachments.length > 0)
    ) {
      onSend();
      resetCodeEditor();
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
            ...attachments,
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
    (value.trim().length > 0 ||
      codeValue.trim().length > 0 ||
      attachments.length > 0);

  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlistenDrop = listen<{ paths: string[] }>(
      "tauri://drag-drop",
      async (event) => {
        const paths = Array.from(event.payload.paths || []);
        if (paths.length === 0) return;

        const newAttachments = [...attachments];
        for (const filePath of paths) {
          const originalName = filePath.split(/[/\\]/).pop() || filePath;
          const ext = getExtension(filePath);
          const isAllowed = ACCEPTED_EXTENSIONS.includes(ext);

          if (isImageExtension(ext)) {
            try {
              const result = await invoke<{ hash: string; path: string }>(
                "store_image_from_path",
                { path: filePath },
              );
              newAttachments.push(
                attachmentFromPath(
                  result.path,
                  undefined,
                  originalName,
                  filePath,
                ),
              );
            } catch (err) {
              console.error("Failed to store dropped image:", err);
            }
          } else if (isAllowed) {
            try {
              const result = await invoke<{ hash: string; path: string }>(
                "store_file_from_path",
                { path: filePath },
              );
              newAttachments.push(
                attachmentFromPath(
                  result.path,
                  undefined,
                  originalName,
                  filePath,
                ),
              );
            } catch (err) {
              console.error("Failed to store dropped file:", err);
            }
          } else {
            // Check if it's a valid text file
            try {
              const isText = await invoke<boolean>("validate_text_file", {
                path: filePath,
              });
              if (isText) {
                const result = await invoke<{ hash: string; path: string }>(
                  "store_file_from_path",
                  { path: filePath },
                );
                newAttachments.push(
                  attachmentFromPath(
                    result.path,
                    undefined,
                    originalName,
                    filePath,
                  ),
                );
              } else {
                console.warn(
                  "Dropped file is binary and not supported:",
                  filePath,
                );
              }
            } catch (err) {
              console.error("Failed to validate dropped text file:", err);
            }
          }
        }
        if (newAttachments.length > attachments.length) {
          onAttachmentsChange(newAttachments);
        }
      },
    );

    return () => {
      unlistenDrop.then((fn) => fn());
    };
  }, [attachments, onAttachmentsChange]);

  const containerContent = (
    <div
      className={`${styles.container} ${disabled && !isStoppable ? styles.disabled : ""} ${
        variant === "transparent" ? styles.transparentVariant : ""
      }`}
      ref={containerRef}
    >
      <AttachmentStrip
        attachments={attachments}
        onClick={app.openMediaViewer}
        onRemove={(id) =>
          onAttachmentsChange(attachments.filter((a) => a.id !== id))
        }
      />

      <InputTextarea
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        isCodeBlockActive={isCodeBlockActive}
        placeholder={placeholder}
        editorRef={editorRef}
        shadowRef={shadowRef}
        textareaRef={textareaRef}
        onContextMenu={handleContextMenu}
        isExpanded={isExpanded}
        setIsExpanded={setIsExpanded}
      />
      {contextMenuData.isOpen && !isCodeBlockActive && (
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
                  ...attachments,
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

      <InputCodeEditor
        isCodeBlockActive={isCodeBlockActive}
        codeLanguage={codeLanguage}
        codeValue={codeValue}
        onCodeValueChange={setCodeValue}
        onCodeKeyDown={handleCodeKeyDown}
        isExpanded={isExpanded}
        codeTaRef={codeTaRef}
      />

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
          const newAttachments = [...attachments];
          for (const filePath of paths) {
            const originalName = filePath.split(/[/\\]/).pop() || filePath;
            const ext = getExtension(filePath);
            const isAllowed = ACCEPTED_EXTENSIONS.includes(ext);

            if (isImageExtension(ext)) {
              try {
                const result = await invoke<{ hash: string; path: string }>(
                  "store_image_from_path",
                  { path: filePath },
                );
                newAttachments.push(
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
            } else if (isAllowed) {
              try {
                const result = await invoke<{ hash: string; path: string }>(
                  "store_file_from_path",
                  { path: filePath },
                );
                newAttachments.push(
                  attachmentFromPath(
                    result.path,
                    undefined,
                    originalName,
                    filePath,
                  ),
                );
              } catch (err) {
                console.error("Failed to store file:", err);
              }
            } else {
              // Validated unknown extensions from 'All files'
              try {
                const isText = await invoke<boolean>("validate_text_file", {
                  path: filePath,
                });
                if (isText) {
                  const result = await invoke<{ hash: string; path: string }>(
                    "store_file_from_path",
                    { path: filePath },
                  );
                  newAttachments.push(
                    attachmentFromPath(
                      result.path,
                      undefined,
                      originalName,
                      filePath,
                    ),
                  );
                } else {
                  console.warn(
                    "Selected file is binary and not supported:",
                    filePath,
                  );
                }
              } catch (err) {
                console.error("Failed to validate selected text file:", err);
              }
            }
          }
          if (newAttachments.length > attachments.length) {
            onAttachmentsChange(newAttachments);
          }
        }}
      />
    </div>
  );

  if (variant === "transparent") {
    return containerContent;
  }

  return (
    <footer className={styles.footer}>
      <div className={styles.inputWrapper}>{containerContent}</div>
    </footer>
  );
};

export default ChatInput;
