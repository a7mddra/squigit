/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Check, Copy } from "lucide-react";
import { useCodeHighlighter, useCopyToClipboard } from "@/hooks";
import styles from "./CodeBlock.shared.module.css";

interface CodeBlockViewerProps {
  language: string;
  value: string;
  stickyHeader?: boolean;
  fillHeight?: boolean;
  hideCodeContent?: boolean;
  hiddenCodeLineCount?: number;
  onRevealCodeContent?: () => void;
}

export const CodeBlockViewer: React.FC<CodeBlockViewerProps> = ({
  language,
  value,
  stickyHeader = true,
  fillHeight = false,
  hideCodeContent = false,
  hiddenCodeLineCount,
  onRevealCodeContent,
}) => {
  const PLAIN_LANGUAGES = ["text", "txt", "plain", "plaintext", "prompt"];
  const isPlain = !language || PLAIN_LANGUAGES.includes(language.toLowerCase());

  const { isCopied, copy } = useCopyToClipboard(1000);

  const { highlightedHtml } = useCodeHighlighter(
    value,
    language,
    !hideCodeContent && !isPlain,
  );
  const handleCopy = () => copy(value);

  const lineCount = Math.max(1, value.split("\n").length);
  const hiddenLines = Math.max(1, hiddenCodeLineCount ?? lineCount);
  const hiddenLineLabel = hiddenLines === 1 ? "line" : "lines";
  const shouldShowPlain = isPlain || !highlightedHtml;

  return (
    <div
      className={`${styles.wrapper} ${fillHeight ? styles.fillHeight : ""}`}
      role="region"
      aria-label="code block"
    >
      <div className={styles.header}>
        <div className={styles.langLabel}>
          <span className={styles.langName}>{language || "text"}</span>
        </div>
      </div>

      <button
        onClick={handleCopy}
        className={`${styles.copyButton} ${stickyHeader ? styles.stickyButton : ""}`}
        title="Copy code"
        aria-label="Copy code"
        data-copied={isCopied ? "true" : "false"}
      >
        <span className={styles.iconWrapper}>
          {isCopied ? <Check size={14} /> : <Copy size={14} />}
        </span>
        <span>Copy</span>
      </button>

      <div className={styles.content}>
        {hideCodeContent ? (
          <button
            type="button"
            className={styles.hiddenCodeContentButton}
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onRevealCodeContent?.();
            }}
          >
            <em>{`show ${hiddenLines} hidden ${hiddenLineLabel}`}</em>
          </button>
        ) : shouldShowPlain ? (
          <pre className={styles.pre}>{value}</pre>
        ) : (
          <div
            className={`${styles.shikiContainer} shiki-dual-theme`}
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
          />
        )}
      </div>
    </div>
  );
};

CodeBlockViewer.displayName = "CodeBlockViewer";
