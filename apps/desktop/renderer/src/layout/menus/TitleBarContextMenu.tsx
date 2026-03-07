/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check } from "lucide-react";
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from "@/components";
import { usePlatform } from "@/hooks";

interface TitleBarContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onNewChat: () => void;
  onOpenSettings: () => void;
  isAlwaysOnTop: boolean;
  onToggleAlwaysOnTop: () => void;
}

export const TitleBarContextMenu: React.FC<TitleBarContextMenuProps> = ({
  x,
  y,
  onClose,
  onNewChat,
  onOpenSettings,
  isAlwaysOnTop,
  onToggleAlwaysOnTop,
}) => {
  const { isMac } = usePlatform();

  const handleMinimize = () => {
    invoke("minimize_window");
    onClose();
  };

  const handleCloseWindow = () => {
    invoke("close_window");
    onClose();
  };

  const handleAlwaysOnTop = () => {
    onToggleAlwaysOnTop();
    onClose();
  };

  const handleNewChat = () => {
    onNewChat();
    onClose();
  };

  const handleSettings = () => {
    onOpenSettings();
    onClose();
  };

  const handleReload = async () => {
    onClose();
    try {
      await invoke("reload_window");
    } catch {
      window.location.reload();
    }
  };

  return (
    <ContextMenu x={x} y={y} onClose={onClose} width={220}>
      <ContextMenuItem onClick={handleMinimize}>Minimize</ContextMenuItem>

      <ContextMenuItem
        onClick={handleAlwaysOnTop}
        shortcut={isAlwaysOnTop ? <Check size={14} /> : undefined}
      >
        Always on Top
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem onClick={handleNewChat}>New Chat</ContextMenuItem>

      <ContextMenuItem onClick={handleSettings}>Settings</ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem onClick={handleReload}>Reload</ContextMenuItem>

      <ContextMenuItem
        onClick={handleCloseWindow}
        shortcut={isMac ? "Cmd+W" : "Alt+F4"}
      >
        Close
      </ContextMenuItem>
    </ContextMenu>
  );
};
