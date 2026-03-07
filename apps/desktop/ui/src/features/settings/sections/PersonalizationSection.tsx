/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { TextContextMenu } from "@/layout";
import { useTextEditor, useTextContextMenu } from "@/hooks";
import { UserPreferences } from "@/lib";
import styles from "./PersonalizationSection.module.css";

interface PersonalizationSectionProps {
  localPrompt: string;
  currentPrompt: string;
  setLocalPrompt: (prompt: string) => void;
  updatePreferences: (updates: Partial<UserPreferences>) => void;
}
export const PersonalizationSection: React.FC<PersonalizationSectionProps> = ({
  localPrompt,
  currentPrompt,
  setLocalPrompt,
  updatePreferences,
}) => {
  React.useEffect(() => {
    if (localPrompt === currentPrompt) return;
    const handler = setTimeout(() => {
      updatePreferences({ prompt: localPrompt });
    }, 1000);
    return () => {
      clearTimeout(handler);
    };
  }, [localPrompt, currentPrompt, updatePreferences]);
  const {
    ref,
    hasSelection,
    handleCopy,
    handleCut,
    handlePaste,
    handleSelectAll,
    handleKeyDown,
  } = useTextEditor({
    value: localPrompt,
    onChange: setLocalPrompt,
  });
  const {
    data: contextMenu,
    handleContextMenu,
    handleClose: handleCloseContextMenu,
  } = useTextContextMenu({
    hasSelection,
  });
  return (
    <section
      className={styles.container}
      aria-labelledby="personalization-heading"
    >
      <header className={styles.sectionHeader}>
        <h2 id="personalization-heading" className={styles.sectionTitle}>
          Personalization
        </h2>
      </header>
      <div className={styles.group}>
        <div className={styles.controlsRow}>
          <span className={styles.description}>
            Customize how the AI responds to you
          </span>
        </div>
        <div className={styles.textareaWrapper}>
          <textarea
            ref={ref as React.RefObject<HTMLTextAreaElement>}
            className={styles.textarea}
            placeholder="Add context about yourself, your preferences, or specific instructions for the AI."
            value={localPrompt}
            onChange={(e) => setLocalPrompt(e.target.value)}
            onContextMenu={handleContextMenu}
            onKeyDown={handleKeyDown as any}
          />
        </div>
      </div>
      {contextMenu.isOpen && (
        <TextContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={handleCloseContextMenu}
          onCopy={handleCopy}
          onCut={handleCut}
          onPaste={handlePaste}
          onSelectAll={handleSelectAll}
          hasSelection={hasSelection}
        />
      )}
    </section>
  );
};
