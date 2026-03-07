/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export interface CodeBlockProps {
  /** Programming language for syntax highlighting */
  language: string;
  /** The code content to display */
  value: string;
  /** Enable textarea editing mode */
  isEditor?: boolean;
  /** Callback when content changes (Editor mode only) */
  onChange?: (value: string) => void;
  /** Keyboard event handler for textarea */
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  /** Placeholder text for empty Editor blocks */
  placeholder?: string;
  /** Enable/Disable sticky header (Default: true) */
  stickyHeader?: boolean;
  style?: React.CSSProperties;
}
