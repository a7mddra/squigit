/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import styles from "./TextShimmer.module.css";

interface TextShimmerProps {
  text: string;
  compact?: boolean;
  className?: string;
}

export const TextShimmer: React.FC<TextShimmerProps> = ({
  text,
  compact = false,
  className,
}) => {
  const containerClassName = [
    styles.container,
    compact ? styles.compact : "",
    className || "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={containerClassName} aria-hidden="true">
      <span className={styles.shimmerText}>{text}</span>
    </div>
  );
};

export default TextShimmer;
