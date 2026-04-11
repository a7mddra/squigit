/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";

export const useAppContextMenu = () => {
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    selectedText: string;
  } | null>(null);

  const handleContextMenu = (e: React.MouseEvent | any) => {
    const target = e.target as HTMLElement;
    const isInput =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement;

    if (
      isInput &&
      !(target as HTMLInputElement | HTMLTextAreaElement).readOnly
    ) {
      return;
    }

    e.preventDefault();

    let selectedText = "";
    if (isInput) {
      const input = target as HTMLInputElement | HTMLTextAreaElement;
      selectedText = input.value.substring(
        input.selectionStart || 0,
        input.selectionEnd || 0,
      );
    } else {
      selectedText = window.getSelection()?.toString() || "";
    }

    if (selectedText) {
      setContextMenu({ x: e.clientX, y: e.clientY, selectedText });
    }
  };

  const handleCloseContextMenu = () => setContextMenu(null);

  const handleCopy = () => {
    if (contextMenu?.selectedText) {
      navigator.clipboard.writeText(contextMenu.selectedText);
    }
  };

  useEffect(() => {
    const handleClick = () => {
      if (contextMenu) handleCloseContextMenu();
    };
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, [contextMenu]);

  return {
    contextMenu,
    handleContextMenu,
    handleCloseContextMenu,
    handleCopy,
  };
};
