/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Welcome, Agreement, UpdateNotes } from "@/features";
import { Gallery } from "@/features/gallery";
import { Chat } from "@/features/chat";
import { useAppContext } from "../providers/AppProvider";

const isOnboardingId = (id: string) => id.startsWith("__system_");

export const AppRoutes: React.FC = () => {
  const app = useAppContext();

  const activeId = app.chatHistory.activeSessionId;
  const hasActiveChatSession = !!activeId && !isOnboardingId(activeId);
  const shouldRenderChatShell =
    app.showChatShellDuringNavigation ||
    hasActiveChatSession ||
    !app.isImageMissing;

  if (activeId && isOnboardingId(activeId)) {
    if (activeId === "__system_welcome") {
      return (
        <Agreement
          onSystemAction={app.handleSystemAction}
          onOpenSettings={app.system.openSettings}
        />
      );
    }
    if (activeId.startsWith("__system_update")) {
      return (
        <UpdateNotes
          appName={app.system.appName}
          onSystemAction={app.handleSystemAction}
        />
      );
    }
    if (activeId === "__system_gallery") {
      return (
        <Gallery
          chats={app.chatHistory.chats}
          activeSessionId={app.chatHistory.activeSessionId}
          refreshChats={app.chatHistory.refreshChats}
        />
      );
    }
  }

  if (!shouldRenderChatShell) {
    return (
      <Welcome
        appName={app.system.appName}
        onImageReady={app.handleImageReady}
        isGuest={!app.system.activeProfile}
        onLoginRequired={() => app.setShowLoginRequiredDialog(true)}
      />
    );
  }

  return <Chat />;
};
