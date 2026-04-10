/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { forwardRef, useMemo, useState } from "react";
import { getIcon } from "material-file-icons";
import styles from "./CitationChip.module.css";

type CitationChipVisual =
  | {
      kind: "favicon";
      src: string;
      alt?: string;
    }
  | {
      kind: "file";
      fileName: string;
    };

export type CitationChipVariant = "site" | "file";

export interface CitationChipProps
  extends Omit<React.AnchorHTMLAttributes<HTMLAnchorElement>, "children"> {
  label: string;
  visual: CitationChipVisual;
  variant?: CitationChipVariant;
  fullWidth?: boolean;
  animate?: boolean;
}

const DEFAULT_FILE_ICON_NAME = "file";

function resolveLocalFileIconMarkup(fileName: string): string {
  const normalizedFileName = fileName.trim() || DEFAULT_FILE_ICON_NAME;
  try {
    return getIcon(normalizedFileName).svg;
  } catch {
    return getIcon(DEFAULT_FILE_ICON_NAME).svg;
  }
}

const LocalFileIcon: React.FC<{ fileName: string }> = ({ fileName }) => {
  const iconMarkup = useMemo(
    () => resolveLocalFileIconMarkup(fileName),
    [fileName],
  );

  return (
    <span
      aria-hidden="true"
      className={styles.fileIcon}
      dangerouslySetInnerHTML={{ __html: iconMarkup }}
    />
  );
};

export const CitationChip = forwardRef<HTMLAnchorElement, CitationChipProps>(
  (
    {
      label,
      visual,
      variant,
      className,
      fullWidth = false,
      animate = false,
      ...anchorProps
    },
    ref,
  ) => {
    const [hideFavicon, setHideFavicon] = useState(false);
    const resolvedVariant = variant || (visual.kind === "file" ? "file" : "site");

    return (
      <a
        ref={ref}
        {...anchorProps}
        className={[
          styles.chip,
          resolvedVariant === "file" ? styles.fileVariant : styles.siteVariant,
          fullWidth ? styles.fullWidth : "",
          animate ? styles.reveal : "",
          className || "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        {visual.kind === "favicon" ? (
          !hideFavicon && (
            <img
              src={visual.src}
              alt={visual.alt || ""}
              className={styles.iconImage}
              onError={() => setHideFavicon(true)}
            />
          )
        ) : (
          <LocalFileIcon fileName={visual.fileName} />
        )}
        <span className={styles.title}>
          <bdi className={styles.titleText}>{label}</bdi>
        </span>
      </a>
    );
  },
);

CitationChip.displayName = "CitationChip";
