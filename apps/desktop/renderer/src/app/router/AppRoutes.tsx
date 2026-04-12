/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import {
  ChatRoute,
  GalleryRoute,
  HomeRoute,
  UpdateNotesRoute,
  WelcomeRoute,
} from "./routes";
import { useAppContext } from "../providers/AppProvider";

const isOnboardingId = (id: string) => id.startsWith("__system_");

interface AppRoutesProps {
  shouldRenderChatShell: boolean;
}

export const AppRoutes: React.FC<AppRoutesProps> = ({
  shouldRenderChatShell,
}) => {
  const app = useAppContext();

  const activeId = app.chatHistory.activeSessionId;

  if (activeId && isOnboardingId(activeId)) {
    if (activeId === "__system_welcome") {
      return (
        <WelcomeRoute
          onSystemAction={app.handleSystemAction}
          onOpenSettings={app.system.openSettings}
        />
      );
    }
    if (activeId.startsWith("__system_update")) {
      return (
        <UpdateNotesRoute
          appName={app.system.appName}
          onSystemAction={app.handleSystemAction}
        />
      );
    }
    if (activeId === "__system_gallery") {
      return (
        <GalleryRoute
          chats={app.chatHistory.chats}
          activeSessionId={app.chatHistory.activeSessionId}
          refreshChats={app.chatHistory.refreshChats}
        />
      );
    }
  }

  if (!shouldRenderChatShell) {
    return (
      <HomeRoute
        appName={app.system.appName}
        onImageReady={app.handleImageReady}
        isGuest={!app.system.activeProfile}
        onLoginRequired={() => app.setShowLoginRequiredDialog(true)}
      />
    );
  }

  return <ChatRoute />;
};
