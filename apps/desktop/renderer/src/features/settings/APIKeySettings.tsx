/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect, useRef, useCallback } from "react";
import { Eye, EyeOff } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { github, google } from "@/core/services";
import { GlowCard } from "@/components/ui";
import { TextContextMenu } from "@/app/layout/menus/TextContextMenu";
import { useTextContextMenu, useTextEditor } from "@/hooks/editor";
import styles from "./APIKeySettings.module.css";

interface APIKeySettingsProps {
  providerApiKey: string;
  imgbbKey: string;
  isGuest?: boolean;
  onSetAPIKey: (
    provider: "google ai studio" | "imgbb",
    key: string,
  ) => Promise<boolean>;
}
const ProviderRow = ({
  title,
  providerKeyName,
  description,
  dashboardUrl,
  currentKey,
  onSave,
  isGuest = false,
}: {
  title: string;
  providerKeyName: string;
  description: string;
  currentKey: string;
  dashboardUrl: string;
  onSave: (key: string) => Promise<boolean>;
  isGuest?: boolean;
}) => {
  const [inputValue, setInputValue] = useState(currentKey);
  const [showKey, setShowKey] = useState(false);
  const [isValid, setIsValid] = useState(true);
  const isFocusedRef = useRef(false);

  const saveIdRef = useRef(0);

  useEffect(() => {
    if (!isFocusedRef.current) {
      setInputValue(currentKey);
      setIsValid(true);
    }
  }, [currentKey]);

  const validate = (key: string): boolean => {
    if (!key) return true;
    if (providerKeyName === "Google AI Studio") {
      return key.startsWith("AIzaS") && key.length === 39;
    }
    if (providerKeyName === "ImgBB") {
      return key.length === 32;
    }
    return true;
  };

  const handleInputChange = (newValue: string) => {
    setInputValue(newValue);
    const trimmed = newValue.trim();
    setIsValid(validate(trimmed));
  };

  const handleBlur = () => {
    isFocusedRef.current = false;
    const trimmed = inputValue.trim();
    const valid = validate(trimmed);

    if (!valid) {
      setInputValue(currentKey);
      setIsValid(true);
      return;
    }

    if (trimmed === currentKey) return;

    if (isGuest) {
      setIsValid(false);
      setTimeout(() => setIsValid(true), 1000);
      return;
    }

    const id = ++saveIdRef.current;
    setTimeout(() => {
      onSave(trimmed).then((success) => {
        if (saveIdRef.current !== id) return;
        if (!success) {
          setIsValid(false);
          alert(
            "Failed to save API key. Please ensure you are logged in to a profile.",
          );
        }
      });
    }, 0);
  };
  const {
    ref,
    hasSelection,
    handleCopy,
    handleCut,
    handlePaste,
    handleSelectAll,
    handleKeyDown: editorKeyDown,
  } = useTextEditor({
    value: inputValue,
    onChange: handleInputChange,
    preventNewLine: true,
  });

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      const isMod = e.ctrlKey || e.metaKey;
      if (isMod && e.key === "c") {
        e.preventDefault();
        handleCopy();
        return;
      }
      if (isMod && e.key === "x") {
        e.preventDefault();
        handleCut();
        return;
      }
      editorKeyDown(e as any);
    },
    [editorKeyDown, handleCopy, handleCut],
  );

  const {
    data: contextMenu,
    handleContextMenu,
    handleClose: handleCloseContextMenu,
  } = useTextContextMenu({
    hasSelection,
  });
  const handleOpenUrl = (url: string) => {
    invoke("open_external_url", { url });
  };
  const inputRef = ref as React.RefObject<HTMLInputElement>;
  return (
    <div className={styles.providerWrapper}>
      <GlowCard className={styles.glowProviderRow}>
        <div className={styles.providerRowInternal}>
          <div className={styles.providerInfo}>
            <div className={styles.providerName}>
              {title}
              {title.toLowerCase() === "imgbb" ? "*" : ""}
            </div>
            <div className={styles.providerMeta}>{description}</div>
          </div>
        </div>
      </GlowCard>
      <div className={styles.accordionContent}>
        <div className={styles.innerPadding}>
          <p className={styles.helperText}>
            You can put in{" "}
            <button
              className={styles.providerLink}
              onClick={() => handleOpenUrl(dashboardUrl)}
            >
              your {providerKeyName} key
            </button>{" "}
            {title.toLowerCase() === "imgbb"
              ? "to get a free public image link."
              : `to use ${title} models at cost.`}
          </p>
          <div
            className={`${styles.inputGroup} ${isGuest ? styles.disabled : ""}`}
          >
            <div
              className={`${styles.inputWrapper} ${!isValid ? styles.error : ""}`}
            >
              <input
                ref={inputRef}
                type={showKey ? "text" : "password"}
                className={styles.modernInput}
                placeholder={`Enter your ${providerKeyName} API Key`}
                value={inputValue}
                disabled={isGuest}
                onChange={(e) => handleInputChange(e.target.value)}
                onFocus={() => {
                  isFocusedRef.current = true;
                }}
                onBlur={handleBlur}
                onClick={(e) => e.stopPropagation()}
                onContextMenu={handleContextMenu}
                onKeyDown={handleKeyDown}
                autoComplete="off"
              />
              <button
                className={styles.iconBtn}
                disabled={isGuest}
                onClick={(e) => {
                  e.stopPropagation();
                  setShowKey(!showKey);
                }}
                title={showKey ? "Hide key" : "Show key"}
              >
                {showKey ? <EyeOff size={14} /> : <Eye size={14} />}
              </button>
            </div>
          </div>
        </div>
      </div>
      {contextMenu.isOpen && (
        <TextContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={handleCloseContextMenu}
          onCopy={handleCopy}
          onCut={handleCut}
          onPaste={handlePaste}
          onSelectAll={handleSelectAll}
          hasSelection={hasSelection}
        />
      )}
    </div>
  );
};
export const APIKeySettings: React.FC<APIKeySettingsProps> = ({
  providerApiKey,
  imgbbKey,
  onSetAPIKey,
  isGuest,
}) => {
  const handleOpenUrl = (url: string) =>
    invoke("open_external_url", { url: url });
  return (
    <section className={styles.container} aria-labelledby="apikeys-heading">
      <header className={styles.sectionHeader}>
        <h2 id="apikeys-heading" className={styles.sectionTitle}>
          API Keys
        </h2>
      </header>
      <div className={styles.group}>
        <ProviderRow
          title="Gemini"
          providerKeyName="Google AI Studio"
          description="Required for AI features"
          currentKey={providerApiKey}
          dashboardUrl={google.aiStudio.key}
          onSave={(key) => onSetAPIKey("google ai studio", key)}
          isGuest={isGuest}
        />
        <ProviderRow
          title="ImgBB"
          providerKeyName="ImgBB"
          description="Reverse Image Search"
          currentKey={imgbbKey}
          dashboardUrl={"https://api.imgbb.com/"}
          onSave={(key) => onSetAPIKey("imgbb", key)}
          isGuest={isGuest}
        />
        <div className={styles.legalNote}>
          <span className={styles.legalNoteStar}>*</span>
          <p className={styles.legalNoteText}>
            ImgBB is a free image hosting service that generates public URLs via
            API to enable reverse image search. We strongly recommend against
            using this option for sensitive data.{" "}
            <button
              className={styles.privacyLink}
              onClick={() =>
                handleOpenUrl(github.docs("06-policies/SECURITY.md"))
              }
            >
              Learn more.
            </button>
          </p>
        </div>
      </div>
      <div className={styles.aboutSection}>
        <div className={styles.divider} />
        <div className={styles.legalRow}>
          <span>Your keys are stored locally — </span>
          <button
            className={styles.privacyLink}
            onClick={() => handleOpenUrl(github.docs("06-policies/BYOK.md"))}
          >
            We never see them.
          </button>
        </div>
      </div>
    </section>
  );
};
