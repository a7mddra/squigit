/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { memo, forwardRef } from "react";
import { CodeBlockEditor } from "./variants/CodeBlockEditor";
import { CodeBlockViewer } from "./variants/CodeBlockViewer";
import type { CodeBlockProps } from "./code-block.types";

const CodeBlockComponent = forwardRef<HTMLTextAreaElement, CodeBlockProps>(
  (
    {
      language,
      value,
      isEditor = false,
      onChange,
      onLanguageChange,
      onKeyDown,
      placeholder,
      stickyHeader,
      actionLabel,
      actionTitle,
      actionIcon,
      onAction,
      actionDisabled,
      fillHeight,
      style,
    },
    ref,
  ) => {
    if (isEditor) {
      return (
        <CodeBlockEditor
          ref={ref}
          language={language}
          value={value}
          onChange={onChange}
          onLanguageChange={onLanguageChange}
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          actionLabel={actionLabel}
          actionTitle={actionTitle}
          actionIcon={actionIcon}
          onAction={onAction}
          actionDisabled={actionDisabled}
          fillHeight={fillHeight}
          style={style}
        />
      );
    }

    return (
      <CodeBlockViewer
        language={language}
        value={value}
        stickyHeader={stickyHeader}
        fillHeight={fillHeight}
      />
    );
  },
);

CodeBlockComponent.displayName = "CodeBlock";

export const CodeBlock = memo(CodeBlockComponent);
