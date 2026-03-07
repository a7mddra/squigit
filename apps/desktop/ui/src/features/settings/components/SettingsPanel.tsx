/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  Settings,
  Package,
  Fingerprint,
  Sparkles,
  HelpCircle,
} from "lucide-react";
import React, { useState } from "react";
import {
  Dropdown,
  DropdownSectionTitle,
  DropdownAction,
  DropdownDivider,
} from "@/components";
import styles from "./SettingsPanel.module.css";
import { SettingsSection } from "@/features";

interface SettingsPanelProps {
  onOpenSettings: (section: SettingsSection) => void;
  isSettingsOpen: boolean;
  onCloseSettings: () => void;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({
  onOpenSettings,
  isSettingsOpen,
  onCloseSettings,
}) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpenSettings = (section: SettingsSection) => {
    setIsOpen(false);
    onOpenSettings(section);
  };

  const handleOpenChange = (newOpen: boolean) => {
    if (isSettingsOpen) {
      onCloseSettings();
    }
    setIsOpen(newOpen);
  };

  return (
    <Dropdown
      className={styles.settingsPanel}
      label={
        <Settings
          size={22}
          className={`${styles.triggerIcon} ${isOpen ? styles.iconActive : ""}`}
        />
      }
      width={200}
      isOpen={isOpen}
      onOpenChange={handleOpenChange}
      hideChevron
    >
      <DropdownSectionTitle>Settings</DropdownSectionTitle>
      <DropdownAction
        icon={<Settings size={18} />}
        onClick={() => handleOpenSettings("general")}
      >
        General
      </DropdownAction>

      <DropdownAction
        icon={<Package size={18} />}
        onClick={() => handleOpenSettings("models")}
      >
        Models
      </DropdownAction>

      <DropdownAction
        icon={<Fingerprint size={18} />}
        onClick={() => handleOpenSettings("apikeys")}
      >
        API keys
      </DropdownAction>

      <DropdownDivider />

      <DropdownAction
        icon={<Sparkles size={18} />}
        onClick={() => handleOpenSettings("personalization")}
      >
        Personalization
      </DropdownAction>

      <DropdownAction
        icon={<HelpCircle size={18} />}
        onClick={() => handleOpenSettings("help")}
      >
        Help & support
      </DropdownAction>
    </Dropdown>
  );
};
