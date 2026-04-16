/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from "react";
import { Download, Check, Loader2, X } from "lucide-react";
import { OcrCircularArcIcon } from "@/components/icons";
import { useModelsStore } from "../ocr-models.store";
import { Dialog } from "@/components/ui";
import { DEFAULT_OCR_MODEL_ID } from "@squigit/core/config";
import { getErrorDialog } from "@squigit/core/helpers";
import styles from "./OCRModelDownloader.module.css";

const CircularProgress: React.FC<{ progress: number }> = ({ progress }) => (
  <div className={styles.circularProgress}>
    <OcrCircularArcIcon
      className={styles.circularChart}
      trackClassName={styles.circleBg}
      arcClassName={styles.circle}
      strokeDasharray={`${progress}, 100`}
    />
  </div>
);

const CircularSpinner: React.FC<{
  paused?: boolean;
  progressHint?: number;
}> = ({ paused = false, progressHint = 0 }) => {
  const dashHead = Math.max(12, Math.min(36, Math.round(progressHint * 0.35)));
  return (
    <div
      className={`${styles.circularProgress} ${paused ? styles.spinnerPaused : ""}`}
    >
      <OcrCircularArcIcon
        className={`${styles.circularChart} ${styles.spinnerOrbit}`}
        trackClassName={styles.circleBg}
        arcClassName={styles.spinnerArc}
        strokeDasharray={`${dashHead}, 100`}
      />
    </div>
  );
};

export const OCRModelDownloader: React.FC = () => {
  const models = useModelsStore((s) => s.models);
  const startDownload = useModelsStore((s) => s.startDownload);
  const cancelDownload = useModelsStore((s) => s.cancelDownload);
  const isLoading = useModelsStore((s) => s.isLoading);
  const [error, setError] = useState<string | null>(null);
  const [hoveredModelId, setHoveredModelId] = useState<string | null>(null);
  const [justStartedDownload, setJustStartedDownload] = useState<string | null>(
    null,
  );

  const handleDownload = async (id: string) => {
    try {
      setJustStartedDownload(id);
      await startDownload(id);
    } catch (error: any) {
      const msg = error.message || error.toString();
      if (msg.toLowerCase().includes("cancelled")) {
        console.log("Download cancelled by user (silent)");
        return;
      }
      console.error("Download failed from component", error);
      setError(msg || "Failed to download model");
    }
  };

  const handleCancel = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    await cancelDownload(id);
  };

  const downloadableModels = models.filter(
    (m) => m.id !== DEFAULT_OCR_MODEL_ID,
  );

  return (
    <div className={styles.downloadSection}>
      <div className={styles.downloadHeader}>
        <div className={styles.downloadTitle}>Get more models</div>
        <div className={styles.downloadDesc}>
          Download additional OCR language models
        </div>
      </div>

      <div className={styles.modelList}>
        {isLoading && <p>Loading model list...</p>}
        {!isLoading &&
          downloadableModels.map((model) => (
            <div key={model.id} className={styles.modelRow}>
              <div className={styles.modelRowInfo}>
                <span className={styles.modelName}>{model.name}</span>
                <span className={styles.modelSize}>{model.size}</span>
              </div>
              <button
                className={`${styles.downloadButton} ${
                  ["downloading", "checking", "paused"].includes(model.state)
                    ? styles.downloading
                    : ""
                }`}
                onClick={(e) => {
                  if (
                    ["downloading", "checking", "paused"].includes(model.state)
                  ) {
                    handleCancel(e, model.id);
                  } else {
                    handleDownload(model.id);
                  }
                }}
                disabled={model.state === "extracting"}
                onMouseEnter={() => setHoveredModelId(model.id)}
                onMouseLeave={() => {
                  setHoveredModelId(null);
                  if (justStartedDownload === model.id) {
                    setJustStartedDownload(null);
                  }
                }}
                title={
                  model.state === "downloaded"
                    ? "Installed"
                    : ["downloading", "checking", "paused"].includes(
                          model.state,
                        )
                      ? "Cancel Download"
                      : `Download ${model.name}`
                }
              >
                {model.state === "idle" && <Download size={16} />}

                {model.state === "checking" &&
                  (hoveredModelId === model.id &&
                  justStartedDownload !== model.id ? (
                    <X size={16} />
                  ) : (
                    <CircularSpinner progressHint={model.progress || 0} />
                  ))}

                {model.state === "downloading" &&
                  (hoveredModelId === model.id &&
                  justStartedDownload !== model.id ? (
                    <X size={16} />
                  ) : (model.progress || 0) <= 0 ? (
                    <CircularSpinner progressHint={model.progress || 0} />
                  ) : (
                    <CircularProgress progress={model.progress || 0} />
                  ))}

                {model.state === "paused" &&
                  (hoveredModelId === model.id &&
                  justStartedDownload !== model.id ? (
                    <X size={16} />
                  ) : (
                    <CircularSpinner
                      paused
                      progressHint={model.progress || 0}
                    />
                  ))}

                {model.state === "extracting" && (
                  <Loader2 size={16} className={styles.spin} />
                )}

                {model.state === "downloaded" && <Check size={16} />}
              </button>
            </div>
          ))}
      </div>

      <Dialog
        isOpen={!!error}
        type={getErrorDialog(error || "")}
        onAction={() => setError(null)}
      />
    </div>
  );
};
