/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from "react";
import { CodeBlock } from "@/components";

interface UseCodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  textareaRef: React.MutableRefObject<HTMLTextAreaElement | null>;
}

export const useCodeEditor = ({
  value,
  onChange,
  textareaRef,
}: UseCodeEditorProps) => {
  const [isCodeBlockActive, setIsCodeBlockActive] = useState(false);
  const [codeLanguage, setCodeLanguage] = useState("");
  const [originalCodeLanguage, setOriginalCodeLanguage] = useState("");
  const [codeBlockDelimiter, setCodeBlockDelimiter] = useState("```");
  const [codeValue, setCodeValue] = useState("");
  const [consecutiveEnters, setConsecutiveEnters] = useState(0);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const cursorPos = e.target.selectionStart;

    const textBeforeCursor = newValue.slice(0, cursorPos);

    const match = textBeforeCursor.match(/(^|\n)(`{3,})([^\n]*)\n$/);

    if (match && !isCodeBlockActive) {
      const delimiter = match[2];

      const codeBlockCount = newValue.split(delimiter).length - 1;

      if (codeBlockCount % 2 === 1) {
        setIsCodeBlockActive(true);
        const lang = match[3];
        setOriginalCodeLanguage(lang);
        setCodeLanguage(lang || "text");
        setCodeBlockDelimiter(delimiter);

        const beforeBackticks = textBeforeCursor.replace(
          /(^|\n)(`{3,})([^\n]*)\n$/,
          "$1",
        );
        const afterCursor = newValue.slice(cursorPos);
        onChange(beforeBackticks + afterCursor);
      } else {
        onChange(newValue);
      }
    } else {
      onChange(newValue);
    }
  };

  const handleCodeKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      setIsCodeBlockActive(false);

      onChange(`${value}${codeBlockDelimiter}${originalCodeLanguage}\n`);
      setCodeValue("");
      setCodeLanguage("");
      setOriginalCodeLanguage("");
      setCodeBlockDelimiter("```");
      setConsecutiveEnters(0);
      setTimeout(() => {
        const ta = textareaRef.current;
        if (ta) {
          ta.focus();
          const end = ta.value.length;
          ta.setSelectionRange(end, end);
        }
      }, 0);
    } else if (e.key === "Enter") {
      setConsecutiveEnters((prev) => prev + 1);
      if (consecutiveEnters >= 2) {
        setIsCodeBlockActive(false);

        const newPrompt = `${value}\n${codeBlockDelimiter}${codeLanguage}\n${codeValue.trim()}\n${codeBlockDelimiter}\n`;
        onChange(newPrompt);
        setCodeValue("");
        setCodeLanguage("");
        setOriginalCodeLanguage("");
        setCodeBlockDelimiter("```");
        setConsecutiveEnters(0);
        setTimeout(() => textareaRef.current?.focus(), 0);
      }
    } else {
      setConsecutiveEnters(0);
    }
  };

  return {
    isCodeBlockActive,
    codeLanguage,
    codeValue,
    setCodeValue,
    handleChange,
    handleCodeKeyDown,
    setIsCodeBlockActive,
    resetCodeEditor: () => {
      setCodeValue("");
      setIsCodeBlockActive(false);
    },
  };
};

interface InputCodeEditorProps {
  isCodeBlockActive: boolean;
  codeLanguage: string;
  codeValue: string;
  onCodeValueChange: (value: string) => void;
  onCodeKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  isExpanded: boolean;
  codeTaRef: React.MutableRefObject<HTMLTextAreaElement | null>;
}

export const InputCodeEditor: React.FC<InputCodeEditorProps> = ({
  isCodeBlockActive,
  codeLanguage,
  codeValue,
  onCodeValueChange,
  onCodeKeyDown,
  isExpanded,
  codeTaRef,
}) => {
  useEffect(() => {
    if (isCodeBlockActive) {
      codeTaRef.current?.focus();
    }
  }, [isCodeBlockActive, codeTaRef]);

  if (!isCodeBlockActive) return null;

  return (
    <CodeBlock
      ref={codeTaRef}
      language={codeLanguage}
      value={codeValue}
      isEditor={true}
      onChange={onCodeValueChange}
      onKeyDown={onCodeKeyDown}
      placeholder={`Enter ${codeLanguage} code... (3 newlines to exit)`}
      style={{
        height: isExpanded ? "360px" : "auto",
        maxHeight: isExpanded ? "360px" : "240px",
      }}
    />
  );
};
