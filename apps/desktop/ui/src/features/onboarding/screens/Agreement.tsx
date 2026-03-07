/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo } from "react";
import { usePlatform } from "@/hooks";
import { ChatBubble, Message } from "@/features";
import { useAppContext } from "@/providers/AppProvider";
import { OnboardingLayout } from "../OnboardingLayout";

import styles from "./Agreement.module.css";

import {
  linuxInstruction as linux,
  macosInstruction as macos,
  windowsInstruction as windows,
} from "@/assets";

const INSTRUCTIONS: Record<string, string> = {
  linux,
  macos,
  windows,
};

export const Agreement: React.FC = () => {
  const { isMac, isWin } = usePlatform();
  const app = useAppContext();
  const [selected, setSelected] = React.useState("disagree");

  const content = useMemo(() => {
    if (isMac) return INSTRUCTIONS.macos;
    if (isWin) return INSTRUCTIONS.windows;
    return INSTRUCTIONS.linux;
  }, [isMac, isWin]);

  const message: Message = {
    id: "welcome-intro",
    role: "system",
    text: content,
    timestamp: Date.now(),
  };

  const handleSelection = (value: string) => {
    setSelected(value);
    app.handleSystemAction(value, value);
  };

  return (
    <OnboardingLayout allowScroll contentClassName={styles.content}>
      <div className={styles.inner}>
        <ChatBubble message={message} />
        <div className={styles.actions}>
          <label
            className={`${styles.radioAction} ${
              selected === "agree" ? styles.radioSelected : ""
            }`}
          >
            <input
              type="radio"
              name="agreement"
              checked={selected === "agree"}
              onChange={() => handleSelection("agree")}
              className={styles.radioInput}
            />
            <span>I have read and understand the instructions</span>
          </label>
          <label
            className={`${styles.radioAction} ${
              selected === "disagree" ? styles.radioSelected : ""
            }`}
          >
            <input
              type="radio"
              name="agreement"
              checked={selected === "disagree"}
              onChange={() => handleSelection("disagree")}
              className={styles.radioInput}
            />
            <span>I do not understand</span>
          </label>
        </div>
        <div className={styles.licenseText}>
          By installing this software, you agree to the{" "}
          <strong>
            <a
              href="https://github.com/a7mddra/snapllm?tab=Apache-2.0-1-ov-file#readme"
              target="_blank"
              rel="noreferrer"
            >
              Apache 2.0 License
            </a>
          </strong>
          .
        </div>
      </div>
    </OnboardingLayout>
  );
};
