/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useRef } from "react";
import { X } from "lucide-react";
import styles from "./WidgetOverlay.module.css";

interface WidgetOverlayProps {
  isOpen: boolean;
  onClose: () => void;
  onContextMenu?: React.MouseEventHandler<HTMLDivElement>;
  sidebarTop?: React.ReactNode;
  sidebarMiddle?: React.ReactNode;
  sidebarBottom?: React.ReactNode;
  contentClassName?: string;
  sectionContentClassName?: string;
  children: React.ReactNode;
}

const joinClasses = (base: string, extra?: string) =>
  extra ? `${base} ${extra}` : base;

export const WidgetOverlay: React.FC<WidgetOverlayProps> = ({
  isOpen,
  onClose,
  onContextMenu,
  sidebarTop,
  sidebarMiddle,
  sidebarBottom,
  contentClassName,
  sectionContentClassName,
  children,
}) => {
  const appRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Element;
      const isContextMenu = target.closest('[data-is-context-menu="true"]');
      const isDialog = target.closest('[data-dialog-container="true"]');
      const isTitleBar = target.closest("[data-tauri-drag-region]");

      if (
        isOpen &&
        appRef.current &&
        !appRef.current.contains(target as Node) &&
        !isContextMenu &&
        !isDialog &&
        !isTitleBar
      ) {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen, onClose]);

  return (
    <div
      className={`${styles.overlay} ${isOpen ? styles.open : ""}`}
      onContextMenu={onContextMenu}
    >
      <div
        ref={appRef}
        className={`${styles.container} ${isOpen ? styles.open : ""}`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className={styles.sidebar}>
          <div className={styles.sidebarSection}>
            <button className={styles.closeButton} onClick={onClose}>
              <X size={18} />
            </button>
            {sidebarTop}
          </div>

          {sidebarMiddle ? (
            <>
              <div className={styles.spacer} />
              <div className={styles.sidebarSection}>{sidebarMiddle}</div>
              <div className={styles.spacer} />
            </>
          ) : (
            <div className={styles.spacer} />
          )}

          <div className={`${styles.sidebarSection} ${styles.footer}`}>
            {sidebarBottom}
          </div>
        </div>

        <div className={joinClasses(styles.content, contentClassName)}>
          <div
            className={joinClasses(
              styles.sectionContent,
              sectionContentClassName,
            )}
          >
            {children}
          </div>
        </div>
      </div>
    </div>
  );
};
