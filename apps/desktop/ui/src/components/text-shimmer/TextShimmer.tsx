/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import styles from "./TextShimmer.module.css";

interface TextShimmerProps {
  text: string;
}

export const TextShimmer: React.FC<TextShimmerProps> = ({ text }) => {
  return (
    <div className={styles.container} aria-hidden="true">
      <span className={styles.shimmerText}>{text}</span>
    </div>
  );
};

export default TextShimmer;
