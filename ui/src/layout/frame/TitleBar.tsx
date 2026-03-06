/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Pin } from "lucide-react";
import { useAppContext } from "@/providers/AppProvider";
import { usePlatform } from "@/hooks";
import { SettingsPanel, SettingsOverlay } from "@/features";
import { TrafficLights } from "./TrafficLights";
import { WindowControls } from "./WindowControls";
import { AccountSwitcher } from "./AccountSwitcher";
import { AuthButton } from "./AuthButton";
import { TitleBarContextMenu } from "@/layout";
import styles from "./TitleBar.module.css";

export const TitleBar: React.FC = () => {
  const app = useAppContext();
  const { os: platform } = usePlatform();
  const [isAlwaysOnTop, setIsAlwaysOnTop] = useState(false);
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
              <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <path
                  d="M9 3V21H5C3.89543 21 3 20.1046 3 19V5C3 3.89543 3.89543 3 5 3H9Z"
                  fill={app.isSidePanelOpen ? "currentColor" : "none"}
                  stroke={app.isSidePanelOpen ? "none" : "currentColor"}
                />
                <line x1="9" y1="3" x2="9" y2="21" />
              </svg>
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
            <SettingsPanel
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
              geminiKey={app.system.apiKey}
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
          onNewChat={app.handleNewSession}
          onOpenSettings={() => app.system.openSettings("general")}
          isAlwaysOnTop={isAlwaysOnTop}
          onToggleAlwaysOnTop={handleToggleAlwaysOnTop}
        />
      )}
    </header>
  );
};
