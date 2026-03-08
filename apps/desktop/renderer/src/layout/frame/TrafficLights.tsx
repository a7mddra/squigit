/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import noFocusSvg from "@/assets/icons/traffic-lights/0-all-three-nofocus.svg";
import closeNormalSvg from "@/assets/icons/traffic-lights/1-close-1-normal.svg";
import closeHoverSvg from "@/assets/icons/traffic-lights/2-close-2-hover.svg";
import closePressSvg from "@/assets/icons/traffic-lights/2-close-3-press.svg";
import minimizeNormalSvg from "@/assets/icons/traffic-lights/2-minimize-1-normal.svg";
import minimizeHoverSvg from "@/assets/icons/traffic-lights/2-minimize-2-hover.svg";
import minimizePressSvg from "@/assets/icons/traffic-lights/2-minimize-3-press.svg";
import maximizeNormalSvg from "@/assets/icons/traffic-lights/3-maximize-1-normal.svg";
import maximizeHoverSvg from "@/assets/icons/traffic-lights/3-maximize-2-hover.svg";
import maximizePressSvg from "@/assets/icons/traffic-lights/3-maximize-3-press.svg";
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
  const [hoveredButton, setHoveredButton] = useState<TrafficButton | null>(
    null,
  );
  const [pressedButton, setPressedButton] = useState<TrafficButton | null>(
    null,
  );

  useEffect(() => {
    const handleFocus = () => setIsWindowFocused(true);
    const handleBlur = () => {
      setIsWindowFocused(false);
      setHoveredButton(null);
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
    if (hoveredButton === button) return HOVER_SVGS[button];
    return NORMAL_SVGS[button];
  };

  const handleMouseLeaveButton = (button: TrafficButton) => {
    setHoveredButton((current) => (current === button ? null : current));
    setPressedButton((current) => (current === button ? null : current));
  };

  return (
    <div className={styles.trafficLights}>
      <button
        type="button"
        aria-label="Close window"
        className={styles.trafficButton}
        onMouseEnter={() => setHoveredButton("close")}
        onMouseLeave={() => handleMouseLeaveButton("close")}
        onMouseDown={(event) => {
          if (event.button === 0) setPressedButton("close");
        }}
        onClick={handleClose}
      >
        <img
          src={getButtonSvg("close")}
          alt=""
          aria-hidden="true"
          className={styles.icon}
          draggable={false}
        />
      </button>
      <button
        type="button"
        aria-label="Minimize window"
        className={styles.trafficButton}
        onMouseEnter={() => setHoveredButton("minimize")}
        onMouseLeave={() => handleMouseLeaveButton("minimize")}
        onMouseDown={(event) => {
          if (event.button === 0) setPressedButton("minimize");
        }}
        onClick={handleMinimize}
      >
        <img
          src={getButtonSvg("minimize")}
          alt=""
          aria-hidden="true"
          className={styles.icon}
          draggable={false}
        />
      </button>
      <button
        type="button"
        aria-label="Toggle fullscreen"
        className={styles.trafficButton}
        onMouseEnter={() => setHoveredButton("maximize")}
        onMouseLeave={() => handleMouseLeaveButton("maximize")}
        onMouseDown={(event) => {
          if (event.button === 0) setPressedButton("maximize");
        }}
        onClick={handleMaximize}
      >
        <img
          src={getButtonSvg("maximize")}
          alt=""
          aria-hidden="true"
          className={styles.icon}
          draggable={false}
        />
      </button>
    </div>
  );
};
