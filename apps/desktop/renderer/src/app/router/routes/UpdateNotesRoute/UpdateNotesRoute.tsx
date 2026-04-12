/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useState } from "react";
import { clsx } from "clsx";
import { ChevronRight, DownloadCloud } from "lucide-react";
import { updateIcon } from "@/assets";
import { CodeBlock } from "@/components/ui";
import { getPendingUpdate, markUpdateDone, usePlatform } from "@/hooks";
import styles from "./UpdateNotesRoute.module.css";

interface UpdateSectionProps {
  title: string;
  items: string[];
  defaultOpen?: boolean;
}

interface UpdateNotesRouteProps {
  appName: string;
  onSystemAction: (actionId: string, value?: string) => void | Promise<void>;
}

const UpdateSection: React.FC<UpdateSectionProps> = ({
  title,
  items,
  defaultOpen = false,
}) => {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  if (!items || items.length === 0) return null;
  return (
    <div className={styles.section}>
      <div
        className={styles.sectionHeader}
        onClick={() => setIsOpen(!isOpen)}
        role="button"
        tabIndex={0}
      >
        <ChevronRight
          className={clsx(styles.chevron, isOpen && styles.chevronRotate)}
        />
        <span className={styles.sectionTitle}>{title}</span>
      </div>
      <div
        className={clsx(
          styles.gridWrapper,
          isOpen ? styles.gridOpen : styles.gridClosed,
        )}
      >
        <div className={styles.sectionContent}>
          <ul className={styles.sectionList}>
            {items.map((item, idx) => (
              <li key={idx}>{item}</li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
};

export const UpdateNotesRoute: React.FC<UpdateNotesRouteProps> = ({
  appName,
  onSystemAction,
}) => {
  const update = useMemo(() => getPendingUpdate(), []);
  const platform = usePlatform();

  const SECTION_ORDER = ["New Features", "Bug Fixes", "UI Improvements"];

  if (!update) {
    return null;
  }

  const sections = update.sections || {};
  const hasSections = Object.keys(sections).length > 0;

  const isTauri = update.component === "tauri";
  const isOcr = update.component === "ocr";

  const titleText = isTauri
    ? appName
    : `Squigit ${update.component.toUpperCase()}`;

  const getUpgradeCommand = () => {
    const pkg = isOcr ? "squigit-ocr" : "squigit-stt";
    return platform.getPkgUpgradeCmd(pkg);
  };

  const handleUpdate = () => {
    if (isTauri) {
      void onSystemAction("update_now");
    } else {
      markUpdateDone(update.component);
      void onSystemAction("dismiss_overlay");
    }
  };

  return (
    <div className={styles.screen}>
      <div className={`${styles.container} ${styles.appOverride}`}>
        <div className={styles.header}>
          <div className={styles.title}>
            <img
              src={updateIcon}
              alt=""
              aria-hidden="true"
              className={styles.titleIcon}
            />
            <span>{titleText}</span>
          </div>
          <div className={styles.subtitle}>v{update.version} is Here!</div>
        </div>

        <div className={styles.scrollableContent}>
          {!isTauri && (
            <div className={styles.section} style={{ marginBottom: "1.5rem" }}>
              <p
                style={{
                  fontSize: "0.875rem",
                  color: "var(--text-secondary)",
                  marginBottom: "0.5rem",
                }}
              >
                The new version is available via {platform.pkgMgrName}. Copy
                the command below and execute it in your terminal.
              </p>
              <div style={{ borderRadius: "8px", overflow: "hidden" }}>
                <CodeBlock language="bash" value={getUpgradeCommand()} />
              </div>
            </div>
          )}

          {hasSections ? (
            <>
              {SECTION_ORDER.map((sectionTitle) => (
                <UpdateSection
                  key={sectionTitle}
                  title={sectionTitle}
                  items={sections[sectionTitle]}
                  defaultOpen={sectionTitle === "New Features"}
                />
              ))}

              {Object.keys(sections)
                .filter((sectionTitle) => !SECTION_ORDER.includes(sectionTitle))
                .map((sectionTitle) => (
                  <UpdateSection
                    key={sectionTitle}
                    title={sectionTitle}
                    items={sections[sectionTitle]}
                  />
                ))}
            </>
          ) : (
            <div className={styles.section}>
              <div className={styles.sectionHeader}>
                <span className={styles.sectionTitle}>Release Notes</span>
              </div>
              <div className={styles.sectionContent}>
                <pre
                  style={{
                    whiteSpace: "pre-wrap",
                    fontFamily: "inherit",
                    margin: 0,
                  }}
                >
                  {update.notes}
                </pre>
              </div>
            </div>
          )}
        </div>

        <div className={styles.footer}>
          <div className={styles.footerLeft}>
            <div className={styles.versionBadge}>
              <DownloadCloud size={22} className={styles.downloadIcon} />
              <div className={styles.versionInfo}>
                <span className={styles.sizeLabel}>
                  {update.size
                    ? `${update.size} will be downloaded`
                    : "Unknown Size"}
                </span>
              </div>
            </div>
          </div>
          <div className={styles.footerRight}>
            <button
              type="button"
              className={styles.updateButton}
              onClick={handleUpdate}
            >
              {isTauri ? "Update Now" : "I've Upgraded"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
