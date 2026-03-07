/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { invoke } from "@tauri-apps/api/core";
import styles from "./WindowControls.module.css";

export const WindowControls: React.FC = () => {
  const handleClose = () => invoke("close_window");
  const handleMinimize = () => invoke("minimize_window");
  const handleMaximize = () => invoke("maximize_window");

  return (
    <div className={styles.windowsControls}>
      <button
        className={`${styles.windowsButton} ${styles.winMinimize}`}
        onClick={handleMinimize}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.1"
        >
          <line x1="1" y1="6" x2="11" y2="6" />
        </svg>
      </button>
      <button
        className={`${styles.windowsButton} ${styles.winMaximize}`}
        onClick={handleMaximize}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.1"
        >
          <rect x="1" y="1" width="10" height="10" />
        </svg>
      </button>
      <button
        className={`${styles.windowsButton} ${styles.winClose}`}
        onClick={handleClose}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.1"
        >
          <line x1="1" y1="1" x2="11" y2="11" />
          <line x1="11" y1="1" x2="1" y2="11" />
        </svg>
      </button>
    </div>
  );
};
