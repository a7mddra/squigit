/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion, getTauriVersion } from "@tauri-apps/api/app";
import { github } from "@/lib";
import { prepareGitHubIssueReport, prepareMailReport } from "@/lib";
import { MarkGithubIcon, MailIcon, BugIcon } from "@primer/octicons-react";
import { CodeBlock } from "@/components";
import { useAppContext } from "@/providers/AppProvider";
import styles from "./HelpSection.module.css";

export const HelpSection: React.FC = () => {
  const app = useAppContext();
  const [sysInfo, setSysInfo] = useState<Record<string, string>>({
    [app.system.appName]: "v1.0.0",
    Tauri: "Loading...",
    React: React.version,
    PaddlePaddle: "v2.6.0",
    Commit: "7a8657542180fb8440c8dcc20d83285fe11360ed",
    Webview: "Loading...",
  });

  useEffect(() => {
    const loadSystemData = async () => {
      try {
        const [appVer, tauriVer] = await Promise.all([
          getVersion(),
          getTauriVersion(),
        ]);

        setSysInfo((prev) => ({
          ...prev,
          [app.system.appName]: `v${appVer}`,
          Tauri: `v${tauriVer}`,
          Webview: navigator.userAgent,
        }));
      } catch (e) {
        console.error("Failed to load system specs", e);
      }
    };
    loadSystemData();
  }, []);

  const handleOpen = (url: string) => invoke("open_external_url", { url });

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error("Failed to copy system info", err);
    }
  };

  const handleContactSupport = async () => {
    const diag = JSON.stringify(sysInfo, null, 2);
    const action = prepareMailReport(app.system.appName, {
      diagnostics: diag,
    });

    if (action.didCopy && action.copyText) {
      await copyToClipboard(action.copyText);
    }

    handleOpen(action.openUrl);
  };

  const handleReportBug = async () => {
    const diag = JSON.stringify(sysInfo, null, 2);
    const action = prepareGitHubIssueReport(app.system.appName, {
      diagnostics: diag,
    });

    if (action.didCopy && action.copyText) {
      await copyToClipboard(action.copyText);
    }

    handleOpen(action.openUrl);
  };

  return (
    <section className={styles.container} aria-labelledby="help-heading">
      <header className={styles.sectionHeader}>
        <h2 id="help-heading" className={styles.sectionTitle}>
          Help & Support
        </h2>
      </header>
      <div className={styles.code}>
        <span className={styles.subLabel}>System Diagnostics</span>
        <CodeBlock
          language="json"
          value={JSON.stringify(sysInfo, null, 2)}
          stickyHeader={false}
        />
      </div>
      <div className={styles.group}>
        <div className={styles.actionRow}>
          <button
            className={styles.actionButton}
            onClick={() => handleOpen(github.repo)}
          >
            <MarkGithubIcon size={18} className={styles.actionIcon} />
            <span className={styles.actionLabel}>View Repository</span>
          </button>

          <button
            className={styles.actionButton}
            onClick={handleContactSupport}
          >
            <MailIcon size={18} className={styles.actionIcon} />
            <span className={styles.actionLabel}>Contact Support</span>
          </button>

          <button className={styles.actionButton} onClick={handleReportBug}>
            <BugIcon size={18} className={styles.actionIcon} />
            <span className={styles.actionLabel}>Report Bug</span>
          </button>
        </div>
      </div>
      <div className={styles.aboutSection}>
        <div className={styles.legalNote}>
          <p className={styles.legalNoteText}>
            Some system diagnostics information may be sent to{" "}
            {app.system.appName} when you contact support or report an issue.
            This information is used to help us troubleshoot problems and bugs,
            subject to our{" "}
            <button
              className={styles.legalLink}
              onClick={() => handleOpen(github.docs("06-policies/SECURITY.md"))}
            >
              Privacy Policy and Terms.
            </button>{" "}
            We may contact you for additional details or updates regarding your
            report.
          </p>
        </div>
        <div className={styles.divider} />
        <div className={styles.legalRow}>
          <span>{app.system.appName} © 2026</span>
          <span className={styles.dot}>•</span>
          <span>
            <button
              className={`${styles.legalLink} ${styles.license}`}
              onClick={() => handleOpen(github.license)}
            >
              Licensed under Apache 2.0
            </button>
          </span>
        </div>
      </div>
    </section>
  );
};
