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
import { Pencil, CheckSquare, Trash2 } from "lucide-react";

interface PanelContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onRename: () => void;
  onToggleSelection: () => void;
  onDelete: () => void;
  isSelected: boolean;
}

export const PanelContextMenu: React.FC<PanelContextMenuProps> = ({
  x,
  y,
  onClose,
  onRename,
  onToggleSelection,
  onDelete,
  isSelected: _isSelected,
}) => {
  return (
    <ContextMenu x={x} y={y} onClose={onClose} width={180}>
      <ContextMenuItem
        onClick={() => {
          onRename();
          onClose();
        }}
        icon={<Pencil size={14} />}
      >
        Rename
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem
        onClick={() => {
          onToggleSelection();
          onClose();
        }}
        icon={<CheckSquare size={14} />}
      >
        Select
      </ContextMenuItem>

      <ContextMenuItem
        variant="danger"
        onClick={() => {
          onDelete();
          onClose();
        }}
        icon={<Trash2 size={14} />}
      >
        Delete
      </ContextMenuItem>
    </ContextMenu>
  );
};
