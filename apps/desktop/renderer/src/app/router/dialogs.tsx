/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { getDialogs, getUpdateAvailableDialog } from "@/core";
import { Dialog } from "@/components/ui";
import { MediaOverlay, SearchOverlay } from "@/features";
import { AppContextMenu } from "@/layout";
import { useAppContext } from "../providers";

export const AppDialogs: React.FC = () => {
  const app = useAppContext();

  const isAgreed = app.system.hasAgreed || app.agreedToTerms;
  const baseLoginDialog = getDialogs(app.system.appName).LOGIN_REQUIRED;
  const loginRequiredDialog = {
    ...baseLoginDialog,
    actions: baseLoginDialog.actions.map((action) => ({
      ...action,
      disabled: action.actionKey === "confirm" ? !isAgreed : action.disabled,
    })),
  };

  return (
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
        isOpen={app.showProviderAuthDialog}
        type="PROVIDER_AUTH"
        onAction={(key) => {
          let msg = "";
          if (key === "confirm") {
            app.system.openSettings("apikeys");
          } else {
            msg = "Please configure your AI provider API key to continue.";
          }
          app.chat.appendErrorMessage(msg, app.chatHistory.activeSessionId);
          app.setShowProviderAuthDialog(false);
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

      {app.showUpdate &&
        app.pendingUpdate &&
        app.pendingUpdate.component !== "tauri" && (
          <Dialog
            isOpen={true}
            type={getUpdateAvailableDialog(
              `squigit-${app.pendingUpdate.component}`,
            )}
            onAction={(key) => {
              if (key === "show_changelog" && app.pendingUpdate) {
                app.handleSelectChat(
                  `__system_update_${app.pendingUpdate.version}`,
                );
              }
              app.setShowUpdate(false);
              sessionStorage.setItem("update_dismissed", "1");
            }}
          />
        )}

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
};
