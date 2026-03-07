/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { ContextMenu, ContextMenuItem } from "@/components";
import { usePlatform } from "@/hooks";

interface AppContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onCopy?: () => void;
  selectedText?: string;
  hasSelection?: boolean;
}

export const AppContextMenu: React.FC<AppContextMenuProps> = ({
  x,
  y,
  onClose,
  onCopy,
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
          onCopy?.();
          onClose();
        }} // Copy requires selection
        disabled={!onCopy || !hasText}
        shortcut={`${mod}C`}
      >
        Copy
      </ContextMenuItem>
    </ContextMenu>
  );
};
