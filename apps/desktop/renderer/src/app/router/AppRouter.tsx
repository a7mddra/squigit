/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect } from "react";
import { usePlatform } from "@/hooks";
import { AppProvider, useAppContext } from "../providers";
import { MainScreen, SplashScreen } from "@/screens";
import { AppRoutes } from "./routes";
import { AppDialogs } from "./dialogs";

import "katex/dist/katex.min.css";

const isOnboardingId = (id: string) => id.startsWith("__system_");

const AppRouterContent: React.FC = () => {
  const app = useAppContext();
  const { isMac } = usePlatform();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.isComposing) return;

      const modPressed = isMac ? e.metaKey : e.ctrlKey;
      if (!modPressed) return;

      const key = e.key.toLowerCase();

      if (!e.shiftKey && !e.altKey && key === "k") {
        e.preventDefault();
        app.openSearchOverlay();
        return;
      }

      if (e.shiftKey && !e.altKey && key === "o") {
        e.preventDefault();
        app.handleNewSession();
      }
    };

    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown, { capture: true });
    };
  }, [app, isMac]);

  const activeId = app.chatHistory.activeSessionId;
  const hasActiveChatSession = !!activeId && !isOnboardingId(activeId);
  const shouldRenderChatShell =
    app.showChatShellDuringNavigation ||
    hasActiveChatSession ||
    !app.isImageMissing;

  if (app.isLoadingState) {
    return <SplashScreen onContextMenu={app.handleContextMenu} />;
  }

  return (
    <MainScreen
      onContextMenu={app.handleContextMenu}
      containerRef={shouldRenderChatShell ? app.containerRef : undefined}
      isSidePanelOpen={app.isSidePanelOpen}
      enablePanelAnimation={app.enablePanelAnimation}
      content={<AppRoutes />}
      dialogs={<AppDialogs />}
    />
  );
};

export const AppRouter: React.FC = () => {
  return (
    <AppProvider>
      <AppRouterContent />
    </AppProvider>
  );
};
