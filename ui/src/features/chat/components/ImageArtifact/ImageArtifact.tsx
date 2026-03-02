/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useState,
  useRef,
  useEffect,
  useCallback,
  RefObject,
} from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import {
  ImageToolbar,
  ImageSearchInput,
  OCRMenu,
  type OCRMenuHandle,
  OCRTextCanvas,
  useTextSelection,
  type SettingsSection,
} from "@/features";
import styles from "./ImageArtifact.module.css";
import { Dialog } from "@/components";
import {
  type DialogContent,
  AUTO_OCR_DISABLED_MODEL_ID,
  DEFAULT_OCR_MODEL_ID,
  getErrorDialog,
  OcrFrame,
  cancelOcrJob,
  saveOcrData,
  useGoogleLens,
  generateTranslateUrl,
  resolveOcrModelId,
} from "@/lib";

interface OCRBox {
  text: string;
  box_coords: number[][];
  confidence?: number;
}

export interface ImageArtifactProps {
  startupImage: {
    path: string;
    mimeType: string;
    imageId: string;
  } | null;
  sessionLensUrl: string | null;
  setSessionLensUrl: (url: string) => void;
  chatTitle: string;
  onDescribeEdits: (description: string) => void;
  isVisible: boolean;
  scrollContainerRef?: RefObject<HTMLDivElement | null>;

  ocrData: OcrFrame;
  onUpdateOCRData: (
    modelId: string,
    data: { text: string; box: number[][] }[],
  ) => void;
  chatId: string | null;
  inputValue: string;
  onInputChange: (value: string) => void;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  onOpenSettings: (section: SettingsSection) => void;
  autoExpandOCR?: boolean;
  activeProfileId: string | null;

  ocrEnabled: boolean;
  currentOcrModel: string;
  onOcrModelChange: (model: string) => void;

  isOcrScanning?: boolean;
  onOcrScanningChange?: (scanning: boolean) => void;
  isNavigating?: boolean;
}

const globalScanLock = new Set<string>();

export const ImageArtifact: React.FC<ImageArtifactProps> = ({
  startupImage,
  sessionLensUrl,
  setSessionLensUrl,
  isVisible,
  ocrData,
  onUpdateOCRData,
  onOpenSettings,
  chatId,
  inputValue,
  onInputChange,
  isExpanded = false,
  onToggleExpand,
  ocrEnabled,
  autoExpandOCR,
  activeProfileId,
  currentOcrModel,
  onOcrModelChange,
  isOcrScanning,
  onOcrScanningChange,
  isNavigating = false,
}) => {
  const [localLoading, setLocalLoading] = useState(false);
  const loading = isOcrScanning ?? localLoading;
  const setLoading = onOcrScanningChange ?? setLocalLoading;

  const [errorDialog, setErrorDialog] = useState<DialogContent | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [showScrollbar, setShowScrollbar] = useState(false);

  const scanRequestRef = useRef(0);

  const cancelledRef = useRef(false);

  const viewerRef = useRef<HTMLDivElement>(null);
  const imgWrapRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);

  const toolbarRef = useRef<HTMLDivElement>(null);
  const expandedContentRef = useRef<HTMLDivElement>(null);
  const OCRMenuRef = useRef<OCRMenuHandle>(null);

  const imageSrc = startupImage?.path ? convertFileSrc(startupImage.path) : "";

  const { isLensLoading, triggerLens, showAuthDialog, setShowAuthDialog } =
    useGoogleLens(
      startupImage,
      sessionLensUrl,
      setSessionLensUrl,
      activeProfileId,
    );

  const currentModelData = ocrData[currentOcrModel] || [];

  const displayData = currentModelData.map((d) => ({
    text: d.text,
    box: d.bbox,
  }));
  const lastTranslateDisabledRef = useRef(displayData.length === 0);
  useEffect(() => {
    if (!isNavigating) {
      lastTranslateDisabledRef.current = displayData.length === 0;
    }
  }, [displayData.length, isNavigating]);
  const isTranslateDisabled = isNavigating
    ? lastTranslateDisabledRef.current
    : displayData.length === 0;

  const autoOcrDisabledForChat = Array.isArray(
    ocrData[AUTO_OCR_DISABLED_MODEL_ID],
  );

  const { svgRef, handleTextMouseDown } = useTextSelection({
    data: displayData,
    onSelectionComplete: (selection: Selection) => {
      OCRMenuRef.current?.showStandardMenu(selection);
    },
  });

  const hasScannedRef = useRef(false);
  const prevImageBase64Ref = useRef<string | null>(null);

  const currentPath = startupImage?.path ?? null;
  useEffect(() => {
    if (currentPath !== prevImageBase64Ref.current) {
      scanRequestRef.current += 1;
      cancelOcrJob();
      hasScannedRef.current = false;
      prevImageBase64Ref.current = currentPath;
      setLoading(false);
      cancelledRef.current = false;
    }
  }, [currentPath, setLoading]);

  const scan = useCallback(
    async (
      modelId?: string,
      requestId?: number,
      options?: { manual?: boolean; force?: boolean },
    ) => {
      if (!startupImage?.path) return;

      const currentChatId = chatId;
      const modelToUse = modelId || currentOcrModel;
      if (!modelToUse) return;

      if (!options?.manual && !ocrEnabled) {
        return;
      }

      if (ocrData[modelToUse] && !options?.force) {
        console.log(
          `[ImageArtifact] Data already exists for model: ${modelToUse}`,
        );
        return;
      }

      const lockKey = `${startupImage.imageId}-${modelToUse}`;
      if (globalScanLock.has(lockKey)) {
        console.log(`[ImageArtifact] Scan already in progress for: ${lockKey}`);
        return;
      }

      globalScanLock.add(lockKey);

      console.log(
        `[ImageArtifact] Scanning with model: ${modelToUse} (Request ID: ${requestId})`,
      );
      setLoading(true);
      setErrorDialog(null);

      cancelledRef.current = false;

      try {
        const casRelativePath =
          startupImage.imageId && startupImage.imageId.length >= 2
            ? `objects/${startupImage.imageId.slice(0, 2)}/${startupImage.imageId}.png`
            : startupImage.path;

        const results = await invoke<OCRBox[]>("ocr_image", {
          imageData: casRelativePath,
          isBase64: false,
          modelName: modelToUse,
        });

        if (requestId !== undefined && requestId !== scanRequestRef.current) {
          console.log(
            `[ImageArtifact] Ignoring result from old request ID: ${requestId} (Current: ${scanRequestRef.current})`,
          );
          return;
        }

        if (cancelledRef.current) {
          console.log(
            "[ImageArtifact] Scan result ignored due to manual cancellation",
          );
          return;
        }

        if (currentChatId === chatId) {
          const converted = results.map((r) => ({
            text: r.text,
            box: r.box_coords,
          }));
          console.log(`[ImageArtifact] Updating OCR data for ${modelToUse}`);
          onUpdateOCRData(modelToUse, converted);

          if (
            autoExpandOCR &&
            onToggleExpand &&
            !hasScannedRef.current &&
            !isExpanded
          ) {
            onToggleExpand();
          }
          hasScannedRef.current = true;
        }
      } catch (e: any) {
        if (requestId !== undefined && requestId !== scanRequestRef.current) {
          return;
        }

        if (
          cancelledRef.current ||
          e.toString().includes("cancelled") ||
          e.toString().includes("Download Cancelled")
        ) {
          return;
        }

        if (currentChatId === chatId) {
          setErrorDialog(getErrorDialog(e.toString()));
        }
      } finally {
        globalScanLock.delete(`${startupImage.imageId}-${modelToUse}`);

        if (currentChatId === chatId && !cancelledRef.current) {
          if (requestId === undefined || requestId === scanRequestRef.current) {
            setLoading(false);
          }
        }
      }
    },
    [
      startupImage,
      chatId,
      ocrEnabled,
      autoExpandOCR,
      onToggleExpand,
      isExpanded,
      onUpdateOCRData,
      currentOcrModel,
      ocrData,
    ],
  );

  const restartScanForModel = useCallback(
    (modelId: string, force = false) => {
      scanRequestRef.current += 1;
      const newRequestId = scanRequestRef.current;

      setLoading(false);
      cancelledRef.current = false;

      if (!modelId) return;

      void (async () => {
        await cancelOcrJob();
        if (newRequestId === scanRequestRef.current) {
          scan(modelId, newRequestId, { manual: true, force });
        }
      })();
    },
    [scan, setLoading],
  );

  const handleUserOcrModelChange = useCallback(
    (model: string) => {
      const resolvedModel = resolveOcrModelId(
        model,
        currentOcrModel || DEFAULT_OCR_MODEL_ID,
      );
      if (!resolvedModel) return;

      if (resolvedModel !== currentOcrModel) {
        onOcrModelChange(resolvedModel);
      }

      const hasCachedData = Array.isArray(ocrData[resolvedModel]);
      if (!hasCachedData) {
        restartScanForModel(resolvedModel, false);
      }
    },
    [currentOcrModel, onOcrModelChange, ocrData, restartScanForModel],
  );

  const handleCancel = async (e: React.MouseEvent) => {
    e.stopPropagation();
    console.log("Cancelling OCR job...");
    cancelledRef.current = true;

    scanRequestRef.current += 1;
    hasScannedRef.current = false;
    setLoading(false);
    onOcrModelChange("");
    await cancelOcrJob();
    if (chatId) {
      saveOcrData(chatId, AUTO_OCR_DISABLED_MODEL_ID, []).catch((err) =>
        console.error("Failed to persist OCR auto-run opt-out:", err),
      );
    }
  };

  useEffect(() => {
    if (
      startupImage &&
      ocrEnabled &&
      currentOcrModel &&
      !ocrData[currentOcrModel] &&
      !autoOcrDisabledForChat &&
      !loading &&
      !errorDialog &&
      !hasScannedRef.current &&
      !cancelledRef.current
    ) {
      scan(currentOcrModel, scanRequestRef.current);
    }
  }, [
    startupImage,
    ocrData,
    autoOcrDisabledForChat,
    loading,
    errorDialog,
    scan,
    currentOcrModel,
    ocrEnabled,
  ]);

  const containerRef = useRef<HTMLDivElement>(null);
  const scrollWrapperRef = useRef<HTMLDivElement>(null);

  const onLoad = () => {
    if (imgRef.current) {
      setSize({
        w: imgRef.current.naturalWidth,
        h: imgRef.current.naturalHeight,
      });
    }
  };

  const handleCopyImage = useCallback(async (): Promise<boolean> => {
    if (!startupImage?.path) return false;

    try {
      await invoke("copy_image_from_path_to_clipboard", {
        path: startupImage.path,
      });
      return true;
    } catch (err) {
      setErrorDialog(
        getErrorDialog(
          `Failed to copy image to clipboard: ${err instanceof Error ? err.message : String(err)}`,
        ),
      );
      return false;
    }
  }, [startupImage]);

  const handleExpandSave = useCallback(async () => {
    if (!startupImage?.path) return;

    try {
      const filePath = await save({
        title: "Save Image As",
        filters: [
          { name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] },
        ],
      });

      if (filePath) {
        await invoke("copy_image_to_path", {
          sourcePath: startupImage.path,
          targetPath: filePath,
        });
      }
    } catch (err) {
      setErrorDialog(
        getErrorDialog(
          `Failed to save image: ${err instanceof Error ? err.message : String(err)}`,
        ),
      );
    }
  }, [startupImage]);

  const toggleExpand = () => {
    if (onToggleExpand) {
      onToggleExpand();
    }
  };

  const handleTranslateAll = useCallback(() => {
    if (displayData.length === 0) return;
    const allText = displayData.map((item) => item.text).join(" ");
    if (allText.trim()) {
      invoke("open_external_url", { url: generateTranslateUrl(allText) });
    }
  }, [displayData]);

  const [isAnimating, setIsAnimating] = useState(false);

  useEffect(() => {
    setIsAnimating(true);
    const timer = setTimeout(() => {
      setIsAnimating(false);
    }, 300);
    return () => clearTimeout(timer);
  }, [isExpanded]);

  useEffect(() => {
    const checkOverflow = () => {
      const el = scrollWrapperRef.current;
      if (el) {
        const hasOverflow = el.scrollHeight > el.clientHeight;
        setShowScrollbar(isExpanded && hasOverflow);
      }
    };

    if (isExpanded) {
      if (!isAnimating) {
        checkOverflow();
      }
      window.addEventListener("resize", checkOverflow);
    } else {
      setShowScrollbar(false);
    }

    return () => window.removeEventListener("resize", checkOverflow);
  }, [isExpanded, size, isAnimating]);

  const isExpandedRef = useRef(isExpanded);
  isExpandedRef.current = isExpanded;

  useEffect(() => {
    const imageBox = scrollWrapperRef.current;
    if (!imageBox) return;

    const handleWheel = (e: WheelEvent) => {
      if (!isExpandedRef.current) return;

      const isScrollable =
        Math.ceil(imageBox.scrollHeight) > Math.ceil(imageBox.clientHeight);

      if (isScrollable) {
        e.stopPropagation();
      }
    };

    imageBox.addEventListener("wheel", handleWheel, { passive: false });
    return () => imageBox.removeEventListener("wheel", handleWheel);
  }, []);

  if (!startupImage) {
    return null;
  }

  if (!isVisible) return null;

  return (
    <>
      <div
        ref={containerRef}
        className={`${styles.floatingContainer} ${isExpanded ? styles.expanded : ""}`}
      >
        <div className={styles.barHeader}>
          <div
            className={`${styles.thumbnailWrapper} ${loading ? styles.thumbnailLoading : ""} ${isExpanded ? styles.thumbnailExpanded : ""}`}
            onClick={loading || isExpanded ? undefined : toggleExpand}
            title={loading ? "Processing..." : undefined}
          >
            <img src={imageSrc} alt="Thumbnail" className={styles.miniThumb} />
            {loading && (
              <div
                className={styles.cancelButton}
                onClick={handleCancel}
                title="Cancel OCR"
              >
                <svg
                  className={styles.cancelIcon}
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="3"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </div>
            )}
          </div>

          <div className={styles.inputContainer}>
            <ImageSearchInput
              value={inputValue}
              onChange={onInputChange}
              onLensClick={(query) => triggerLens(query)}
              onTranslateClick={handleTranslateAll}
              onCollapse={toggleExpand}
              isLensLoading={isLensLoading}
              isTranslateDisabled={isTranslateDisabled}
              isOCRLoading={loading}
              isExpanded={isExpanded}
              placeholder="Add to your search"
              currentOcrModel={currentOcrModel}
              onOcrModelChange={handleUserOcrModelChange}
              onOpenSettings={onOpenSettings}
            />
          </div>
        </div>

        <div className={styles.expandedContent} ref={expandedContentRef}>
          <div
            className={`${styles.bigImageBox} ${showScrollbar ? styles.showScrollbar : ""}`}
            ref={scrollWrapperRef}
            style={isAnimating ? { overflow: "hidden" } : undefined}
          >
            <div className={styles.viewer} ref={viewerRef}>
              <div className={styles.imageWrap}>
                <div className={styles.innerContent} ref={imgWrapRef}>
                  <img
                    ref={imgRef}
                    src={imageSrc}
                    alt=""
                    onLoad={onLoad}
                    onError={() =>
                      setErrorDialog(getErrorDialog("Failed to load image"))
                    }
                    draggable={false}
                    className={styles.bigImage}
                  />
                  {displayData.length > 0 && (
                    <OCRTextCanvas
                      data={displayData}
                      size={size}
                      svgRef={svgRef}
                      onTextMouseDown={handleTextMouseDown}
                    />
                  )}
                </div>
              </div>
            </div>
          </div>

          <ImageToolbar
            key={chatId}
            toolbarRef={toolbarRef}
            isLensLoading={isLensLoading}
            onLensClick={triggerLens}
            onCopyImage={handleCopyImage}
            onSaveClick={handleExpandSave}
            constraintRef={scrollWrapperRef}
            isExpanded={isExpanded}
          />
        </div>
      </div>

      <OCRMenu
        ref={OCRMenuRef}
        data={displayData}
        size={size}
        imgRef={imgRef}
        imgWrapRef={imgWrapRef}
        viewerRef={viewerRef}
      />

      <Dialog
        isOpen={showAuthDialog}
        type="IMGBB_AUTH"
        onAction={(key: string) => {
          if (key === "confirm") {
            onOpenSettings("apikeys");
          }
          setShowAuthDialog(false);
        }}
      />

      {errorDialog && (
        <Dialog
          isOpen={!!errorDialog}
          type={errorDialog}
          onAction={() => setErrorDialog(null)}
        />
      )}
    </>
  );
};
