/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useRef, useCallback } from "react";
import { Paperclip, ArrowUp, Square, Camera } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { MODELS } from "@/lib";
import {
  Dropdown,
  DropdownItem,
  DropdownSectionTitle,
  Tooltip,
} from "@/components";
import { VoiceButton } from "./VoiceButton";
import styles from "./ChatInput.module.css";

const GEMINI_MODELS = MODELS.map((m) => ({
  id: m.id,
  label: m.name,
  triggerLabel: m.name.replace("Gemini ", ""),
}));

import { ACCEPTED_EXTENSIONS } from "@/features";

interface InputActionsProps {
  onSubmit: () => void;
  onStop?: () => void;
  isButtonActive: boolean;
  isAiTyping: boolean;
  isStoppable: boolean;
  disabled: boolean;
  selectedModel: string;
  onModelChange: (model: string) => void;
  onTranscript: (text: string, isFinal: boolean) => void;
  onCaptureToInput?: () => void;
  onFilePaths?: (paths: string[]) => void;
}

export const InputActions: React.FC<InputActionsProps> = ({
  onSubmit,
  onStop,
  isButtonActive,
  isAiTyping,
  isStoppable,
  disabled,
  selectedModel,
  onModelChange,
  onTranscript,
  onCaptureToInput,
  onFilePaths,
}) => {
  const [showFileButtonTooltip, setShowFileButtonTooltip] = useState(false);
  const [showKeepProgressTooltip, setShowKeepProgressTooltip] = useState(false);
  const [aiMenuOpen, setAiMenuOpen] = useState(false);

  const fileButtonRef = useRef<HTMLButtonElement>(null);
  const keepProgressInfoRef = useRef<HTMLButtonElement>(null);

  const handlePaperclipClick = async () => {
    try {
      const result = await open({
        multiple: true,
        filters: [
          {
            name: "Supported Files",
            extensions: ACCEPTED_EXTENSIONS,
          },
          {
            name: "All Files",
            extensions: ["*"],
          },
        ],
      });
      if (result) {
        const paths = Array.isArray(result) ? result : [result];
        onFilePaths?.(paths);
      }
    } catch (err) {
      console.error("Failed to open file dialog:", err);
    }
  };

  const handleModelSelect = useCallback(
    (id: string) => {
      onModelChange(id);
      setAiMenuOpen(false);
    },
    [onModelChange],
  );

  const selectedModelLabel =
    GEMINI_MODELS.find((m) => m.id === selectedModel)?.triggerLabel || "Auto";

  return (
    <div className={styles.actions}>
      <div className={styles.leftActions}>
        <button
          className={styles.iconButton}
          aria-label="Attach file"
          ref={fileButtonRef}
          onMouseEnter={() => setShowFileButtonTooltip(true)}
          onMouseLeave={() => setShowFileButtonTooltip(false)}
          onClick={handlePaperclipClick}
        >
          <Paperclip size={18} />
        </button>
        <Tooltip
          text="Add photos & files"
          parentRef={fileButtonRef}
          show={showFileButtonTooltip}
          above
        />

        <div className={styles.keepProgressGroup}>
          <button
            className={styles.toggleItem}
            aria-label="Keep Progress"
            ref={keepProgressInfoRef}
            onMouseEnter={() => setShowKeepProgressTooltip(true)}
            onMouseLeave={() => setShowKeepProgressTooltip(false)}
            onClick={async () => {
              setShowKeepProgressTooltip(false);
              // Wait for the next macro task so React flushes the DOM updates and the tooltip disappears
              await new Promise((r) => setTimeout(r, 0));
              onCaptureToInput?.();
            }}
          >
            <Camera size={16} />
            <span>Keep Progress</span>
          </button>
          <Tooltip
            text="Bring your screen directly to the chat"
            parentRef={keepProgressInfoRef}
            show={showKeepProgressTooltip}
            above
          />
        </div>

        <Dropdown
          label={selectedModelLabel}
          direction="up"
          isOpen={aiMenuOpen}
          onOpenChange={setAiMenuOpen}
          width={180}
          align="left"
        >
          <DropdownSectionTitle>Model</DropdownSectionTitle>
          {GEMINI_MODELS.map((model) => (
            <div
              key={model.id}
              style={{
                marginTop: "2px",
              }}
            >
              <DropdownItem
                label={model.label}
                isActive={model.id === selectedModel}
                onClick={() => handleModelSelect(model.id)}
              />
            </div>
          ))}
        </Dropdown>
      </div>

      <div className={styles.rightActions}>
        <VoiceButton onTranscript={onTranscript} disabled={disabled} />

        {isAiTyping || isStoppable ? (
          <button
            className={styles.stopButton}
            onClick={onStop}
            aria-label="Stop generating"
          >
            <Square size={14} fill="currentColor" />
          </button>
        ) : (
          <button
            className={`${styles.submitButton} ${isButtonActive ? styles.submitActive : ""}`}
            onClick={onSubmit}
            disabled={!isButtonActive}
            aria-label="Submit"
          >
            <ArrowUp size={18} />
          </button>
        )}
      </div>
    </div>
  );
};
