/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from "react";
import { usePlatform } from "@/hooks";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import styles from "./Welcome.module.css";
import { ImageResult, storeImageFromPath } from "@/lib";
import { useAppContext } from "@/providers/AppProvider";
import { OnboardingLayout } from "../OnboardingLayout";
import { AppLogo } from "@/assets";

const ALLOWED_EXTENSIONS = [".jpg", ".jpeg", ".png", ".webp"];

export interface CASImageData {
  imageId: string;
  path: string;
}

interface WelcomeProps {
  onImageReady: (data: CASImageData) => void;
  isActive?: boolean;
  isGuest?: boolean;
  onLoginRequired?: () => void;
}

export const Welcome: React.FC<WelcomeProps> = ({
  onImageReady,
  isActive = true,
  isGuest = false,
  onLoginRequired,
}) => {
  const app = useAppContext();
  const [isDragging, setIsDragging] = useState(false);
  const { isMac, isWin, modSymbol, shiftSymbol } = usePlatform();

  const platformInfo = React.useMemo(() => {
    if (isMac) {
      return {
        screenshotKeys: (
          <>
            <span className={styles.key}>{modSymbol}</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>{shiftSymbol}</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>A</span>
          </>
        ),
        pasteKeys: (
          <>
            <span className={styles.key}>{modSymbol}</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>V</span>
          </>
        ),
      };
    } else if (isWin) {
      return {
        screenshotKeys: (
          <>
            <span className={styles.key}>Win</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>Shift</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>A</span>
          </>
        ),
        pasteKeys: (
          <>
            <span className={styles.key}>Ctrl</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>V</span>
          </>
        ),
      };
    } else {
      return {
        screenshotKeys: (
          <>
            <span className={styles.key}>Super</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>Shift</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>A</span>
          </>
        ),
        pasteKeys: (
          <>
            <span className={styles.key}>Ctrl</span>
            <span className={styles.keySep}>+</span>
            <span className={styles.key}>V</span>
          </>
        ),
      };
    }
  }, [isMac, isWin, modSymbol, shiftSymbol]);

  useEffect(() => {
    if (!isActive) return;

    const handlePaste = async (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "v") {
        if (isGuest) {
          onLoginRequired?.();
          return;
        }
        try {
          const result = await invoke<ImageResult>("read_clipboard_image");
          if (result) {
            onImageReady({
              imageId: result.hash,
              path: result.path,
            });
          }
        } catch (error) {
          console.error("Failed to read clipboard image:", error);
        }
      }
    };

    window.addEventListener("keydown", handlePaste);

    return () => {
      window.removeEventListener("keydown", handlePaste);
    };
  }, [onImageReady, isActive]);

  useEffect(() => {
    if (!isActive) return;

    const unlistenHover = listen<{ paths: string[] }>(
      "tauri://drag-over",
      () => {
        setIsDragging(true);
      },
    );

    const unlistenLeave = listen("tauri://drag-leave", () => {
      setIsDragging(false);
    });

    const unlistenDrop = listen<{ paths: string[] }>(
      "tauri://drag-drop",
      async (event) => {
        setIsDragging(false);
        const paths = event.payload.paths;

        if (paths && paths.length > 0) {
          const filePath = paths[0];

          const lowerPath = filePath.toLowerCase();
          const isAllowed = ALLOWED_EXTENSIONS.some((ext) =>
            lowerPath.endsWith(ext),
          );

          if (isAllowed) {
            if (isGuest) {
              onLoginRequired?.();
              return;
            }
            try {
              console.log("Processing dropped file:", filePath);
              const result = await storeImageFromPath(filePath);
              onImageReady({
                imageId: result.hash,
                path: result.path,
              });
            } catch (error) {
              console.error("Failed to process dropped file:", error);
            }
          } else {
            console.warn(
              "Dropped file is not an allowed image type:",
              filePath,
            );
          }
        }
      },
    );

    return () => {
      unlistenHover.then((fn) => fn());
      unlistenLeave.then((fn) => fn());
      unlistenDrop.then((fn) => fn());
    };
  }, [onImageReady, isActive]);

  return (
    <OnboardingLayout
      className={`${isDragging ? styles.dragging : ""}`}
      tabIndex={-1}
    >
      <div className={styles.content}>
        <AppLogo size={80} color="var(--c-raw-000)" />
        <h1 className={styles.title}>{app.system.appName}</h1>

        <div className={styles.actions}>
          <div className={styles.actionRow}>
            <span className={styles.actionLabel}>
              Analyze part of your screen
            </span>
            <span className={styles.actionKeys}>
              {platformInfo.screenshotKeys}
            </span>
          </div>
          <div className={styles.actionRow}>
            <span className={styles.actionLabel}>Paste an image</span>
            <span className={styles.actionKeys}>{platformInfo.pasteKeys}</span>
          </div>
          <div className={styles.actionRow}>
            <span className={styles.actionLabel}>Drop an image</span>
            <span className={styles.actionKeys}>
              <span className={styles.key} title="Drag">
                D
              </span>
              <span className={styles.keySep}>&</span>
              <span className={styles.key} title="Drop">
                D
              </span>
            </span>
          </div>
        </div>
      </div>
    </OnboardingLayout>
  );
};
