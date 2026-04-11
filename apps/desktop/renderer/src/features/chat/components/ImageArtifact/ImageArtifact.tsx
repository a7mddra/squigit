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
import { CloseCrossIcon } from "@/assets";
import {
  ImageToolbar,
  ImageSearchInput,
  OCRMenu,
  type OCRMenuHandle,
  OCRTextCanvas,
  useTextSelection,
  type SettingsSection,
} from "@/features";
import { usePlatform } from "@/hooks/core/usePlatform";
import styles from "./ImageArtifact.module.css";
import { Dialog } from "@/components";
import {
  type DialogContent,
  AUTO_OCR_DISABLED_MODEL_ID,
  DEFAULT_OCR_MODEL_ID,
  getErrorDialog,
  getMissingPackageDialog,
  getOutdatedPackageDialog,
  OcrFrame,
  cancelOcrJob,
  saveOcrData,
  useGoogleLens,
  generateTranslateUrl,
  resolveOcrModelId,
  saveImageTone,
} from "@/core";

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
    tone?: string;
  } | null;
  sessionLensUrl: string | null;
  setSessionLensUrl: (url: string) => void;
  chatTitle: string;
  onDescribeEdits: (description: string) => void;
  isVisible: boolean;
  scrollContainerRef?: RefObject<HTMLDivElement | null>;

  ocrData: OcrFrame;
  onUpdateOCRData: (
    chatId: string | null,
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
type ImageToneMode = "dark" | "light";

const normalizeToneResult = (value: string): ImageToneMode => {
  const normalized = value.trim().toLowerCase();
  if (normalized === "light" || normalized === "l") {
    return "light";
  }
  return "dark";
};

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
  const [displayedThumb, setDisplayedThumb] = useState<{
    path: string;
    src: string;
  } | null>(null);
  const [imageToneMode, setImageToneMode] = useState<ImageToneMode>(
    (startupImage?.tone as ImageToneMode) || "dark",
  );

  useEffect(() => {
    if (startupImage?.tone) {
      setImageToneMode(startupImage.tone as ImageToneMode);
    }
  }, [startupImage?.tone]);

  useEffect(() => {
    let cancelled = false;

    const runToneDetection = async () => {
      if (!startupImage?.path || startupImage?.tone || isNavigating) {
        return;
      }

      let attempt = 0;
      while (attempt < 3 && !cancelled) {
        try {
          const raw = await invoke<string>("detect_image_tone", {
            path: startupImage.path,
          });
          const tone = normalizeToneResult(raw);

          if (!cancelled) {
            setImageToneMode(tone);
            if (chatId) {
              saveImageTone(chatId, tone).catch(console.error);
            }
            console.log(
              `[ToneDetector] Attempt ${attempt + 1}: Success, detected ${tone}`,
            );
            return;
          }
        } catch (err) {
          console.error(
            `[ToneDetector] Attempt ${attempt + 1} failed for image=${startupImage.imageId}:`,
            err,
          );
          attempt++;
          if (attempt < 3 && !cancelled) {
            await new Promise((r) =>
              setTimeout(r, 500 * Math.pow(2, attempt - 1)),
            );
          }
        }
      }
      console.warn(
        `[ToneDetector] All retries failed for image=${startupImage.imageId}`,
      );
    };

    void runToneDetection();

    return () => {
      cancelled = true;
    };
  }, [
    isNavigating,
    startupImage?.imageId,
    startupImage?.path,
    startupImage?.tone,
  ]);

  const scanRequestRef = useRef(0);

  const cancelledRef = useRef(false);

  const viewerRef = useRef<HTMLDivElement>(null);
  const imgWrapRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);

  const toolbarRef = useRef<HTMLDivElement>(null);
  const expandedContentRef = useRef<HTMLDivElement>(null);
  const OCRMenuRef = useRef<OCRMenuHandle>(null);

  const imageSrc = startupImage?.path ? convertFileSrc(startupImage.path) : "";
  const displayedThumbPath = displayedThumb?.path ?? null;
  const displayedThumbSrc = displayedThumb?.src ?? null;

  useEffect(() => {
    if (!startupImage?.path || !imageSrc) {
      setDisplayedThumb(null);
      return;
    }

    if (displayedThumbPath === startupImage.path) {
      return;
    }

    let cancelled = false;
    const preloadImage = new window.Image();

    const commitThumb = () => {
      if (cancelled) {
        return;
      }

      setDisplayedThumb({
        path: startupImage.path,
        src: imageSrc,
      });
    };

    preloadImage.onload = commitThumb;
    preloadImage.onerror = commitThumb;
    preloadImage.src = imageSrc;

    return () => {
      cancelled = true;
      preloadImage.onload = null;
      preloadImage.onerror = null;
    };
  }, [displayedThumbPath, imageSrc, startupImage?.path]);

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
  const latestOcrModelRef = useRef(currentOcrModel);
  const latestContextRef = useRef<{
    chatId: string | null;
    imageId: string | null;
    imagePath: string | null;
  }>({
    chatId,
    imageId: startupImage?.imageId ?? null,
    imagePath: startupImage?.path ?? null,
  });
  const prevScanContextRef = useRef<string | null>(null);

  useEffect(() => {
    latestContextRef.current = {
      chatId,
      imageId: startupImage?.imageId ?? null,
      imagePath: startupImage?.path ?? null,
    };
  }, [chatId, startupImage?.imageId, startupImage?.path]);
  useEffect(() => {
    latestOcrModelRef.current = currentOcrModel;
  }, [currentOcrModel]);

  const isScanContextCurrent = useCallback(
    (
      context: { chatId: string | null; imageId: string; imagePath: string },
      requestId?: number,
    ) => {
      if (requestId !== undefined && requestId !== scanRequestRef.current) {
        return false;
      }
      if (cancelledRef.current) {
        return false;
      }
      const latest = latestContextRef.current;
      return (
        latest.chatId === context.chatId &&
        latest.imageId === context.imageId &&
        latest.imagePath === context.imagePath
      );
    },
    [],
  );

  const scanContextKey = `${chatId ?? "__none__"}::${startupImage?.imageId ?? "__none__"}::${startupImage?.path ?? "__none__"}`;
  useEffect(() => {
    if (scanContextKey !== prevScanContextRef.current) {
      scanRequestRef.current += 1;
      cancelOcrJob();
      hasScannedRef.current = false;
      prevScanContextRef.current = scanContextKey;
      setLoading(false);
      cancelledRef.current = false;
    }
  }, [scanContextKey, setLoading]);

  const scan = useCallback(
    async (
      modelId?: string,
      requestId?: number,
      options?: { manual?: boolean; force?: boolean },
    ) => {
      if (!startupImage?.path) return;
      const modelToUse = modelId || currentOcrModel;
      if (!modelToUse) return;
      const scanContext = {
        chatId,
        imageId: startupImage.imageId,
        imagePath: startupImage.path,
      };

      if (!options?.manual && !ocrEnabled) {
        return;
      }

      if (ocrData[modelToUse] && !options?.force) {
        console.log(
          `[ImageArtifact] Data already exists for model: ${modelToUse}`,
        );
        return;
      }

      const lockKey = `${scanContext.chatId ?? "__no_chat__"}-${scanContext.imageId}-${modelToUse}`;
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

        if (!isScanContextCurrent(scanContext, requestId)) {
          console.log(
            `[ImageArtifact] Ignoring stale OCR result for ${modelToUse}`,
          );
          return;
        }

        const converted = results.map((r) => ({
          text: r.text,
          box: r.box_coords,
        }));
        console.log(`[ImageArtifact] Updating OCR data for ${modelToUse}`);
        onUpdateOCRData(scanContext.chatId, modelToUse, converted);

        if (
          autoExpandOCR &&
          onToggleExpand &&
          !hasScannedRef.current &&
          !isExpanded &&
          !isNavigating &&
          !loading
        ) {
          onToggleExpand();
        }
        hasScannedRef.current = true;
      } catch (e: any) {
        const errorText = e instanceof Error ? e.message : String(e);
        if (!isScanContextCurrent(scanContext, requestId)) {
          return;
        }

        if (
          errorText.toLowerCase().includes("cancelled") ||
          errorText.includes("Download Cancelled")
        ) {
          return;
        }

        if (errorText.includes("ERR_MISSING_OCR_PACKAGE")) {
          setErrorDialog(
            getMissingPackageDialog(
              "squigit-ocr",
              platform.getPkgInstallCmd("squigit-ocr"),
            ),
          );
        } else if (errorText.includes("ERR_OUTDATED_OCR_PACKAGE")) {
          setErrorDialog(getOutdatedPackageDialog("squigit-ocr"));
        } else {
          setErrorDialog(getErrorDialog(errorText));
        }

        if (latestOcrModelRef.current === modelToUse) {
          onOcrModelChange("");
        }
      } finally {
        globalScanLock.delete(lockKey);

        if (isScanContextCurrent(scanContext, requestId)) {
          setLoading(false);
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
      isNavigating,
      loading,
      onUpdateOCRData,
      currentOcrModel,
      onOcrModelChange,
      ocrData,
      isScanContextCurrent,
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
      !isNavigating &&
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
    isNavigating,
    ocrEnabled,
  ]);

  const platform = usePlatform();
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

  const isThumbLoaded =
    !!startupImage?.path && displayedThumbPath === startupImage.path;
  const isExpandLocked = isNavigating || loading || !isThumbLoaded;
  const isArtifactExpanded = isExpanded && !isExpandLocked;
  const showThumbnailSkeleton = !displayedThumbSrc;
  const showThumbnailLoadingState = showThumbnailSkeleton;

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
    if (isExpandLocked) {
      return;
    }
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

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;

    const checkOverflow = () => {
      const el = scrollWrapperRef.current;
      if (el) {
        const hasOverflow = el.scrollHeight > el.clientHeight;
        setShowScrollbar(isArtifactExpanded && hasOverflow);
      }
    };

    if (isArtifactExpanded) {
      timeoutId = setTimeout(checkOverflow, 260);
      window.addEventListener("resize", checkOverflow);
    } else {
      setShowScrollbar(false);
    }

    return () => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
      window.removeEventListener("resize", checkOverflow);
    };
  }, [isArtifactExpanded, size]);

  const isExpandedRef = useRef(isExpanded);
  isExpandedRef.current = isArtifactExpanded;

  useEffect(() => {
    if (isExpandLocked && isExpanded && onToggleExpand) {
      onToggleExpand();
    }
  }, [isExpandLocked, isExpanded, onToggleExpand]);

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
        className={`${styles.floatingContainer} ${isArtifactExpanded ? styles.expanded : ""}`}
      >
        <div className={styles.barHeader}>
          <div
            className={`${styles.thumbnailWrapper} ${showThumbnailLoadingState ? styles.thumbnailLoading : ""} ${isArtifactExpanded ? styles.thumbnailExpanded : ""}`}
            onClick={
              isExpandLocked || isArtifactExpanded ? undefined : toggleExpand
            }
            title={isExpandLocked ? "Loading..." : undefined}
          >
            <img
              src={displayedThumbSrc ?? undefined}
              alt="Thumbnail"
              className={`${styles.miniThumb} ${showThumbnailSkeleton ? styles.miniThumbHidden : ""}`}
            />
            {showThumbnailSkeleton && (
              <div className={styles.miniThumbSkeleton} aria-hidden="true" />
            )}
            {loading && (
              <div
                className={styles.cancelButton}
                onClick={handleCancel}
                title="Cancel OCR"
              >
                <CloseCrossIcon
                  size={24}
                  className={styles.cancelIcon}
                  ariaHidden
                />
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
              isExpanded={isArtifactExpanded}
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
                      imageToneMode={imageToneMode}
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
            isExpanded={isArtifactExpanded}
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
