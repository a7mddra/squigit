/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  forwardRef,
  useRef,
  useCallback,
  useImperativeHandle,
  useLayoutEffect,
  useEffect,
  useState,
} from "react";
import { Terminal } from "lucide-react";
import { useCodeHighlighter, useTextContextMenu, useKeyDown } from "@/hooks";
import { TextContextMenu } from  "@/app/shell/menus/TextContextMenu";
import styles from "./CodeBlock.shared.module.css";

const INDENT = "  ";

const getLineCount = (text: string) => text.split("\n").length;

const getLineNumberAtPosition = (text: string, position: number) =>
  text.slice(0, position).split("\n").length;

const getLineIndexAtPosition = (text: string, position: number) =>
  getLineNumberAtPosition(text, position) - 1;

const getLineStartOffset = (text: string, position: number) =>
  text.lastIndexOf("\n", Math.max(0, position) - 1) + 1;

const getLineEndOffset = (text: string, position: number) => {
  const lineBreakIndex = text.indexOf("\n", position);
  return lineBreakIndex === -1 ? text.length : lineBreakIndex;
};

const getLineStartOffsets = (lines: string[]) => {
  const offsets: number[] = [];
  let offset = 0;

  for (const line of lines) {
    offsets.push(offset);
    offset += line.length + 1;
  }

  return offsets;
};

const getSelectedLineIndexes = (text: string, start: number, end: number) => {
  const normalizedEnd = end > start && text[end - 1] === "\n" ? end - 1 : end;

  return {
    startLine: getLineIndexAtPosition(text, start),
    endLine: getLineIndexAtPosition(text, normalizedEnd),
  };
};

const escapeRegExp = (value: string) =>
  value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const AUTO_CLOSE_PAIRS: Record<string, string> = {
  "(": ")",
  "[": "]",
  "{": "}",
  '"': '"',
  "'": "'",
  "`": "`",
};

const HASH_COMMENT_LANGUAGES = new Set([
  "bash",
  "conf",
  "config",
  "dockerfile",
  "dotenv",
  "fish",
  "graphql",
  "ini",
  "makefile",
  "perl",
  "pl",
  "powershell",
  "properties",
  "ps1",
  "py",
  "python",
  "r",
  "rb",
  "ruby",
  "shell",
  "sh",
  "toml",
  "yaml",
  "yml",
  "zsh",
]);

const DASH_COMMENT_LANGUAGES = new Set([
  "ada",
  "elm",
  "haskell",
  "hs",
  "lua",
  "sql",
]);

const BLOCK_COMMENT_LANGUAGES = new Set(["css", "less", "sass", "scss"]);

const MARKUP_COMMENT_LANGUAGES = new Set([
  "astro",
  "html",
  "markdown",
  "md",
  "mdx",
  "svg",
  "xml",
]);

const getCommentDelimiters = (language: string) => {
  const normalizedLanguage = language.trim().toLowerCase();

  if (HASH_COMMENT_LANGUAGES.has(normalizedLanguage)) {
    return { line: "#" };
  }

  if (DASH_COMMENT_LANGUAGES.has(normalizedLanguage)) {
    return { line: "--" };
  }

  if (BLOCK_COMMENT_LANGUAGES.has(normalizedLanguage)) {
    return { block: ["/*", "*/"] as const };
  }

  if (MARKUP_COMMENT_LANGUAGES.has(normalizedLanguage)) {
    return { block: ["<!--", "-->"] as const };
  }

  return {
    line: "//",
    block: ["/*", "*/"] as const,
  };
};

interface EditResult {
  value: string;
  selectionStart: number;
  selectionEnd: number;
}

const indentSelection = (
  text: string,
  start: number,
  end: number,
): EditResult => {
  const hasMultilineSelection = start !== end && text.slice(start, end).includes("\n");

  if (!hasMultilineSelection) {
    const nextValue = `${text.slice(0, start)}${INDENT}${text.slice(end)}`;
    const nextPosition = start + INDENT.length;

    return {
      value: nextValue,
      selectionStart: nextPosition,
      selectionEnd: nextPosition,
    };
  }

  const rangeStart = getLineStartOffset(text, start);
  const normalizedEnd = end > start && text[end - 1] === "\n" ? end - 1 : end;
  const rangeEnd = getLineEndOffset(text, normalizedEnd);
  const block = text.slice(rangeStart, rangeEnd);
  const lines = block.split("\n");
  const nextBlock = lines.map((line) => `${INDENT}${line}`).join("\n");

  return {
    value: `${text.slice(0, rangeStart)}${nextBlock}${text.slice(rangeEnd)}`,
    selectionStart: start + INDENT.length,
    selectionEnd: end + INDENT.length * lines.length,
  };
};

const getOutdentCount = (line: string) => {
  if (line.startsWith(INDENT)) return INDENT.length;
  if (line.startsWith("\t")) return 1;

  let count = 0;
  while (count < INDENT.length && line[count] === " ") {
    count += 1;
  }

  return count;
};

const outdentSelection = (
  text: string,
  start: number,
  end: number,
): EditResult | null => {
  const hasMultilineSelection = start !== end && text.slice(start, end).includes("\n");

  if (!hasMultilineSelection) {
    const lineStart = getLineStartOffset(text, start);
    const lineEnd = getLineEndOffset(text, start);
    const line = text.slice(lineStart, lineEnd);
    const removeCount = getOutdentCount(line);

    if (removeCount === 0) {
      return null;
    }

    return {
      value: `${text.slice(0, lineStart)}${line.slice(removeCount)}${text.slice(lineEnd)}`,
      selectionStart: Math.max(lineStart, start - removeCount),
      selectionEnd: Math.max(lineStart, end - removeCount),
    };
  }

  const rangeStart = getLineStartOffset(text, start);
  const normalizedEnd = end > start && text[end - 1] === "\n" ? end - 1 : end;
  const rangeEnd = getLineEndOffset(text, normalizedEnd);
  const block = text.slice(rangeStart, rangeEnd);
  const lines = block.split("\n");
  const removedCounts = lines.map(getOutdentCount);

  if (removedCounts.every((count) => count === 0)) {
    return null;
  }

  const nextBlock = lines
    .map((line, index) => line.slice(removedCounts[index]))
    .join("\n");

  return {
    value: `${text.slice(0, rangeStart)}${nextBlock}${text.slice(rangeEnd)}`,
    selectionStart: Math.max(rangeStart, start - removedCounts[0]),
    selectionEnd: Math.max(
      rangeStart,
      end - removedCounts.reduce((total, count) => total + count, 0),
    ),
  };
};

const toggleLineComments = (
  text: string,
  start: number,
  end: number,
  token: string,
): EditResult => {
  const rangeStart = getLineStartOffset(text, start);
  const normalizedEnd = end > start && text[end - 1] === "\n" ? end - 1 : end;
  const rangeEnd = getLineEndOffset(text, normalizedEnd);
  const block = text.slice(rangeStart, rangeEnd);
  const lines = block.split("\n");
  const tokenPattern = new RegExp(`^(\\s*)${escapeRegExp(token)}(?:\\s)?`);
  const shouldUncomment =
    lines.some((line) => line.trim().length > 0) &&
    lines.every((line) => line.trim().length === 0 || tokenPattern.test(line));

  const nextBlock = lines
    .map((line) => {
      if (line.trim().length === 0) {
        return line;
      }

      if (shouldUncomment) {
        return line.replace(tokenPattern, "$1");
      }

      return line.replace(/^(\s*)/, `$1${token} `);
    })
    .join("\n");

  const nextValue = `${text.slice(0, rangeStart)}${nextBlock}${text.slice(rangeEnd)}`;

  if (start === end) {
    const lineStart = getLineStartOffset(text, start);
    const line = text.slice(lineStart, getLineEndOffset(text, start));
    const indentationLength = line.match(/^\s*/)?.[0].length ?? 0;
    const tokenLength = token.length + 1;
    let nextPosition = start;

    if (shouldUncomment) {
      if (start > lineStart + indentationLength) {
        const uncommentedLength = line.slice(indentationLength).startsWith(`${token} `)
          ? tokenLength
          : line.slice(indentationLength).startsWith(token)
            ? token.length
            : 0;

        nextPosition = Math.max(
          lineStart + indentationLength,
          start - uncommentedLength,
        );
      }
    } else if (start >= lineStart + indentationLength) {
      nextPosition = start + tokenLength;
    }

    return {
      value: nextValue,
      selectionStart: nextPosition,
      selectionEnd: nextPosition,
    };
  }

  return {
    value: nextValue,
    selectionStart: rangeStart,
    selectionEnd: rangeStart + nextBlock.length,
  };
};

const toggleBlockComments = (
  text: string,
  start: number,
  end: number,
  [open, close]: readonly [string, string],
): EditResult => {
  const targetStart = start === end ? getLineStartOffset(text, start) : start;
  const targetEnd = start === end ? getLineEndOffset(text, start) : end;
  const target = text.slice(targetStart, targetEnd);
  const leadingWhitespace = target.match(/^\s*/)?.[0] ?? "";
  const trailingWhitespace = target.match(/\s*$/)?.[0] ?? "";
  const core = target.slice(
    leadingWhitespace.length,
    target.length - trailingWhitespace.length,
  );
  const shouldUncomment = core.startsWith(open) && core.endsWith(close);

  const nextCore = shouldUncomment
    ? (() => {
        let inner = core.slice(open.length, core.length - close.length);

        if (inner.startsWith(" ")) {
          inner = inner.slice(1);
        }
        if (inner.endsWith(" ")) {
          inner = inner.slice(0, -1);
        }

        return inner;
      })()
    : core.length > 0
      ? `${open} ${core} ${close}`
      : `${open}${close}`;

  const nextTarget = `${leadingWhitespace}${nextCore}${trailingWhitespace}`;
  const nextValue = `${text.slice(0, targetStart)}${nextTarget}${text.slice(targetEnd)}`;

  if (start === end) {
    const nextPosition = shouldUncomment
      ? Math.max(targetStart + leadingWhitespace.length, start - open.length - 1)
      : start + open.length + (core.length > 0 ? 1 : 0);

    return {
      value: nextValue,
      selectionStart: nextPosition,
      selectionEnd: nextPosition,
    };
  }

  return {
    value: nextValue,
    selectionStart: targetStart,
    selectionEnd: targetStart + nextTarget.length,
  };
};

const moveSelectedLines = (
  text: string,
  start: number,
  end: number,
  direction: -1 | 1,
): EditResult | null => {
  const lines = text.split("\n");
  const lineStarts = getLineStartOffsets(lines);
  const { startLine, endLine } = getSelectedLineIndexes(text, start, end);
  const isCollapsed = start === end;
  const lastMovableLineIndex = text.endsWith("\n") ? lines.length - 2 : lines.length - 1;

  if (direction === -1) {
    if (startLine === 0) {
      return null;
    }

    const movingLines = lines.slice(startLine, endLine + 1);
    const nextLines = [
      ...lines.slice(0, startLine - 1),
      ...movingLines,
      lines[startLine - 1],
      ...lines.slice(endLine + 1),
    ];
    const nextLineStarts = getLineStartOffsets(nextLines);
    const nextStartLine = startLine - 1;
    const nextEndLine = endLine - 1;
    const nextValue = nextLines.join("\n");

    if (isCollapsed) {
      const column = start - lineStarts[startLine];
      const nextPosition =
        nextLineStarts[nextStartLine] +
        Math.min(column, nextLines[nextStartLine].length);

      return {
        value: nextValue,
        selectionStart: nextPosition,
        selectionEnd: nextPosition,
      };
    }

    return {
      value: nextValue,
      selectionStart: nextLineStarts[nextStartLine],
      selectionEnd: nextLineStarts[nextEndLine] + nextLines[nextEndLine].length,
    };
  }

  if (endLine >= lastMovableLineIndex) {
    return null;
  }

  const movingLines = lines.slice(startLine, endLine + 1);
  const nextLines = [
    ...lines.slice(0, startLine),
    lines[endLine + 1],
    ...movingLines,
    ...lines.slice(endLine + 2),
  ];
  const nextLineStarts = getLineStartOffsets(nextLines);
  const nextStartLine = startLine + 1;
  const nextEndLine = endLine + 1;
  const nextValue = nextLines.join("\n");

  if (isCollapsed) {
    const column = start - lineStarts[startLine];
    const nextPosition =
      nextLineStarts[nextStartLine] + Math.min(column, nextLines[nextStartLine].length);

    return {
      value: nextValue,
      selectionStart: nextPosition,
      selectionEnd: nextPosition,
    };
  }

  return {
    value: nextValue,
    selectionStart: nextLineStarts[nextStartLine],
    selectionEnd: nextLineStarts[nextEndLine] + nextLines[nextEndLine].length,
  };
};

export interface CodeBlockEditorProps {
  language: string;
  value: string;
  onChange?: (value: string) => void;
  onLanguageChange?: (language: string) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  placeholder?: string;
  actionLabel?: string;
  actionTitle?: string;
  actionIcon?: React.ReactNode;
  onAction?: () => void;
  actionDisabled?: boolean;
  fillHeight?: boolean;
  style?: React.CSSProperties;
}

export const CodeBlockEditor = forwardRef<
  HTMLTextAreaElement,
  CodeBlockEditorProps
>(
  (
    {
      language,
      value,
      onChange,
      onLanguageChange,
      onKeyDown,
      placeholder,
      actionLabel,
      actionTitle,
      actionIcon,
      onAction,
      actionDisabled = false,
      fillHeight = false,
      style,
    },
    ref,
  ) => {
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const highlightRef = useRef<HTMLElement | null>(null);
    const gutterContentRef = useRef<HTMLDivElement>(null);
    const languageInputRef = useRef<HTMLInputElement>(null);
    const isCancellingRenameRef = useRef(false);
    const {
      data: contextMenuData,
      handleContextMenu,
      handleClose: handleCloseContextMenu,
    } = useTextContextMenu();
    const displayLanguage = language?.trim() || "text";
    const [isRenamingLanguage, setIsRenamingLanguage] = useState(false);
    const [renameValue, setRenameValue] = useState(displayLanguage);
    const [activeLine, setActiveLine] = useState(1);
    const lineCount = getLineCount(value);
    const lineNumbers = Array.from({ length: lineCount }, (_, index) => index + 1);
    const lineNumberDigits = Math.max(String(lineCount).length, 2);
    const editorStyle = {
      ...style,
      "--cb-editor-gutter-digits": lineNumberDigits,
    } as React.CSSProperties;

    // Expose the textarea ref to parent
    useImperativeHandle(ref, () => textareaRef.current as HTMLTextAreaElement);

    // Only highlight if a valid language is provided (not "text")
    const shouldHighlight = Boolean(
      displayLanguage && displayLanguage.toLowerCase() !== "text",
    );
    const { highlightedHtml } = useCodeHighlighter(
      shouldHighlight ? value : "",
      displayLanguage,
    );

    const handleBeginRename = useCallback(() => {
      if (!onLanguageChange || isRenamingLanguage) return;
      isCancellingRenameRef.current = false;
      setIsRenamingLanguage(true);
    }, [isRenamingLanguage, onLanguageChange]);

    useEffect(() => {
      if (isRenamingLanguage && languageInputRef.current) {
        const input = languageInputRef.current;
        const caretPosition = input.value.length;
        input.focus();
        input.setSelectionRange(caretPosition, caretPosition);
      }
    }, [isRenamingLanguage]);

    useEffect(() => {
      if (!isRenamingLanguage) {
        setRenameValue(displayLanguage);
      }
    }, [displayLanguage, isRenamingLanguage]);

    const syncGutterScroll = useCallback(() => {
      if (!textareaRef.current || !gutterContentRef.current) return;
      gutterContentRef.current.style.transform = `translateY(-${textareaRef.current.scrollTop}px)`;
    }, []);

    const updateActiveLine = useCallback(() => {
      const textarea = textareaRef.current;
      if (!textarea) return;

      const nextActiveLine = getLineNumberAtPosition(
        textarea.value,
        textarea.selectionStart,
      );

      setActiveLine((currentLine) =>
        currentLine === nextActiveLine ? currentLine : nextActiveLine,
      );
    }, []);

    // Sync scroll between textarea, highlight overlay, and line number gutter.
    const handleScroll = useCallback(() => {
      if (textareaRef.current && highlightRef.current) {
        highlightRef.current.scrollTop = textareaRef.current.scrollTop;
        highlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
      }

      syncGutterScroll();
    }, [syncGutterScroll]);

    // Force sync on render updates (e.g. when highlighting loads or content changes)
    useLayoutEffect(() => {
      handleScroll();
    }, [handleScroll, highlightedHtml, value]);

    useEffect(() => {
      updateActiveLine();
    }, [updateActiveLine, value]);

    useEffect(() => {
      const handleSelectionChange = () => {
        const textarea = textareaRef.current;
        if (!textarea || document.activeElement !== textarea) return;
        updateActiveLine();
      };

      document.addEventListener("selectionchange", handleSelectionChange);
      return () =>
        document.removeEventListener("selectionchange", handleSelectionChange);
    }, [updateActiveLine]);

    // Ensure trailing newline so cursor position matches
    const displayValue = value.endsWith("\n") ? value + " " : value;

    const handleRenameSubmit = useCallback(() => {
      isCancellingRenameRef.current = false;
      const nextLanguage = renameValue.trim() || "text";
      if (nextLanguage !== displayLanguage) {
        onLanguageChange?.(nextLanguage);
      }
      setRenameValue(nextLanguage);
      setIsRenamingLanguage(false);
    }, [displayLanguage, onLanguageChange, renameValue]);

    const handleRenameCancel = useCallback(() => {
      isCancellingRenameRef.current = true;
      setRenameValue(displayLanguage);
      setIsRenamingLanguage(false);
    }, [displayLanguage]);

    const handleRenameBlur = useCallback(() => {
      if (isCancellingRenameRef.current) {
        isCancellingRenameRef.current = false;
        setRenameValue(displayLanguage);
        setIsRenamingLanguage(false);
        return;
      }

      handleRenameSubmit();
    }, [displayLanguage, handleRenameSubmit]);

    const handleRenameKeyDown = useKeyDown(
      {
        Enter: handleRenameSubmit,
        Escape: handleRenameCancel,
      },
      { stopPropagation: true },
    );

    const handleCopy = useCallback(() => {
      const textarea = textareaRef.current;
      if (!textarea) return;

      if (textarea.selectionStart !== textarea.selectionEnd) {
        const selectedText = textarea.value.substring(
          textarea.selectionStart,
          textarea.selectionEnd,
        );
        navigator.clipboard.writeText(selectedText);
      }
    }, []);

    const handleCut = useCallback(() => {
      const textarea = textareaRef.current;
      if (!textarea || !onChange) return;

      if (textarea.selectionStart !== textarea.selectionEnd) {
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const selectedText = textarea.value.substring(start, end);
        navigator.clipboard.writeText(selectedText);

        const newValue =
          textarea.value.substring(0, start) + textarea.value.substring(end);
        onChange(newValue);

        setTimeout(() => {
          textarea.selectionStart = textarea.selectionEnd = start;
          updateActiveLine();
        }, 0);
      }
    }, [onChange, updateActiveLine, value]);

    const handlePaste = useCallback(async () => {
      const textarea = textareaRef.current;
      if (!textarea || !onChange) return;

      try {
        const text = await navigator.clipboard.readText();
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const newValue = value.substring(0, start) + text + value.substring(end);
        onChange(newValue);

        setTimeout(() => {
          textarea.selectionStart = textarea.selectionEnd = start + text.length;
          updateActiveLine();
        }, 0);
      } catch (err) {
        console.error("Failed to read clipboard:", err);
      }
    }, [onChange, updateActiveLine, value]);

    const handleSelectAll = useCallback(() => {
      textareaRef.current?.select();
    }, []);

    const setEditorSelection = useCallback(
      (selectionStart: number, selectionEnd: number = selectionStart) => {
        window.setTimeout(() => {
          const textarea = textareaRef.current;
          if (!textarea) return;

          textarea.focus();
          textarea.setSelectionRange(selectionStart, selectionEnd);
          updateActiveLine();
        }, 0);
      },
      [updateActiveLine],
    );

    const applyEdit = useCallback(
      (
        nextValue: string,
        selectionStart: number,
        selectionEnd: number = selectionStart,
      ) => {
        if (!onChange) {
          setEditorSelection(selectionStart, selectionEnd);
          return;
        }

        if (nextValue !== value) {
          onChange(nextValue);
        }

        setEditorSelection(selectionStart, selectionEnd);
      },
      [onChange, setEditorSelection, value],
    );

    const handleEditorKeyDown = useCallback(
      (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
        const textarea = textareaRef.current;
        if (!textarea) return;

        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const hasSelection = start !== end;

        if (e.altKey && !e.ctrlKey && !e.metaKey) {
          if (e.key === "ArrowUp" || e.key === "ArrowDown") {
            e.preventDefault();

            const nextState = moveSelectedLines(
              value,
              start,
              end,
              e.key === "ArrowUp" ? -1 : 1,
            );

            if (nextState) {
              applyEdit(
                nextState.value,
                nextState.selectionStart,
                nextState.selectionEnd,
              );
            }
          }

          return;
        }

        if ((e.ctrlKey || e.metaKey) && !e.altKey && e.code === "Slash") {
          e.preventDefault();

          const commentDelimiters = getCommentDelimiters(displayLanguage);
          const nextState = commentDelimiters.line
            ? toggleLineComments(value, start, end, commentDelimiters.line)
            : commentDelimiters.block
              ? toggleBlockComments(value, start, end, commentDelimiters.block)
              : null;

          if (nextState) {
            applyEdit(
              nextState.value,
              nextState.selectionStart,
              nextState.selectionEnd,
            );
          }

          return;
        }

        if (e.ctrlKey || e.metaKey || e.altKey || !onChange) {
          return;
        }

        if (e.key === "Tab") {
          e.preventDefault();

          const nextState = e.shiftKey
            ? outdentSelection(value, start, end)
            : indentSelection(value, start, end);

          if (nextState) {
            applyEdit(
              nextState.value,
              nextState.selectionStart,
              nextState.selectionEnd,
            );
          }

          return;
        }

        if (e.key === "Backspace" && !hasSelection) {
          const previousCharacter = value[start - 1];
          const nextCharacter = value[start];

          if (
            previousCharacter &&
            AUTO_CLOSE_PAIRS[previousCharacter] === nextCharacter
          ) {
            e.preventDefault();
            applyEdit(
              `${value.slice(0, start - 1)}${value.slice(start + 1)}`,
              start - 1,
            );
          }

          return;
        }

        if (
          Object.prototype.hasOwnProperty.call(AUTO_CLOSE_PAIRS, e.key)
        ) {
          const closingCharacter = AUTO_CLOSE_PAIRS[e.key];

          if (!hasSelection && value[end] === e.key) {
            e.preventDefault();
            applyEdit(value, end + 1);
            return;
          }

          e.preventDefault();

          if (hasSelection) {
            applyEdit(
              `${value.slice(0, start)}${e.key}${value.slice(start, end)}${closingCharacter}${value.slice(end)}`,
              start + 1,
              end + 1,
            );
            return;
          }

          applyEdit(
            `${value.slice(0, start)}${e.key}${closingCharacter}${value.slice(end)}`,
            start + 1,
          );
          return;
        }

        if (
          !hasSelection &&
          [")", "]", "}"].includes(e.key) &&
          value[end] === e.key
        ) {
          e.preventDefault();
          applyEdit(value, end + 1);
        }
      },
      [applyEdit, displayLanguage, onChange, value],
    );

    return (
      <div
        className={`${styles.wrapper} ${fillHeight ? styles.fillHeight : ""}`}
        role="region"
        aria-label="Editor code block"
      >
        <div
          className={`${styles.header} ${
            actionLabel && onAction ? styles.headerWithAction : ""
          }`}
        >
          <div
            className={`${styles.langLabel} ${
              onLanguageChange ? styles.headerEditableZone : ""
            }`}
            onClick={
              onLanguageChange
                ? (e) => {
                    e.stopPropagation();
                    handleBeginRename();
                  }
                : undefined
            }
          >
            <Terminal size={14} />
            {isRenamingLanguage ? (
              <input
                ref={languageInputRef}
                className={styles.langInput}
                value={renameValue}
                onChange={(e) => setRenameValue(e.target.value)}
                onBlur={handleRenameBlur}
                onKeyDown={handleRenameKeyDown}
                onClick={(e) => e.stopPropagation()}
                spellCheck={false}
                aria-label="Code language"
              />
            ) : (
              <span
                className={`${styles.langName} ${
                  onLanguageChange ? styles.renameableLangName : ""
                }`}
              >
                {displayLanguage}
              </span>
            )}
          </div>
        </div>
        {actionLabel && onAction && (
          <button
            type="button"
            onClick={onAction}
            className={`${styles.copyButton} ${styles.stickyButton}`}
            title={actionTitle || actionLabel}
            aria-label={actionTitle || actionLabel}
            disabled={actionDisabled}
          >
            {actionIcon && (
              <span className={styles.iconWrapper}>{actionIcon}</span>
            )}
            <span>{actionLabel}</span>
          </button>
        )}
        <div className={styles.EditorContainer} style={editorStyle}>
          <div className={styles.gutter} aria-hidden="true">
            <div ref={gutterContentRef} className={styles.gutterContent}>
              {lineNumbers.map((lineNumber) => (
                <div
                  key={lineNumber}
                  className={`${styles.gutterLine} ${
                    lineNumber === activeLine ? styles.gutterLineActive : ""
                  }`}
                >
                  {lineNumber}
                </div>
              ))}
            </div>
          </div>
          <div className={styles.editorCanvas}>
            {/* Sizer to drive auto-height since other layers are absolute */}
            <div className={styles.sizer} aria-hidden="true">
              {displayValue || placeholder}
            </div>

            {/* Highlighted code layer (behind) */}
            {shouldHighlight && highlightedHtml ? (
              <div
                ref={(element) => {
                  highlightRef.current = element;
                }}
                className={`${styles.highlightLayer} shiki-dual-theme`}
                dangerouslySetInnerHTML={{ __html: highlightedHtml }}
                aria-hidden="true"
              />
            ) : (
              <pre
                ref={(element) => {
                  highlightRef.current = element;
                }}
                className={styles.highlightLayer}
                aria-hidden="true"
              >
                {displayValue || placeholder}
              </pre>
            )}
            {/* Transparent textarea (on top) */}
            <textarea
              ref={textareaRef}
              className={styles.textareaOverlay}
              value={value}
              wrap="off"
              onChange={(e) => onChange?.(e.target.value)}
              onKeyDown={(e) => {
                handleEditorKeyDown(e);
                onKeyDown?.(e);
              }}
              onScroll={handleScroll}
              onSelect={updateActiveLine}
              onFocus={updateActiveLine}
              placeholder={placeholder}
              spellCheck={false}
              aria-label="code editor"
              onContextMenu={handleContextMenu}
            />
            {contextMenuData.isOpen && (
              <TextContextMenu
                x={contextMenuData.x}
                y={contextMenuData.y}
                onClose={handleCloseContextMenu}
                onCopy={handleCopy}
                onCut={handleCut}
                onPaste={handlePaste}
                onSelectAll={handleSelectAll}
                hasSelection={
                  textareaRef.current
                    ? textareaRef.current.selectionStart !==
                      textareaRef.current.selectionEnd
                    : false
                }
              />
            )}
          </div>
        </div>
      </div>
    );
  },
);

CodeBlockEditor.displayName = "CodeBlockEditor";
