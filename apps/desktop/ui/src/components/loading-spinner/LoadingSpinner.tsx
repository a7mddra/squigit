/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import styles from "./LoadingSpinner.module.css";

const BLADE_COUNT = 8;

export const LoadingSpinner: React.FC = () => {
  return (
    <div className={styles.spinner}>
      {Array.from({ length: BLADE_COUNT }).map((_, i) => (
        <div
          key={i}
          className={styles.arm}
          style={{ transform: `rotate(${i * (360 / BLADE_COUNT)}deg)` }}
        >
          <div className={styles.blade} />
        </div>
      ))}
    </div>
  );
};
