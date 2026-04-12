/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  noFocusSvg,
  closeNormalSvg,
  closeHoverSvg,
  closePressSvg,
  minimizeNormalSvg,
  minimizeHoverSvg,
  minimizePressSvg,
  maximizeNormalSvg,
  maximizeHoverSvg,
  maximizePressSvg,
} from "@/assets";
import styles from "./TrafficLights.module.css";

type TrafficButton = "close" | "minimize" | "maximize";

const NORMAL_SVGS: Record<TrafficButton, string> = {
  close: closeNormalSvg,
  minimize: minimizeNormalSvg,
  maximize: maximizeNormalSvg,
};

const HOVER_SVGS: Record<TrafficButton, string> = {
  close: closeHoverSvg,
  minimize: minimizeHoverSvg,
  maximize: maximizeHoverSvg,
};

const PRESS_SVGS: Record<TrafficButton, string> = {
  close: closePressSvg,
  minimize: minimizePressSvg,
  maximize: maximizePressSvg,
};

export const TrafficLights: React.FC = () => {
  const [isWindowFocused, setIsWindowFocused] = useState<boolean>(() =>
    document.hasFocus(),
  );
  const [isClusterHovered, setIsClusterHovered] = useState(false);
  const [pressedButton, setPressedButton] = useState<TrafficButton | null>(
    null,
  );

  useEffect(() => {
    const handleFocus = () => setIsWindowFocused(true);
    const handleBlur = () => {
      setIsWindowFocused(false);
      setIsClusterHovered(false);
      setPressedButton(null);
    };
    const handleMouseUp = () => setPressedButton(null);

    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("blur", handleBlur);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  const handleClose = () => invoke("close_window");
  const handleMinimize = () => invoke("minimize_window");
  const handleMaximize = () => invoke("maximize_window");

  const getButtonSvg = (button: TrafficButton): string => {
    if (!isWindowFocused) return noFocusSvg;
    if (pressedButton === button) return PRESS_SVGS[button];
    if (isClusterHovered) return HOVER_SVGS[button];
    return NORMAL_SVGS[button];
  };

  const handleMouseEnterButton = () => {
    setIsClusterHovered(true);
  };

  const handleMouseLeaveButton = (
    event: React.MouseEvent<HTMLButtonElement>,
  ) => {
    const nextTarget = event.relatedTarget as Node | null;
    const stillInsideCluster =
      nextTarget instanceof Element &&
      !!nextTarget.closest(`.${styles.trafficLights}`);
    if (!stillInsideCluster) {
      setIsClusterHovered(false);
      setPressedButton(null);
    }
  };

  return (
    <div className={styles.trafficLights}>
      <button
        type="button"
        aria-label="Close window"
        className={styles.trafficButton}
        onMouseEnter={handleMouseEnterButton}
        onMouseLeave={handleMouseLeaveButton}
        onMouseDown={(event) => {
          if (event.button === 0) setPressedButton("close");
        }}
        onClick={handleClose}
      >
        <img
          src={getButtonSvg("close")}
          alt=""
          aria-hidden="true"
          className={`${styles.icon} ${!isWindowFocused ? styles.iconUnfocused : ""}`}
          draggable={false}
        />
      </button>
      <button
        type="button"
        aria-label="Minimize window"
        className={styles.trafficButton}
        onMouseEnter={handleMouseEnterButton}
        onMouseLeave={handleMouseLeaveButton}
        onMouseDown={(event) => {
          if (event.button === 0) setPressedButton("minimize");
        }}
        onClick={handleMinimize}
      >
        <img
          src={getButtonSvg("minimize")}
          alt=""
          aria-hidden="true"
          className={`${styles.icon} ${!isWindowFocused ? styles.iconUnfocused : ""}`}
          draggable={false}
        />
      </button>
      <button
        type="button"
        aria-label="Toggle fullscreen"
        className={styles.trafficButton}
        onMouseEnter={handleMouseEnterButton}
        onMouseLeave={handleMouseLeaveButton}
        onMouseDown={(event) => {
          if (event.button === 0) setPressedButton("maximize");
        }}
        onClick={handleMaximize}
      >
        <img
          src={getButtonSvg("maximize")}
          alt=""
          aria-hidden="true"
          className={`${styles.icon} ${!isWindowFocused ? styles.iconUnfocused : ""}`}
          draggable={false}
        />
      </button>
    </div>
  );
};
