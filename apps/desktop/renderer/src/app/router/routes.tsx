/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { Welcome, Agreement, UpdateNotes, Chat, Gallery } from "@/features";
import { useAppContext } from "../providers";

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
      return <Agreement />;
    }
    if (activeId.startsWith("__system_update")) {
      return <UpdateNotes />;
    }
    if (activeId === "__system_gallery") {
      return <Gallery />;
    }
  }

  if (!shouldRenderChatShell) {
    return (
      <Welcome
        onImageReady={app.handleImageReady}
        isGuest={!app.system.activeProfile}
        onLoginRequired={() => app.setShowLoginRequiredDialog(true)}
      />
    );
  }

  return <Chat />;
};
