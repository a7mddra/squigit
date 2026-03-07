/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useState } from "react";
import styles from "./ModelsSection.module.css";
import { Dropdown, DropdownItem, DropdownSectionTitle } from "@/components";
import { UserPreferences, DEFAULT_OCR_MODEL_ID, MODELS } from "@/lib";
import { OCRModelDownloader, useModelsStore, getModelById } from "@/features";

interface ModelsSectionProps {
  localModel: string;
  ocrLanguage: string;
  updatePreferences: (updates: Partial<UserPreferences>) => void;
}

export const ModelsSection: React.FC<ModelsSectionProps> = ({
  localModel,
  ocrLanguage,
  updatePreferences,
}) => {
  const models = useModelsStore((s) => s.models);
  const installedModels = models.filter((m) => m.state === "downloaded");
  const [activeModel, setActiveModel] = useState(localModel);
  const [ocrMenuOpen, setOcrMenuOpen] = useState(false);
  const [aiMenuOpen, setAiMenuOpen] = useState(false);

  useEffect(() => {
    setActiveModel(localModel);
  }, [localModel]);

  const activeOcrModel =
    installedModels.find((m) => m.id === ocrLanguage) ??
    getModelById(DEFAULT_OCR_MODEL_ID);

  const handleModelSelect = useCallback(
    (id: string) => {
      setActiveModel(id);
      updatePreferences({ model: id });
      setAiMenuOpen(false);
    },
    [updatePreferences],
  );

  const handleOcrSelect = (id: string) => {
    updatePreferences({ ocrLanguage: id });
    setOcrMenuOpen(false);
  };

  const getModelLabel = (id: string) =>
    MODELS.find((model) => model.id === id)?.name || id;

  return (
    <section className={styles.container} aria-labelledby="models-heading">
      <header className={styles.sectionHeader}>
        <h2 id="models-heading" className={styles.sectionTitle}>
          Models
        </h2>
      </header>

      <div className={styles.group}>
        <div className={styles.row}>
          <div className={styles.rowMeta}>
            <span className={styles.label}>AI features</span>
            <span className={styles.description}>
              Choose your preferred model for future chats
            </span>
          </div>
          <div className={styles.rowControl}>
            <Dropdown
              label={getModelLabel(activeModel)}
              width={180}
              isOpen={aiMenuOpen}
              onOpenChange={setAiMenuOpen}
            >
              <DropdownSectionTitle>Gemini Models</DropdownSectionTitle>
              <div className={styles.list}>
                {MODELS.map((model) => (
                  <DropdownItem
                    key={model.id}
                    label={model.name}
                    isActive={activeModel === model.id}
                    onClick={() => handleModelSelect(model.id)}
                  />
                ))}
              </div>
            </Dropdown>
          </div>
        </div>

        <div className={styles.divider} />

        <div className={styles.row}>
          <div className={styles.rowMeta}>
            <span className={styles.label}>OCR language</span>
            <span className={styles.description}>
              Set the default language for text recognition
            </span>
          </div>
          <div className={styles.rowControl}>
            <Dropdown
              label={activeOcrModel?.name || "Select Model"}
              width={230}
              isOpen={ocrMenuOpen}
              onOpenChange={setOcrMenuOpen}
            >
              <DropdownSectionTitle>Installed Models</DropdownSectionTitle>
              <div className={`${styles.list} ${styles.ocr}`}>
                {installedModels.map((m) => (
                  <DropdownItem
                    key={m.id}
                    label={m.name}
                    isActive={activeOcrModel?.id === m.id}
                    onClick={() => handleOcrSelect(m.id)}
                  />
                ))}
              </div>
            </Dropdown>
          </div>
        </div>
      </div>

      <div className={styles.divider} />

      <OCRModelDownloader />
    </section>
  );
};
