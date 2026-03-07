/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from "@/components";
import { usePlatform } from "@/hooks";

interface TextContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  onCut?: () => void;
  onSelectAll?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  selectedText?: string;
  hasSelection?: boolean;
}

export const TextContextMenu: React.FC<TextContextMenuProps> = ({
  x,
  y,
  onClose,
  onCopy,
  onPaste,
  onCut,
  onSelectAll,
  onUndo,
  onRedo,
  selectedText = "",
  hasSelection = false,
}) => {
  const hasText = hasSelection || selectedText.length > 0;

  const { isMac, modSymbol } = usePlatform();
  const mod = isMac ? modSymbol : `${modSymbol}+`;

  return (
    <ContextMenu x={x} y={y} onClose={onClose} width={200}>
      <ContextMenuItem
        onClick={(e) => {
          e.stopPropagation();
          onUndo?.();
          onClose();
        }}
        disabled={!onUndo}
        shortcut={`${mod}Z`}
      >
        Undo
      </ContextMenuItem>
      <ContextMenuItem
        onClick={(e) => {
          e.stopPropagation();
          onRedo?.();
          onClose();
        }}
        disabled={!onRedo}
        shortcut={isMac ? `${mod}Shift+Z` : `${mod}Y`}
      >
        Redo
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem
        onClick={(e) => {
          e.stopPropagation();
          onCut?.();
          onClose();
        }}
        disabled={!onCut || !hasText}
        shortcut={`${mod}X`}
      >
        Cut
      </ContextMenuItem>
      <ContextMenuItem
        onClick={(e) => {
          e.stopPropagation();
          onCopy?.();
          onClose();
        }}
        disabled={!onCopy || !hasText}
        shortcut={`${mod}C`}
      >
        Copy
      </ContextMenuItem>
      <ContextMenuItem
        onClick={(e) => {
          e.stopPropagation();
          onPaste?.();
          onClose();
        }}
        disabled={!onPaste}
        shortcut={`${mod}V`}
      >
        Paste
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem
        onClick={(e) => {
          e.stopPropagation();
          onSelectAll?.();
          onClose();
        }}
        disabled={!onSelectAll}
        shortcut={`${mod}A`}
      >
        Select All
      </ContextMenuItem>
    </ContextMenu>
  );
};
