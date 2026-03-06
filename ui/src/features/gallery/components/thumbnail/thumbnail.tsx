/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import styles from "./thumbnail.module.css";

interface ThumbnailProps {
  imagePath: string;
  title: string;
  onClick: () => void;
}

export const Thumbnail: React.FC<ThumbnailProps> = ({
  imagePath,
  title,
  onClick,
}) => {
  const src = useMemo(() => convertFileSrc(imagePath), [imagePath]);

  return (
    <button className={styles.card} onClick={onClick} title={title}>
      <img src={src} alt={title} className={styles.preview} loading="lazy" />
      <div className={styles.meta}>
        <span className={styles.title}>{title}</span>
      </div>
    </button>
  );
};
