/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { AppLogo } from "@/assets";
import { TitleBar } from "@/layout";

import styles from "./SplashScreen.module.css";

type SplashScreenProps = {
  onContextMenu: React.MouseEventHandler<HTMLDivElement>;
};

export const SplashScreen: React.FC<SplashScreenProps> = ({
  onContextMenu,
}) => {
  return (
    <div className={styles.chatContainer} onContextMenu={onContextMenu}>
      <TitleBar />
      <div className={styles.mainContent}>
        <div className={styles.loadingContent}>
          <div className={styles.logoOffset}>
            <AppLogo size={40} color="var(--c-raw-002)" />
          </div>
        </div>
      </div>
    </div>
  );
};
