/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { FolderOpen, Copy, MessageSquare } from "lucide-react";
import { WidgetOverlayIconButton } from "@/components";

interface MediaSidebarProps {
  onReveal: () => void;
  onCopyPath: () => void;
  onRevealInChat?: () => void;
}

export const MediaSidebar: React.FC<MediaSidebarProps> = ({
  onReveal,
  onCopyPath,
  onRevealInChat,
}) => {
  return (
    <>
      {onRevealInChat && (
        <WidgetOverlayIconButton
          icon={<MessageSquare size={22} />}
          label="Reveal in chat"
          onClick={onRevealInChat}
        />
      )}
      <WidgetOverlayIconButton
        icon={<FolderOpen size={22} />}
        label="Reveal in folder"
        onClick={onReveal}
      />
      <WidgetOverlayIconButton
        icon={<Copy size={22} />}
        label="Copy path"
        onClick={onCopyPath}
      />
    </>
  );
};
