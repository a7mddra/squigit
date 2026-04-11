/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Attachment } from "@/core";
import styles from "./ImageCollage.module.css";

interface ImageCollageProps {
  images: Attachment[];
  onImageClick: (attachment: Attachment, index: number) => void;
  className?: string;
}

const joinClasses = (...values: Array<string | undefined>) =>
  values.filter(Boolean).join(" ");

function resolveImageSrc(path: string): string {
  return /^(?:https?:\/\/|data:)/iu.test(path) ? path : convertFileSrc(path);
}

interface CollageTileProps {
  image: Attachment;
  index: number;
  onImageClick: (attachment: Attachment, index: number) => void;
  className?: string;
  overlayText?: string;
  imageClassName?: string;
}

const CollageTile: React.FC<CollageTileProps> = ({
  image,
  index,
  onImageClick,
  className,
  overlayText,
  imageClassName,
}) => (
  <button
    type="button"
    className={joinClasses(styles.tileButton, className)}
    onClick={() => onImageClick(image, index)}
    aria-label={`Open image ${index + 1}`}
  >
    <img
      src={resolveImageSrc(image.path)}
      alt={image.name}
      className={joinClasses(styles.image, imageClassName)}
      draggable={false}
    />
    {overlayText && <span className={styles.moreOverlay}>{overlayText}</span>}
  </button>
);

export const ImageCollage: React.FC<ImageCollageProps> = ({
  images,
  onImageClick,
  className,
}) => {
  const imageItems = useMemo(
    () => images.filter((attachment) => attachment.type === "image"),
    [images],
  );

  if (imageItems.length === 0) return null;

  if (imageItems.length === 1) {
    return (
      <div className={joinClasses(styles.root, className)}>
        <CollageTile
          image={imageItems[0]}
          index={0}
          onImageClick={onImageClick}
          className={styles.singleTile}
          imageClassName={styles.singleImage}
        />
      </div>
    );
  }

  if (imageItems.length === 2) {
    return (
      <div className={joinClasses(styles.root, styles.twoGrid, className)}>
        <CollageTile
          image={imageItems[0]}
          index={0}
          onImageClick={onImageClick}
          className={styles.leftTile}
        />
        <CollageTile
          image={imageItems[1]}
          index={1}
          onImageClick={onImageClick}
          className={styles.rightTile}
        />
      </div>
    );
  }

  const hiddenCount = Math.max(0, imageItems.length - 3);

  return (
    <div className={joinClasses(styles.root, styles.threeGrid, className)}>
      <CollageTile
        image={imageItems[0]}
        index={0}
        onImageClick={onImageClick}
        className={joinClasses(styles.largeTile, styles.leftTile)}
      />
      <CollageTile
        image={imageItems[1]}
        index={1}
        onImageClick={onImageClick}
        className={styles.rightTopTile}
      />
      <CollageTile
        image={imageItems[2]}
        index={2}
        onImageClick={onImageClick}
        className={styles.rightBottomTile}
        overlayText={hiddenCount > 0 ? `+${hiddenCount}` : undefined}
      />
    </div>
  );
};
