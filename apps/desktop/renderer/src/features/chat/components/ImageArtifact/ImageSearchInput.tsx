/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { ChevronUp, Loader2 } from "lucide-react";
import { GoogleLensIcon, TranslateIcon } from "@/components/icons";
import { TextContextMenu } from "@/app/shell/menus/TextContextMenu";
import { useTextContextMenu, useTextEditor } from "@/hooks";
import { OCRModelSwitcher } from "@/features/ocr";
import { SettingsSection } from "@/features/settings";
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
                <GoogleLensIcon size={24} color="currentColor" />
              )}
            </button>

            <button
              className={styles.actionBtn}
              onClick={onTranslateClick}
              disabled={isTranslateDisabled || isOCRLoading}
              title="Translate all text"
            >
              <TranslateIcon size={22} />
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
