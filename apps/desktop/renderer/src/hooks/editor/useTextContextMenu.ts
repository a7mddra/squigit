/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useCallback } from "react";

interface ContextMenuState {
  isOpen: boolean;
  x: number;
  y: number;
  hasSelection: boolean;
}

interface UseTextContextMenuProps {
  hasSelection?: boolean;
}

export function useTextContextMenu({
  hasSelection = false,
}: UseTextContextMenuProps = {}) {
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    isOpen: false,
    x: 0,
    y: 0,
    hasSelection: false,
  });

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setContextMenu({
        isOpen: true,
        x: e.clientX,
        y: e.clientY,
        hasSelection,
      });
    },
    [hasSelection],
  );

  const handleClose = useCallback(() => {
    setContextMenu((prev) => ({ ...prev, isOpen: false }));
  }, []);

  return {
    data: contextMenu,
    handleContextMenu,
    handleClose,
    isOpen: contextMenu.isOpen,
    setData: setContextMenu,
  };
}
