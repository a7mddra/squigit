/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Box, Pin } from "lucide-react";
import { SidePanelToggleIcon } from "@/components/icons";
import { usePlatform } from "@/hooks/shared";
import { TrafficLights } from "./TrafficLights";
import { WindowControls } from "./WindowControls";
import { AccountSwitcher } from "./AccountSwitcher";
import { AuthButton } from "./AuthButton";
import { SettingsMenu } from "./SettingsMenu";
import { useAppContext } from "../../providers/AppProvider";
import { TitleBarContextMenu } from "../menus/TitleBarContextMenu";
import { SettingsOverlay } from "../overlays/SettingsOverlay";
import styles from "./TitleBar.module.css";

export const TitleBar: React.FC = () => {
  const app = useAppContext();
  const { os: platform } = usePlatform();
  const [isAlwaysOnTop, setIsAlwaysOnTop] = useState(false);
  const [hasSeenUpdateButton, setHasSeenUpdateButton] = useState(false);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
  } | null>(null);

  const isUnix = platform === "macos" || platform === "linux";
  const isWindows = platform === "windows";
  const currentWindow = getCurrentWindow();
  const dragRegionProps = isWindows
    ? {}
    : ({ "data-tauri-drag-region": true } as const);
  const pendingUpdate = app.pendingUpdate;
  const updateTitle = pendingUpdate
    ? `Update Available: ${pendingUpdate.version}`
    : "Update Available";

  useEffect(() => {
    setHasSeenUpdateButton(false);
  }, [pendingUpdate?.version]);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const handleToggleAlwaysOnTop = async () => {
    const newState = !isAlwaysOnTop;
    setIsAlwaysOnTop(newState);
    try {
      await invoke("set_always_on_top", { state: newState });
    } catch (error) {
      console.error("Failed to toggle always on top:", error);
      // Revert on failure
      setIsAlwaysOnTop(!newState);
    }
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLElement>) => {
    if (
      contextMenu &&
      !(e.target as Element).closest("[data-is-context-menu]")
    ) {
      setContextMenu(null);
    }

    if (!isWindows || e.button !== 0 || contextMenu) return;

    const target = e.target as Element | null;
    if (!target) return;

    const isInteractive = target.closest(
      [
        "button",
        "a",
        "input",
        "textarea",
        "select",
        "option",
        "label",
        "[role='button']",
        "[contenteditable='true']",
        "[data-no-window-drag]",
        "[data-is-context-menu]",
      ].join(","),
    );

    if (isInteractive) return;

    e.preventDefault();
    currentWindow.startDragging().catch((error) => {
      console.error("Failed to start window drag:", error);
    });
  };

  const handleOpenUpdate = () => {
    if (!pendingUpdate) return;
    setHasSeenUpdateButton(true);
    app.handleSelectChat(`__system_update_${pendingUpdate.version}`);
  };

  return (
    <header
      className={`${styles.header} ${isWindows ? styles.headerWindows : ""}`}
      {...dragRegionProps}
      onContextMenu={handleContextMenu}
      onMouseDown={handleMouseDown}
    >
      <h1 className={styles.chatTitle}>{app.chatTitle}</h1>

      <div className={styles.leftSection}>
        {isUnix && <TrafficLights />}

        {!app.isLoadingState && (
          <div className={styles.controlsWrapper}>
            <button
              onClick={app.toggleSidePanel}
              className={`${styles.iconButton} ${
                app.isSidePanelOpen ? styles.active : ""
              }`}
              title="Recent Chats"
              aria-label="Recent chats"
            >
              <SidePanelToggleIcon size={20} active={app.isSidePanelOpen} />
            </button>
          </div>
        )}
      </div>

      <div
        className={`${styles.rightSection} ${
          isWindows ? styles.rightSectionWindows : ""
        }`}
      >
        {!app.isLoadingState && (
          <>
            {pendingUpdate && (
              <button
                onClick={handleOpenUpdate}
                className={`${styles.iconButton} ${styles.updateButton} ${
                  app.chatHistory.activeSessionId?.startsWith("__system_update")
                    ? styles.active
                    : ""
                }`}
                title={updateTitle}
                aria-label={updateTitle}
              >
                <span className={styles.updateEmoji} aria-hidden="true">
                  <Box size={22} />
                </span>
                {!hasSeenUpdateButton && (
                  <span className={styles.updateDot} aria-hidden="true" />
                )}
              </button>
            )}

            <SettingsMenu
              onOpenSettings={app.system.openSettings}
              isSettingsOpen={app.system.isSettingsOpen}
              onCloseSettings={() => app.system.setSettingsOpen(false)}
            />
            <SettingsOverlay
              isOpen={app.system.isSettingsOpen}
              onClose={() => app.system.setSettingsOpen(false)}
              activeSection={app.system.settingsSection}
              onSectionChange={app.system.setSettingsSection}
              currentPrompt={app.system.prompt}
              defaultModel={app.system.startupModel}
              defaultOcrLanguage={app.system.startupOcrLanguage}
              updatePreferences={app.system.updatePreferences}
              themePreference={app.system.themePreference}
              onSetTheme={app.system.onSetTheme}
              autoExpandOCR={app.system.autoExpandOCR}
              ocrEnabled={app.system.ocrEnabled}
              captureType={app.system.captureType}
              providerApiKey={app.system.apiKey}
              imgbbKey={app.system.imgbbKey}
              onSetAPIKey={app.system.handleSetAPIKey}
              isGuest={!app.system.activeProfile}
            />

            {app.system.activeProfile ? (
              <>
                <AccountSwitcher
                  activeProfile={app.system.activeProfile}
                  onNewSession={app.handleNewSession}
                  profiles={app.system.profiles}
                  onSwitchProfile={app.handleSwitchProfile}
                  onAddAccount={app.handleAddAccount}
                  onCancelAuth={app.system.cancelAuth}
                  onLogout={app.performLogout}
                  onDeleteProfile={app.system.deleteProfile}
                  switchingProfileId={app.system.switchingProfileId}
                />
              </>
            ) : (
              <AuthButton
                onLogin={app.handleAddAccount}
                onCancel={app.system.cancelAuth}
                isLoading={app.system.switchingProfileId === "creating_account"}
                disabled={app.isAgreementPending && !app.agreedToTerms}
                disabledTitle="Please read and agree to the instructions first"
              />
            )}
          </>
        )}

        {isUnix && (
          <button
            onClick={handleToggleAlwaysOnTop}
            className={`${styles.iconButton} ${isAlwaysOnTop ? styles.active : ""}`}
            title={isAlwaysOnTop ? "Unpin window" : "Pin window on top"}
            aria-label={isAlwaysOnTop ? "Unpin window" : "Pin window on top"}
          >
            <Pin
              size={14}
              style={{ transform: "rotate(45deg)" }}
              fill={isAlwaysOnTop ? "currentColor" : "none"}
            />
          </button>
        )}

        {isWindows && <WindowControls />}
      </div>

      {contextMenu && (
        <TitleBarContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          onNewThread={app.handleNewSession}
          onOpenSettings={() => app.system.openSettings("general")}
          isAlwaysOnTop={isAlwaysOnTop}
          onToggleAlwaysOnTop={handleToggleAlwaysOnTop}
        />
      )}
    </header>
  );
};
