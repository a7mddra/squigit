/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useRef } from "react";

import { createPortal } from "react-dom";
import { AlertCircle, AlertTriangle, Info, Sparkles } from "lucide-react";
import styles from "./Dialog.module.css";

export interface DialogAction {
  label: string;
  onClick: () => void;
  variant?: "primary" | "secondary" | "danger";
  disabled?: boolean;
}

import { commands } from "@/platform/tauri";
import {
  getDialogs,
  type DialogContent,
  type DialogVariant,
} from "@squigit/core/helpers";

interface DialogProps {
  variant?: DialogVariant;
  title?: string;
  message?: string;
  actions?: DialogAction[];
  isOpen: boolean;
  type?: string | DialogContent;
  appName?: string;
  onAction?: (actionKey: string) => void;
}

export const Dialog: React.FC<DialogProps> = ({
  variant = "error",
  title,
  message,
  actions,
  isOpen,
  type,
  onAction,
  appName = "Squigit",
}) => {
  let activeContent: Partial<DialogContent> = {};

  if (typeof type === "string") {
    const dialogs = getDialogs(appName);
    if (dialogs[type]) {
      activeContent = dialogs[type];
    }
  } else if (typeof type === "object") {
    activeContent = type;
  }

  const finalVariant = activeContent.variant || variant;
  const finalTitle = activeContent.title || title;
  const finalMessage = activeContent.message || message || "";

  const finalActions = activeContent.actions
    ? activeContent.actions.map((a) => ({
        ...a,
        onClick: () => onAction?.(a.actionKey),
      }))
    : actions || [];

  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (isOpen && textAreaRef.current) {
      textAreaRef.current.style.height = "auto";
      textAreaRef.current.style.height = `${textAreaRef.current.scrollHeight}px`;
    }
  }, [isOpen, finalMessage]);

  useEffect(() => {
    if (isOpen) {
      document.body.classList.add("has-open-dialog");
      void commands.playUiSound("dialog-warning").catch((error) => {
        console.warn("[dialog] Failed to play ui sound:", error);
      });
    } else {
      document.body.classList.remove("has-open-dialog");
    }

    return () => {
      document.body.classList.remove("has-open-dialog");
    };
  }, [isOpen]);

  if (!isOpen) return null;

  const getIcon = () => {
    switch (finalVariant) {
      case "error":
        return <AlertCircle size={18} className={styles.iconError} />;
      case "warning":
        return <AlertTriangle size={18} className={styles.iconWarning} />;
      case "info":
        return <Info size={18} className={styles.iconInfo} />;
      case "update":
        return <Sparkles size={18} className={styles.iconUpdate} />;
    }
  };

  const getButtonClass = (btnVariant?: "primary" | "secondary" | "danger") => {
    const base = styles.btnBase;
    switch (btnVariant) {
      case "danger":
        return `${base} ${styles.btnDanger}`;
      case "primary":
        return `${base} ${styles.btnPrimary}`;
      default:
        return `${base} ${styles.btnSecondary}`;
    }
  };

  const displayMessage = finalMessage
    ? finalMessage.replace(/\\n/g, "\n").trim()
    : "";

  return createPortal(
    <div className={styles.dialogOverlay}>
      <div className={styles.dialogContainer} data-dialog-container="true">
        <div className={styles.header}>
          <div className={styles.iconWrapper}>{getIcon()}</div>
          <div className={styles.contentWrapper}>
            {finalTitle && <h4 className={styles.title}>{finalTitle}</h4>}

            <textarea
              ref={textAreaRef}
              readOnly
              value={displayMessage}
              rows={1}
              className={`${styles.messageArea} ${
                finalVariant === "error"
                  ? styles.messageError
                  : styles.messageDefault
              }`}
            />
          </div>
        </div>
        <div className={styles.footer}>
          {finalActions.map((action, idx) => (
            <button
              key={idx}
              onClick={(e) => {
                e.stopPropagation();
                action.onClick();
              }}
              disabled={action.disabled}
              className={getButtonClass(action.variant)}
            >
              {action.label}
            </button>
          ))}
        </div>
      </div>
    </div>,
    document.body,
  );
};
