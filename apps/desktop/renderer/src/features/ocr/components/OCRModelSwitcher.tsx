/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef, useEffect, useState, useLayoutEffect } from "react";
import { createPortal } from "react-dom";
import { ChevronDown, Check, PackagePlus } from "lucide-react";
import { SettingsSection, useModelsStore, getLanguageCode } from "@/features";
import { DEFAULT_OCR_MODEL_ID } from "@/core";
import styles from "./OCRModelSwitcher.module.css";

interface OCRModelSwitcherProps {
  currentOcrModel: string;
  onOcrModelChange: (model: string) => void;
  onOpenSettings: (section: SettingsSection) => void;
  disabled?: boolean;
}

export const OCRModelSwitcher: React.FC<OCRModelSwitcherProps> = ({
  currentOcrModel,
  onOcrModelChange,
  onOpenSettings,
  disabled = false,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dropdownStyle, setDropdownStyle] = useState<React.CSSProperties>({});
  const models = useModelsStore((s) => s.models);
  const installedModels = models.filter((m) => m.state === "downloaded");

  const updatePosition = () => {
    if (isOpen && containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      setDropdownStyle({
        position: "fixed",
        top: `${rect.bottom + 18}px`,
        right: `${window.innerWidth - rect.right}px`,
        minWidth: "220px",
        zIndex: 9999,
      });
    }
  };

  useLayoutEffect(() => {
    updatePosition();
    window.addEventListener("scroll", updatePosition, true);
    window.addEventListener("resize", updatePosition);

    return () => {
      window.removeEventListener("scroll", updatePosition, true);
      window.removeEventListener("resize", updatePosition);
    };
  }, [isOpen]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        const target = event.target as Element;
        if (!target.closest(`.${styles.dropdown}`)) {
          setIsOpen(false);
        }
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  const isCurrentModelValid = installedModels.some(
    (m) => m.id === currentOcrModel,
  );
  const fallbackModel =
    installedModels.find((model) => model.id === DEFAULT_OCR_MODEL_ID)?.id ||
    installedModels[0]?.id ||
    "";
  const effectiveModel = !currentOcrModel
    ? ""
    : isCurrentModelValid
      ? currentOcrModel
      : fallbackModel;

  const dropdownContent = (
    <div
      className={`${styles.dropdown} ${isOpen ? styles.open : ""}`}
      style={dropdownStyle}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <div className={styles.sectionTitle}>OCR Model</div>
      <div className={styles.modelList}>
        {installedModels.map((model) => (
          <button
            key={model.id}
            className={`${styles.modelItem} ${
              model.id === effectiveModel ? styles.activeModel : ""
            }`}
            onClick={() => {
              onOcrModelChange(model.id);
              setIsOpen(false);
            }}
          >
            <div className={styles.modelInfo}>
              <span className={styles.modelName}>{model.name}</span>
            </div>
            {model.id === effectiveModel && (
              <Check size={14} className={styles.checkIcon} />
            )}
          </button>
        ))}
      </div>

      <div className={styles.divider} />

      <div className={styles.actions}>
        <button
          className={styles.actionButton}
          onClick={() => {
            onOpenSettings("models");
            setIsOpen(false);
          }}
        >
          <PackagePlus size={16} />
          <span>Get more models</span>
        </button>
      </div>
    </div>
  );

  return (
    <div className={styles.modelSwitcher} ref={containerRef}>
      <button
        disabled={disabled}
        className={`${styles.trigger} ${styles.triggerGlobal} ${isOpen ? styles.active : ""}`}
        onClick={() => setIsOpen(!isOpen)}
        title="Select OCR Model"
      >
        <span className={styles.triggerText}>
          {effectiveModel ? getLanguageCode(effectiveModel) : "Select Model"}
        </span>
        <ChevronDown
          size={14}
          className={`${styles.chevron} ${styles.chevronClr} ${isOpen ? styles.rotate : ""}`}
        />
      </button>

      {createPortal(dropdownContent, document.body)}
    </div>
  );
};
