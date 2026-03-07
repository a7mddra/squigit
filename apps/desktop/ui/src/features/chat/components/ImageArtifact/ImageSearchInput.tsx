/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { ChevronUp, Loader2 } from "lucide-react";
import { TextContextMenu } from "@/layout";
import { useTextContextMenu, useTextEditor } from "@/hooks";
import { OCRModelSwitcher, SettingsSection } from "@/features";
import styles from "./ImageArtifact.module.css";

interface ImageSearchInputProps {
  value: string;
  onChange: (value: string) => void;
  onLensClick: (query: string) => void;
  onTranslateClick: () => void;
  onCollapse?: () => void;
  isLensLoading: boolean;
  isTranslateDisabled: boolean;
  isOCRLoading: boolean;
  isExpanded?: boolean;
  placeholder?: string;
  currentOcrModel: string;
  onOcrModelChange: (model: string) => void;
  onOpenSettings: (section: SettingsSection) => void;
}

export const ImageSearchInput: React.FC<ImageSearchInputProps> = ({
  value,
  onChange,
  onLensClick,
  onTranslateClick,
  onCollapse,
  isLensLoading,
  isTranslateDisabled,
  isOCRLoading,
  isExpanded = false,
  placeholder = "Add to your search",
  currentOcrModel,
  onOcrModelChange,
  onOpenSettings,
}) => {
  const {
    ref,
    hasSelection,
    handleCopy,
    handleCut,
    handlePaste,
    handleSelectAll,
    handleKeyDown,
  } = useTextEditor({
    value,
    onChange,
    onSubmit: () => {
      if (!isLensLoading && !isOCRLoading) {
        onLensClick(value);
      }
    },
    preventNewLine: true,
  });

  const {
    data: contextMenu,
    handleContextMenu,
    handleClose: handleCloseContextMenu,
  } = useTextContextMenu({
    hasSelection,
  });

  return (
    <div className={styles.searchContainer}>
      <input
        ref={ref as React.RefObject<HTMLInputElement>}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown as any}
        onContextMenu={handleContextMenu}
        placeholder={placeholder}
        disabled={isOCRLoading}
        className={styles.searchInput}
      />

      <div className={styles.searchActions}>
        {!isExpanded && (
          <>
            <button
              className={styles.actionBtn}
              onClick={() => onLensClick(value)}
              disabled={isLensLoading || isOCRLoading}
              title="Search with Google Lens"
            >
              {isLensLoading ? (
                <Loader2 size={20} className={styles.spinner} />
              ) : (
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="22"
                  height="22"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z" />
                  <circle cx="12" cy="13" r="3" />
                </svg>
              )}
            </button>

            <button
              className={styles.actionBtn}
              onClick={onTranslateClick}
              disabled={isTranslateDisabled || isOCRLoading}
              title="Translate all text"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="22"
                height="22"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="m5 8 6 6" />
                <path d="m4 14 6-6 2-3" />
                <path d="M2 5h12" />
                <path d="M7 2h1" />
                <path d="m22 22-5-10-5 10" />
                <path d="M14 18h6" />
              </svg>
            </button>

            <div className={styles.searchDivider} />
            <OCRModelSwitcher
              currentOcrModel={currentOcrModel}
              onOcrModelChange={onOcrModelChange}
              onOpenSettings={onOpenSettings}
              disabled={isOCRLoading}
            />
          </>
        )}

        {isExpanded && onCollapse && (
          <button
            className={styles.actionBtn}
            onClick={onCollapse}
            title="Collapse"
          >
            <ChevronUp size={22} />
          </button>
        )}
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
    </div>
  );
};

export default ImageSearchInput;
