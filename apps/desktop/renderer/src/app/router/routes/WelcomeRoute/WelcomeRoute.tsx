/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useState } from "react";
import {
  linuxInstruction as linux,
  macosInstruction as macos,
  windowsInstruction as windows,
} from "@/assets";
import { ChatBubble } from "@/features/chat";
import { SettingsSection } from "@/features/settings";
import { usePlatform } from "@/hooks/shared";
import type { Message } from "@/core/brain/engine";
import styles from "./WelcomeRoute.module.css";

const INSTRUCTIONS: Record<string, string> = {
  linux,
  macos,
  windows,
};

const SETTINGS_LINKS: Array<{ label: string; section: SettingsSection }> = [
  { label: "Settings -> Models", section: "models" },
  { label: "Settings -> API Keys", section: "apikeys" },
  { label: "Settings -> Personalization", section: "personalization" },
  { label: "Settings -> Help & Support", section: "help" },
  { label: "Settings -> General", section: "general" },
];

const linkifySettingsMentions = (raw: string): string => {
  let next = raw;
  for (const item of SETTINGS_LINKS) {
    const escaped = item.label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const regex = new RegExp(escaped, "g");
    next = next.replace(regex, `[${item.label}](#settings-${item.section})`);
  }
  return next;
};

interface WelcomeRouteProps {
  onSystemAction: (actionId: string, value?: string) => void | Promise<void>;
  onOpenSettings: (section: SettingsSection) => void;
}

export const WelcomeRoute: React.FC<WelcomeRouteProps> = ({
  onSystemAction,
  onOpenSettings,
}) => {
  const { isMac, isWin } = usePlatform();
  const [selected, setSelected] = useState("disagree");

  const content = useMemo(() => {
    const raw = isMac
      ? INSTRUCTIONS.macos
      : isWin
        ? INSTRUCTIONS.windows
        : INSTRUCTIONS.linux;

    return linkifySettingsMentions(raw);
  }, [isMac, isWin]);

  const message: Message = {
    id: "welcome-intro",
    role: "system",
    text: content,
    timestamp: Date.now(),
  };

  const handleSelection = (value: string) => {
    setSelected(value);
    void onSystemAction(value, value);
  };

  const handleAgreementAction = (actionId: string, value?: string) => {
    if (actionId === "open_settings" && value) {
      onOpenSettings(value as SettingsSection);
      return;
    }
    void onSystemAction(actionId, value);
  };

  return (
    <div className={styles.container}>
      <div className={styles.content}>
        <div className={styles.inner}>
          <div>
            <ChatBubble
              message={message}
              enableInternalLinks={true}
              onAction={handleAgreementAction}
            />
          </div>
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
                href="https://github.com/a7mddra/squigit?tab=Apache-2.0-1-ov-file#readme"
                target="_blank"
                rel="noreferrer"
              >
                Apache 2.0 License
              </a>
            </strong>
            .
          </div>
        </div>
      </div>
    </div>
  );
};
