/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { CodeBlock } from "@/components";
import styles from "./MediaTextViewer.module.css";

interface MediaTextViewerProps {
  extension: string;
  textContent: string;
}

export const MediaTextViewer: React.FC<MediaTextViewerProps> = ({
  extension,
  textContent,
}) => {
  return (
    <div className={styles.textViewerWrap}>
      <CodeBlock language={extension || "text"} value={textContent} />
    </div>
  );
};
