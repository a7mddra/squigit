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
      onKeyDown,
      placeholder,
      stickyHeader,
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
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          style={style}
        />
      );
    }

    return (
      <CodeBlockViewer
        language={language}
        value={value}
        stickyHeader={stickyHeader}
      />
    );
  },
);

CodeBlockComponent.displayName = "CodeBlock";

export const CodeBlock = memo(CodeBlockComponent);
