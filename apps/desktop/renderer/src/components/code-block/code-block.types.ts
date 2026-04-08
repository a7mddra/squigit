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
  /** Callback when the language label is renamed (Editor mode only) */
  onLanguageChange?: (language: string) => void;
  /** Keyboard event handler for textarea */
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  /** Placeholder text for empty Editor blocks */
  placeholder?: string;
  /** Enable/Disable sticky header (Default: true) */
  stickyHeader?: boolean;
  /** Optional editor action button label */
  actionLabel?: string;
  /** Optional editor action button title */
  actionTitle?: string;
  /** Optional editor action button icon */
  actionIcon?: React.ReactNode;
  /** Optional editor action button handler */
  onAction?: () => void;
  /** Disable editor action button */
  actionDisabled?: boolean;
  /** Make the block fill its parent height */
  fillHeight?: boolean;
  style?: React.CSSProperties;
}
