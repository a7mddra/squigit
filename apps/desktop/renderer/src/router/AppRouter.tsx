/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect } from "react";
import { getDialogs } from "@/lib";
import { Dialog } from "@/components";
import { usePlatform } from "@/hooks";
import {
  Welcome,
  Agreement,
  UpdateNotes,
  Chat,
  GalleryScreen,
  MediaOverlay,
  SearchOverlay,
} from "@/features";
import { AppProvider, useAppContext } from "@/providers/AppProvider";
import { AppContextMenu } from "@/layout";
import { MainScreen, SplashScreen } from "@/screens";

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

  const isAgreed = app.system.hasAgreed || app.agreedToTerms;
  const baseLoginDialog = getDialogs(app.system.appName).LOGIN_REQUIRED;
  const loginRequiredDialog = {
    ...baseLoginDialog,
    actions: baseLoginDialog.actions.map((action) => ({
      ...action,
      disabled: action.actionKey === "confirm" ? !isAgreed : action.disabled,
    })),
  };

  const renderMainContent = () => {
    const activeId = app.chatHistory.activeSessionId;

    if (activeId && isOnboardingId(activeId)) {
      if (activeId === "__system_welcome") {
        return <Agreement />;
      }
      if (activeId.startsWith("__system_update")) {
        return <UpdateNotes />;
      }
      if (activeId === "__system_gallery") {
        return <GalleryScreen />;
      }
    }

    if (app.isImageMissing) {
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

  const appDialogs = (
    <>
      {app.contextMenu && (
        <AppContextMenu
          x={app.contextMenu.x}
          y={app.contextMenu.y}
          onClose={app.handleCloseContextMenu}
          onCopy={app.handleCopy}
          selectedText={app.contextMenu.selectedText}
          hasSelection={true}
        />
      )}

      <Dialog
        isOpen={app.showGeminiAuthDialog}
        type="GEMINI_AUTH"
        onAction={(key) => {
          let msg = "";
          if (key === "confirm") {
            app.system.openSettings("apikeys");
          } else {
            msg = "Please configure your Gemini API key to continue.";
          }
          app.chat.appendErrorMessage(msg, app.chatHistory.activeSessionId);
          app.setShowGeminiAuthDialog(false);
        }}
      />

      <Dialog
        isOpen={app.system.showExistingProfileDialog}
        type="EXISTING_PROFILE"
        onAction={() => app.system.setShowExistingProfileDialog(false)}
      />

      <Dialog
        isOpen={app.showLoginRequiredDialog}
        type={loginRequiredDialog}
        onAction={(key) => {
          if (key === "confirm") {
            app.system.addAccount();
          }
          app.setShowLoginRequiredDialog(false);
        }}
      />

      <Dialog
        isOpen={app.showCaptureDeniedDialog}
        type="CAPTURE_PERMISSION_DENIED"
        onAction={() => app.setShowCaptureDeniedDialog(false)}
      />

      <Dialog
        isOpen={!!app.busyDialog}
        type={app.busyDialog || undefined}
        appName={app.system.appName}
        onAction={app.handleBusyDialogAction}
      />

      <MediaOverlay
        isOpen={app.mediaViewer.isOpen}
        onClose={app.closeMediaViewer}
        item={app.mediaViewer.item}
        onRevealInChat={(chatId) => {
          app.closeMediaViewer();
          app.handleSelectChat(chatId);
        }}
      />

      <SearchOverlay
        isOpen={app.searchOverlay.isOpen}
        onClose={app.closeSearchOverlay}
      />
    </>
  );

  if (app.isLoadingState) {
    return <SplashScreen onContextMenu={app.handleContextMenu} />;
  }

  return (
    <MainScreen
      onContextMenu={app.handleContextMenu}
      containerRef={app.isImageMissing ? undefined : app.containerRef}
      isSidePanelOpen={app.isSidePanelOpen}
      enablePanelAnimation={app.enablePanelAnimation}
      content={renderMainContent()}
      dialogs={appDialogs}
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
