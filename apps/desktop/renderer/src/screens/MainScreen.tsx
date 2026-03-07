/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { SidePanel, TitleBar } from "@/layout";

import styles from "./MainScreen.module.css";

type MainScreenProps = {
  content: React.ReactNode;
  dialogs?: React.ReactNode;
  onContextMenu: React.MouseEventHandler<HTMLDivElement>;
  containerRef?: React.Ref<HTMLDivElement>;
  isSidePanelOpen: boolean;
  enablePanelAnimation: boolean;
};

export const MainScreen: React.FC<MainScreenProps> = ({
  content,
  dialogs,
  onContextMenu,
  containerRef,
  isSidePanelOpen,
  enablePanelAnimation,
}) => {
  return (
    <div
      ref={containerRef}
      onContextMenu={onContextMenu}
      className={styles.chatContainer}
    >
      <TitleBar />
      <div className={styles.mainContent}>
        <div
          className={`
            ${styles.sidePanelWrapper}
            ${!isSidePanelOpen ? styles.hidden : ""}
            ${enablePanelAnimation ? styles.animated : ""}`}
        >
          <SidePanel />
        </div>
        <div className={styles.contentArea}>{content}</div>
      </div>
      {dialogs}
    </div>
  );
};
