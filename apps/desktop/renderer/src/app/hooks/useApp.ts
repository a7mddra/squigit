/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { exit, relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { invoke } from "@tauri-apps/api/core";
import {
  AUTO_OCR_DISABLED_MODEL_ID,
  OcrFrame,
  appendChatMessage,
  overwriteChatMessages,
} from "@squigit/core/config";
import type { Attachment } from "@squigit/core/brain/session/attachments";
import { github } from "@squigit/core/services/github";
import { resolveOcrModelId, SUPPORTED_OCR_MODEL_IDS } from "@squigit/core/config";
import {
  getPendingUpdate,
  useAuth,
  useSystemSync,
  useUpdateCheck,
} from "@/hooks/system";
import { useBrainTitle } from "@squigit/core/brain/hooks";
import { useAttachments } from "@/hooks/shared";
import { useChat, useChatHistory } from "@/features/chat";
import { useAppBusyGuard } from "./useAppBusyGuard";
import { useAppCapture } from "./useAppCapture";
import { useAppContextMenu } from "./useAppContextMenu";
import { useAppDialogs } from "./useAppDialogs";
import { useAppDrafts } from "./useAppDrafts";
import { useAppMedia } from "./useAppMedia";
import { useAppNavigation } from "./useAppNavigation";
import { useAppOcr } from "./useAppOcr";
import { useAppPanel } from "./useAppPanel";

const isOnboardingId = (id: string) => id.startsWith("__system_");

const getChatOcrModel = (
  frame: OcrFrame,
  metadataOcrLanguage?: string,
): string => {
  const hasModelData = (modelId?: string) =>
    !!modelId &&
    modelId !== AUTO_OCR_DISABLED_MODEL_ID &&
    Array.isArray(frame[modelId]);

  const resolvedMetadataModel = resolveOcrModelId(metadataOcrLanguage, "");
  if (resolvedMetadataModel && hasModelData(resolvedMetadataModel)) {
    return resolvedMetadataModel;
  }

  const scannedModelSet = new Set(
    Object.entries(frame)
      .filter(
        ([modelId, regions]) =>
          modelId !== AUTO_OCR_DISABLED_MODEL_ID && Array.isArray(regions),
      )
      .map(([modelId]) => resolveOcrModelId(modelId, ""))
      .filter((modelId) => !!modelId),
  );

  for (const modelId of SUPPORTED_OCR_MODEL_IDS) {
    if (scannedModelSet.has(modelId)) {
      return modelId;
    }
  }

  const fallbackScannedModel = Object.keys(frame)
    .filter(
      (modelId) =>
        modelId !== AUTO_OCR_DISABLED_MODEL_ID && hasModelData(modelId),
    )
    .sort()[0];

  return fallbackScannedModel
    ? resolveOcrModelId(fallbackScannedModel, "")
    : "";
};

function getAttachmentAnalysisCounts(items: Attachment[]) {
  if (items.length === 0) {
    return null;
  }

  const imageCount = items.filter(
    (attachment) => attachment.type === "image",
  ).length;
  const fileCount = items.length - imageCount;

  return imageCount > 0 || fileCount > 0 ? { imageCount, fileCount } : null;
}

export const useApp = () => {
  const system = useSystemSync();
  const auth = useAuth();

  const activeProfileRef = useRef<any>(null);
  const systemRef = useRef(system);
  const chatHistoryRef = useRef<any>(null);
  const agreedToTermsRef = useRef(false);
  const hasShownCaptureTerminalHintRef = useRef(false);

  useEffect(() => {
    activeProfileRef.current = system.activeProfile;
    systemRef.current = system;
  }, [system]);

  const [pendingUpdate] = useState(() => getPendingUpdate());
  const chatHistory = useChatHistory(system.activeProfile?.id || null);

  useEffect(() => {
    chatHistoryRef.current = chatHistory;
  }, [chatHistory]);

  useUpdateCheck();

  const performLogout = async () => {
    await system.handleLogout();
    auth.logout();
  };

  const dialogs = useAppDialogs();
  const drafts = useAppDrafts();
  const attachments = useAttachments();
  const contextMenuState = useAppContextMenu();
  const ocr = useAppOcr(chatHistory.activeSessionId);
  const [pendingPromptAttachmentAnalysis, setPendingPromptAttachmentAnalysis] =
    useState<ReturnType<typeof getAttachmentAnalysisCounts>>(null);
  const [hasAutoSelectedWelcome, setHasAutoSelectedWelcome] = useState(false);
  const [showUpdate, setShowUpdate] = useState(() => {
    const wasDismissed = sessionStorage.getItem("update_dismissed");
    return !!pendingUpdate && !wasDismissed;
  });
  const [agreedToTerms, setAgreedToTerms] = useState(false);

  useEffect(() => {
    agreedToTermsRef.current = agreedToTerms;
  }, [agreedToTerms]);

  const shouldAutoClosePanelForWelcome =
    system.hasAgreed === false && !system.activeProfile;

  const hasActiveOnboarding = chatHistory.activeSessionId
    ? isOnboardingId(chatHistory.activeSessionId)
    : false;
  const isImageMissing = !system.startupImage && !hasActiveOnboarding;
  const isAuthPending = auth.authStage === "LOGIN";
  const isPendingAutoSelectWelcome =
    system.hasAgreed === false &&
    !system.activeProfile &&
    !chatHistory.activeSessionId &&
    !hasAutoSelectedWelcome;
  const isLoadingState =
    !system.profileLoaded ||
    !system.prefsLoaded ||
    system.hasAgreed === null ||
    auth.authStage === "LOADING" ||
    isPendingAutoSelectWelcome;

  const panel = useAppPanel(isLoadingState, shouldAutoClosePanelForWelcome);

  const { isGeneratingTitle, generateTitleForText } = useBrainTitle({
    apiKey: system.apiKey,
  });

  const handleMessageAdded = useCallback(
    (msg: any, targetChatId?: string) => {
      const activeId = targetChatId || chatHistory.activeSessionId;
      if (activeId && !isOnboardingId(activeId)) {
        const role = msg.role === "user" ? "user" : "assistant";
        appendChatMessage(activeId, role, msg.text).catch(console.error);
      }
    },
    [chatHistory.activeSessionId],
  );

  const handleOverwriteMessages = useCallback(
    (msgs: any[]) => {
      const activeId = chatHistory.activeSessionId;
      if (!activeId) return;

      const formatted = msgs.map((message: any) => ({
        role: (message.role === "user" ? "user" : "assistant") as
          | "user"
          | "assistant",
        content: message.text,
        timestamp: new Date(message.timestamp).toISOString(),
        citations: Array.isArray(message.citations) ? message.citations : [],
        tool_steps: Array.isArray(message.toolSteps) ? message.toolSteps : [],
      }));

      overwriteChatMessages(activeId, formatted).catch(console.error);
    },
    [chatHistory.activeSessionId],
  );

  const chatTitle = isImageMissing
    ? system.appName
    : isGeneratingTitle
      ? "New thread"
      : system.sessionChatTitle || "New thread";

  const isChatActive = !isLoadingState && !isImageMissing && !isAuthPending;
  const chat = useChat({
    apiKey: system.apiKey,
    currentModel: system.sessionModel,
    startupImage: system.startupImage,
    prompt: system.prompt,
    setCurrentModel: system.setSessionModel,
    enabled: isChatActive,
    onMessage: handleMessageAdded,
    onOverwriteMessages: handleOverwriteMessages,
    chatId: chatHistory.activeSessionId,
    chatTitle,
    onMissingApiKey: () => {
      dialogs.setShowProviderAuthDialog(true);
    },
    onTitleGenerated: (title: string) => {
      system.setSessionChatTitle(title);
    },
    generateTitle: generateTitleForText,
    userName: system.userName,
    userEmail: system.userEmail,
  });

  const isActiveChatBusy =
    chat.isAnalyzing ||
    chat.isGenerating ||
    chat.isAiTyping ||
    ocr.isOcrScanning;

  const media = useAppMedia({ attachments: attachments.attachments });
  const busyGuard = useAppBusyGuard({
    chat,
    ocr,
    system,
    getSafeOcrModel: () => getChatOcrModel(ocr.ocrData, undefined),
  });
  const navigation = useAppNavigation({
    chat,
    chatHistory,
    ocr,
    system,
    chatTitle,
    isActiveChatBusy,
    closeMediaViewer: media.closeMediaViewer,
    runWithBusyGuard: busyGuard.runWithBusyGuard,
  });
  const capture = useAppCapture({
    system,
    auth,
    chatHistory,
    ocr,
    dialogs,
    activeProfileRef,
    systemRef,
    chatHistoryRef,
    performSelectChat: navigation.performSelectChat,
    performNewSession: navigation.performNewSession,
    closeMediaViewer: media.closeMediaViewer,
    runWithBusyGuardRef: busyGuard.runWithBusyGuardRef,
    agreedToTermsRef,
    hasShownCaptureTerminalHintRef,
  });

  const isAgreementPending = system.hasAgreed === false;

  useEffect(() => {
    const pendingTurn = chat.pendingAssistantTurn;
    if (!pendingTurn || pendingTurn.phase !== "thinking") {
      setPendingPromptAttachmentAnalysis(null);
    }
  }, [chat.pendingAssistantTurn]);

  useEffect(() => {
    if (
      system.hasAgreed === false &&
      auth.authStage !== "LOADING" &&
      !capture.isCheckingImage &&
      !system.activeProfile &&
      !chatHistory.activeSessionId &&
      !hasAutoSelectedWelcome
    ) {
      navigation.performSelectChat("__system_welcome");
      setHasAutoSelectedWelcome(true);
    }
  }, [
    auth.authStage,
    capture.isCheckingImage,
    chatHistory.activeSessionId,
    hasAutoSelectedWelcome,
    navigation,
    system.activeProfile,
    system.hasAgreed,
  ]);

  const handleUpdateLensUrl = useCallback(
    (url: string | null) => {
      ocr.handleUpdateLensUrl(url);
    },
    [ocr],
  );

  const handleUpdateOCRData = useCallback(
    (
      chatId: string | null,
      modelId: string,
      data: { text: string; box: number[][] }[],
    ) => {
      ocr.handleUpdateOCRData(chatId, modelId, data);
    },
    [ocr],
  );

  const trackPendingPromptAttachmentAnalysis = useCallback(
    (nextAttachments: Attachment[]) => {
      setPendingPromptAttachmentAnalysis(
        getAttachmentAnalysisCounts(nextAttachments),
      );
    },
    [],
  );

  const handleDeleteChatWrapper = async (id: string) => {
    const isActive = chatHistory.activeSessionId === id;
    await chatHistory.handleDeleteChat(id);
    if (isActive) {
      await navigation.performNewSession();
    }
  };

  const handleDeleteChatsWrapper = async (ids: string[]) => {
    const isActiveIncluded =
      chatHistory.activeSessionId && ids.includes(chatHistory.activeSessionId);
    await chatHistory.handleDeleteChats(ids);
    if (isActiveIncluded) {
      await navigation.performNewSession();
    }
  };

  const handleSwitchProfile = async (profileId: string) => {
    await navigation.performNewSession();
    await system.switchProfile(profileId);
  };

  const handleAddAccount = () => {
    system.addAccount();
  };

  const handleSystemAction = useCallback(
    async (actionId: string, _value?: string) => {
      switch (actionId) {
        case "agree":
          setAgreedToTerms(true);
          break;
        case "disagree":
          setAgreedToTerms(false);
          break;
        case "update_now":
          try {
            const update = await check();
            if (update && update.available) {
              await update.downloadAndInstall();
              await relaunch();
            } else {
              invoke("open_external_url", { url: github.latestRelease });
            }
          } catch {
            invoke("open_external_url", { url: github.latestRelease });
          }
          break;
        case "update_later":
          setShowUpdate(false);
          sessionStorage.setItem("update_dismissed", "true");
          await navigation.performNewSession();
          break;
      }
    },
    [navigation],
  );

  const containerRef = useRef<HTMLDivElement>(null);

  return {
    system,
    auth,
    chat,
    chatHistory,
    isSidePanelOpen: panel.isSidePanelOpen,
    enablePanelAnimation: panel.enablePanelAnimation,
    showProviderAuthDialog: dialogs.showProviderAuthDialog,
    showLoginRequiredDialog: dialogs.showLoginRequiredDialog,
    showCaptureDeniedDialog: dialogs.showCaptureDeniedDialog,
    sessionLensUrl: ocr.sessionLensUrl,
    ocrData: ocr.ocrData,
    input: drafts.input,
    imageInput: drafts.imageInput,
    inputModel: drafts.inputModel,
    attachments: attachments.attachments,
    pendingPromptAttachmentAnalysis,
    setInputModel: drafts.setInputModel,
    pendingUpdate,
    showUpdate,
    contextMenu: contextMenuState.contextMenu,
    isLoadingState,
    isAgreementPending,
    isImageMissing,
    chatTitle,
    agreedToTerms,
    busyDialog: busyGuard.busyDialog,
    mediaViewer: media.mediaViewer,
    searchOverlay: navigation.searchOverlay,
    toggleSidePanel: panel.toggleSidePanel,
    isNavigating: navigation.isNavigating,
    isChatContentReady: navigation.isChatContentReady,
    showChatShellDuringNavigation: navigation.showChatShellDuringNavigation,
    setShowProviderAuthDialog: dialogs.setShowProviderAuthDialog,
    setShowLoginRequiredDialog: dialogs.setShowLoginRequiredDialog,
    setShowCaptureDeniedDialog: dialogs.setShowCaptureDeniedDialog,
    performLogout,
    handleUpdateLensUrl,
    handleUpdateOCRData,
    handleImageReady: capture.handleImageReady,
    handleSelectChat: navigation.handleSelectChat,
    handleNewSession: navigation.handleNewSession,
    handleAddAccount,
    setInput: drafts.setInput,
    setImageInput: drafts.setImageInput,
    setAttachments: attachments.setAttachments,
    trackPendingPromptAttachmentAnalysis,
    addAttachmentFromPath: attachments.addFromPath,
    clearAttachments: attachments.clearAttachments,
    rememberAttachmentSourcePath: media.rememberAttachmentSourcePath,
    setShowUpdate,
    handleContextMenu: contextMenuState.handleContextMenu,
    handleCloseContextMenu: contextMenuState.handleCloseContextMenu,
    handleCopy: contextMenuState.handleCopy,
    handleDeleteChatWrapper,
    handleDeleteChatsWrapper,
    handleToggleStarChat: chatHistory.handleToggleStarChat,
    handleExit: () => exit(0),
    handleSwitchProfile,
    handleSystemAction,
    handleBusyDialogAction: busyGuard.handleBusyDialogAction,
    openMediaViewer: media.openMediaViewer,
    closeMediaViewer: media.closeMediaViewer,
    openSearchOverlay: navigation.openSearchOverlay,
    closeSearchOverlay: navigation.closeSearchOverlay,
    revealSearchMatch: navigation.revealSearchMatch,
    clearSearchReveal: navigation.clearSearchReveal,
    getAttachmentSourcePath: media.getAttachmentSourcePath,
    containerRef,
    isOcrScanning: ocr.isOcrScanning,
    setIsOcrScanning: ocr.setIsOcrScanning,
  };
};
