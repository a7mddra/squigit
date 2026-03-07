/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { X, Minus, Plus } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import styles from "./TrafficLights.module.css";

export const TrafficLights: React.FC = () => {
  const handleClose = () => invoke("close_window");
  const handleMinimize = () => invoke("minimize_window");
  const handleMaximize = () => invoke("maximize_window");

  return (
    <div className={styles.trafficLights}>
      <button
        className={`${styles.trafficButton} ${styles.close}`}
        onClick={handleClose}
      >
        <X className={styles.icon} />
      </button>
      <button
        className={`${styles.trafficButton} ${styles.minimize}`}
        onClick={handleMinimize}
      >
        <Minus className={styles.icon} />
      </button>
      <button
        className={`${styles.trafficButton} ${styles.maximize}`}
        onClick={handleMaximize}
      >
        <Plus className={styles.icon} />
      </button>
    </div>
  );
};
