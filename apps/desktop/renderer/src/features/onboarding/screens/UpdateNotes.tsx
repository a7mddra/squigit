/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useState } from "react";
import { OnboardingLayout } from "../OnboardingLayout";
import { useAppContext } from "@/providers/AppProvider";
import { getPendingUpdate } from "@/hooks";
import { updateIcon } from "@/assets";
import { ChevronRight, DownloadCloud } from "lucide-react";
import { clsx } from "clsx";
import styles from "./UpdateNotes.module.css";

interface UpdateSectionProps {
  title: string;
  items: string[];
  defaultOpen?: boolean;
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

export const UpdateNotes: React.FC = () => {
  const app = useAppContext();
  const update = useMemo(() => getPendingUpdate(), []);

  const SECTION_ORDER = ["New Features", "Bug Fixes", "UI Improvements"];

  if (!update) {
    return null;
  }

  const sections = update.sections || {};
  const hasSections = Object.keys(sections).length > 0;

  return (
    <OnboardingLayout
      contentClassName={`${styles.container} ${styles.appOverride}`}
    >
      <div className={styles.header}>
        <div className={styles.title}>
          <img
            src={updateIcon}
            alt=""
            aria-hidden="true"
            className={styles.titleIcon}
          />
          <span>{app.system.appName}</span>
        </div>
        <div className={styles.subtitle}>v{update.version} is Here!</div>
      </div>

      <div className={styles.scrollableContent}>
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
              .filter((k) => !SECTION_ORDER.includes(k))
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
                  ? update.size + " will be downloaded"
                  : "Unknown Size"}
              </span>
            </div>
          </div>
        </div>
        <div className={styles.footerRight}>
          <button
            className={styles.updateButton}
            onClick={() => app.handleSystemAction("update_now")}
          >
            Update Now
          </button>
        </div>
      </div>
    </OnboardingLayout>
  );
};
