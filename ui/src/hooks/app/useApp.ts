/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect, useCallback, useRef } from "react";
import { exit, relaunch } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import {
  AUTO_OCR_DISABLED_MODEL_ID,
  DialogContent,
  OcrFrame,
  loadChat,
  getImagePath,
  createChat,
  updateChatMetadata,
  appendChatMessage,
  overwriteChatMessages,
  cancelOcrJob,
  saveOcrData,
  hasAgreedFlag,
  commands,
  getAppBusyDialog,
  github,
  resolveOcrModelId,
} from "@/lib";
import {
  useSystemSync,
  useUpdateCheck,
  getPendingUpdate,
  useAuth,
} from "@/hooks";
import {
  useAttachments,
  useChat,
  useChatHistory,
  useChatTitle,
  isImageExtension,
  type Attachment,
  type MediaViewerItem,
} from "@/features";

import { useAppDialogs } from "./useAppDialogs";
import { useAppDrafts } from "./useAppDrafts";
import { useAppContextMenu } from "./useAppContextMenu";
import { useAppOcr } from "./useAppOcr";
import { useAppPanel } from "./useAppPanel";

const isOnboardingId = (id: string) => id.startsWith("__system_");

const getChatOcrModel = (
  frame: OcrFrame,
  metadataOcrLanguage?: string,
  fallbackModel: string = "",
): string => {
  const hasModelData = (modelId?: string) =>
    !!modelId &&
    modelId !== AUTO_OCR_DISABLED_MODEL_ID &&
    Array.isArray(frame[modelId]);

  const resolvedMetadataModel = resolveOcrModelId(metadataOcrLanguage, "");
  if (resolvedMetadataModel && hasModelData(resolvedMetadataModel)) {
    return resolvedMetadataModel;
  }

  const scannedModels = Object.entries(frame)
    .filter(
      ([modelId, regions]) =>
        modelId !== AUTO_OCR_DISABLED_MODEL_ID && Array.isArray(regions),
    )
    .map(([modelId]) => resolveOcrModelId(modelId, ""))
    .filter((modelId) => !!modelId);

  if (scannedModels.length > 0) {
    return scannedModels[scannedModels.length - 1];
  }

  const firstScannedModel = Object.entries(frame).find(
    ([modelId, regions]) =>
      modelId !== AUTO_OCR_DISABLED_MODEL_ID && Array.isArray(regions),
  );
  if (firstScannedModel?.[0]) {
    return resolveOcrModelId(firstScannedModel[0], fallbackModel);
  }

  const firstKnownModel = Object.keys(frame).find(
    (modelId) => modelId !== AUTO_OCR_DISABLED_MODEL_ID,
  );
  if (firstKnownModel) {
    return resolveOcrModelId(firstKnownModel, fallbackModel);
  }

  return fallbackModel === "" ? "" : resolveOcrModelId(undefined, fallbackModel);
};

const withNavigationOcrGuard = (frame: OcrFrame): OcrFrame => ({
  ...frame,
  [AUTO_OCR_DISABLED_MODEL_ID]: [],
});

type GuardedAction = () => void | Promise<void>;

const UNSUPPORTED_PREVIEW_EXTENSIONS = new Set([
  "doc",
  "docx",
  "xls",
  "xlsx",
  "ppt",
  "pptx",
  "rtf",
  "odt",
  "ods",
  "odp",
  "pages",
  "numbers",
  "key",
]);

const ATTACHMENT_SOURCE_MAP_STORAGE_KEY =
  "snapllm:attachment-source-map:v1";
const ATTACHMENT_SOURCE_MAP_MAX_ENTRIES = 2048;

function readAttachmentSourceMap(): Map<string, string> {
  if (typeof window === "undefined") return new Map();

  try {
    const raw = localStorage.getItem(ATTACHMENT_SOURCE_MAP_STORAGE_KEY);
    if (!raw) return new Map();

    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return new Map();
    }

    const map = new Map<string, string>();
    for (const [casPath, sourcePath] of Object.entries(parsed)) {
      if (typeof sourcePath === "string" && sourcePath.length > 0) {
        map.set(casPath, sourcePath);
      }
    }

    return map;
  } catch {
    return new Map();
  }
}

function persistAttachmentSourceMap(map: Map<string, string>) {
  if (typeof window === "undefined") return;

  try {
    const entries = Array.from(map.entries());
    const trimmedEntries =
      entries.length > ATTACHMENT_SOURCE_MAP_MAX_ENTRIES
        ? entries.slice(entries.length - ATTACHMENT_SOURCE_MAP_MAX_ENTRIES)
        : entries;

    localStorage.setItem(
      ATTACHMENT_SOURCE_MAP_STORAGE_KEY,
      JSON.stringify(Object.fromEntries(trimmedEntries)),
    );
  } catch {
    // Ignore storage quota / JSON errors and keep in-memory map.
  }
}

export const useApp = () => {
  const system = useSystemSync();
  const auth = useAuth();

  const activeProfileRef = useRef<any>(null);
  const systemRef = useRef(system);

  useEffect(() => {
    activeProfileRef.current = system.activeProfile;
    systemRef.current = system;
  }, [system]);

  const [pendingUpdate] = useState(() => getPendingUpdate());
  const chatHistory = useChatHistory(system.activeProfile?.id || null);
  const chatHistoryRef = useRef(chatHistory);

  useEffect(() => {
    chatHistoryRef.current = chatHistory;
  }, [chatHistory]);

  useUpdateCheck();

  const handleImageReadyRef = useRef<any>(null);
  const handleSelectChatRef = useRef<any>(null);

  const performLogout = async () => {
    await system.handleLogout();
    auth.logout();
  };

  const dialogs = useAppDialogs();
  const drafts = useAppDrafts();
  const attachments = useAttachments();
  const contextMenuState = useAppContextMenu();
  const ocr = useAppOcr(chatHistory.activeSessionId);
  const attachmentSourceMapRef = useRef<Map<string, string>>(
    readAttachmentSourceMap(),
  );

  const [isCheckingImage, setIsCheckingImage] = useState(true);
  const [hasCheckedStartupImage, setHasCheckedStartupImage] = useState(false);

  const [hasAutoSelectedWelcome, setHasAutoSelectedWelcome] = useState(false);
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
    isCheckingImage ||
    isPendingAutoSelectWelcome;

  const panel = useAppPanel(isLoadingState);

  const hasActiveOnboarding = chatHistory.activeSessionId
    ? isOnboardingId(chatHistory.activeSessionId)
    : false;
  const isImageMissing = !system.startupImage && !hasActiveOnboarding;
  const isAuthPending = auth.authStage === "LOGIN";
  const isAgreementPending = system.hasAgreed === false;
  const isChatActive = !isLoadingState && !isImageMissing && !isAuthPending;

  const [showUpdate, setShowUpdate] = useState(() => {
    const wasDismissed = sessionStorage.getItem("update_dismissed");
    return !!pendingUpdate && !wasDismissed;
  });

  const [agreedToTerms, setAgreedToTerms] = useState(false);
  const agreedToTermsRef = useRef(false);

  useEffect(() => {
    agreedToTermsRef.current = agreedToTerms;
  }, [agreedToTerms]);

  const { isGeneratingTitle, generateTitleForText } = useChatTitle({
    apiKey: system.apiKey,
  });

  const handleMessageAdded = useCallback(
    (msg: any, targetChatId?: string) => {
      const activeId = targetChatId || chatHistory.activeSessionId;
      if (activeId) {
        const role = msg.role === "user" ? "user" : "assistant";
        appendChatMessage(activeId, role, msg.text).catch(console.error);
      }
    },
    [chatHistory.activeSessionId],
  );

  const handleOverwriteMessages = useCallback(
    (msgs: any[]) => {
      const activeId = chatHistory.activeSessionId;
      if (activeId) {
        const formatted = msgs.map((m: any) => ({
          role: (m.role === "user" ? "user" : "assistant") as
            | "user"
            | "assistant",
          content: m.text,
          timestamp: new Date(m.timestamp).toISOString(),
        }));
        overwriteChatMessages(activeId, formatted).catch(console.error);
      }
    },
    [chatHistory.activeSessionId],
  );

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
    onMissingApiKey: () => {
      dialogs.setShowGeminiAuthDialog(true);
    },
    onTitleGenerated: (title: string) => {
      system.setSessionChatTitle(title);
    },
    generateTitle: generateTitleForText,
  });

  const chatTitle = isImageMissing
    ? system.appName
    : isGeneratingTitle
      ? "New Chat"
      : system.sessionChatTitle || "New Chat";

  useEffect(() => {
    const activeId = chatHistory.activeSessionId;
    if (activeId && chatTitle && chatTitle !== "New Chat") {
      const currentChat = chatHistory.chats.find((c: any) => c.id === activeId);
      if (currentChat && currentChat.title !== chatTitle) {
        updateChatMetadata({
          ...currentChat,
          title: chatTitle,
          updated_at: new Date().toISOString(),
        }).then(() => {
          chatHistory.handleRenameChat(activeId, chatTitle);
        });
      }
    }
  }, [chatTitle, chatHistory.activeSessionId]);

  useEffect(() => {
    const activeId = chatHistory.activeSessionId;
    if (!activeId || isOnboardingId(activeId)) return;

    const currentChat = chatHistory.chats.find((c: any) => c.id === activeId);
    if (!currentChat) return;

    const targetOcrLang = system.sessionOcrLanguage || undefined;
    const currentOcrLang = currentChat.ocr_lang || undefined;
    if (currentOcrLang === targetOcrLang) return;

    updateChatMetadata({
      ...currentChat,
      ocr_lang: targetOcrLang,
      updated_at: new Date().toISOString(),
    }).then(() => {
      console.log("Automatically saved OCR language to chat metadata:", targetOcrLang);
    });
  }, [system.sessionOcrLanguage, chatHistory.activeSessionId, chatHistory.chats]);

  const [isNavigating, setIsNavigating] = useState(false);
  const [busyDialog, setBusyDialog] = useState<DialogContent | null>(null);
  const pendingBusyActionRef = useRef<GuardedAction | null>(null);
  const runWithBusyGuardRef = useRef<(action: GuardedAction) => void>(
    () => {},
  );

  const runAction = useCallback((action: GuardedAction) => {
    Promise.resolve(action()).catch((error) => {
      console.error("[busy-gate] Action failed:", error);
    });
  }, []);

  const getBusyReason = useCallback((): string | null => {
    const activeStates: string[] = [];

    if (chat.isAnalyzing) activeStates.push("analyzing an image");
    if (chat.isGenerating) activeStates.push("generating a response");
    if (chat.isAiTyping) activeStates.push("typing a response");
    if (ocr.isOcrScanning) activeStates.push("scanning an image");

    if (activeStates.length === 0) return null;
    if (activeStates.length === 1) return activeStates[0];

    const last = activeStates.pop();
    return `${activeStates.join(", ")} and ${last}`;
  }, [chat.isAnalyzing, chat.isGenerating, chat.isAiTyping, ocr.isOcrScanning]);

  const runWithBusyGuard = useCallback(
    (action: GuardedAction) => {
      const reason = getBusyReason();
      if (reason) {
        pendingBusyActionRef.current = action;
        setBusyDialog(getAppBusyDialog(reason));
        return;
      }

      runAction(action);
    },
    [getBusyReason, runAction],
  );

  const killActiveJobs = useCallback(() => {
    cancelOcrJob();
    ocr.setIsOcrScanning(false);
    const safeOcrModel = getChatOcrModel(ocr.ocrData, undefined, "");
    system.setSessionOcrLanguage(safeOcrModel);

    if (chat.isAnalyzing || chat.isGenerating || chat.isAiTyping) {
      chat.handleStopGeneration();
    }
  }, [
    chat.isAnalyzing,
    chat.isGenerating,
    chat.isAiTyping,
    chat.handleStopGeneration,
    ocr,
    system,
  ]);

  const handleBusyDialogAction = useCallback(
    (actionKey: string) => {
      if (actionKey !== "confirm") {
        pendingBusyActionRef.current = null;
        setBusyDialog(null);
        return;
      }

      const pendingAction = pendingBusyActionRef.current;
      pendingBusyActionRef.current = null;
      setBusyDialog(null);
      killActiveJobs();

      if (pendingAction) {
        setTimeout(() => {
          runAction(pendingAction);
        }, 0);
      }
    },
    [killActiveJobs, runAction],
  );

  useEffect(() => {
    runWithBusyGuardRef.current = runWithBusyGuard;
  }, [runWithBusyGuard]);

  const rememberAttachmentSourcePath = useCallback(
    (casPath: string, sourcePath: string) => {
      if (!casPath || !sourcePath) return;
      attachmentSourceMapRef.current.set(casPath, sourcePath);
      persistAttachmentSourceMap(attachmentSourceMapRef.current);
    },
    [],
  );

  useEffect(() => {
    attachments.attachments.forEach((attachment) => {
      if (attachment.sourcePath) {
        rememberAttachmentSourcePath(attachment.path, attachment.sourcePath);
      }
    });
  }, [attachments.attachments, rememberAttachmentSourcePath]);

  const getAttachmentSourcePath = useCallback((path: string) => {
    const directMatch = attachmentSourceMapRef.current.get(path);
    if (directMatch) return directMatch;

    const fileName = path.split(/[/\\]/).pop();
    if (!fileName) return null;

    for (const [casPath, sourcePath] of attachmentSourceMapRef.current) {
      if (casPath.endsWith(`/${fileName}`) || casPath.endsWith(`\\${fileName}`)) {
        return sourcePath;
      }
    }

    return null;
  }, []);

  const [mediaViewer, setMediaViewer] = useState<{
    isOpen: boolean;
    item: MediaViewerItem | null;
  }>({
    isOpen: false,
    item: null,
  });

  const closeMediaViewer = useCallback(() => {
    setMediaViewer((prev) => ({ ...prev, isOpen: false }));
  }, []);

  const revealInFileManager = useCallback(async (path: string) => {
    try {
      await invoke("reveal_in_file_manager", { path });
    } catch (error) {
      console.error("[media] Failed to reveal in file manager:", error);
      throw error;
    }
  }, []);

  const openMediaViewer = useCallback(
    async (attachment: Attachment) => {
      const extension = attachment.extension.toLowerCase();
      const sourcePath =
        attachment.sourcePath || getAttachmentSourcePath(attachment.path) || undefined;

      let resolvedPath = attachment.path;
      try {
        resolvedPath = await invoke<string>("resolve_attachment_path", {
          path: attachment.path,
        });
      } catch (error) {
        console.warn("[media] Could not resolve attachment path:", error);
      }

      const revealPath = sourcePath || resolvedPath;

      if (UNSUPPORTED_PREVIEW_EXTENSIONS.has(extension)) {
        try {
          await revealInFileManager(revealPath);
        } catch {
          if (revealPath !== resolvedPath) {
            await revealInFileManager(resolvedPath);
          }
        }
        return;
      }

      if (attachment.type === "image" || isImageExtension(extension)) {
        setMediaViewer({
          isOpen: true,
          item: {
            kind: "image",
            path: resolvedPath,
            sourcePath,
            name: attachment.name,
            extension,
          },
        });
        return;
      }

      if (extension === "pdf") {
        setMediaViewer({
          isOpen: true,
          item: {
            kind: "pdf",
            path: resolvedPath,
            sourcePath,
            name: attachment.name,
            extension,
          },
        });
        return;
      }

      try {
        const textContent = await invoke<string>("read_attachment_text", {
          path: resolvedPath,
        });

        setMediaViewer({
          isOpen: true,
          item: {
            kind: "text",
            path: resolvedPath,
            sourcePath,
            name: attachment.name,
            extension,
            textContent,
          },
        });
      } catch (error) {
        console.warn("[media] Falling back to file-manager reveal:", error);
        try {
          await revealInFileManager(revealPath);
        } catch {
          if (revealPath !== resolvedPath) {
            await revealInFileManager(resolvedPath);
          }
        }
      }
    },
    [getAttachmentSourcePath, revealInFileManager],
  );

  const handleImageReady = async (imageData: {
    imageId: string;
    path: string;
  }) => {
    if (!activeProfileRef.current) {
      console.log("Image upload attempted in guest mode - requiring login");
      dialogs.setShowLoginRequiredDialog(true);
      return;
    }

    console.log("Raw image path:", imageData.path);
    closeMediaViewer();

    chatHistory.setActiveSessionId(null);
    chatHistory.setActiveSessionId(null);
    ocr.setOcrData({});
    ocr.setSessionLensUrl(null);

    systemRef.current.setSessionOcrLanguage(
      systemRef.current.ocrEnabled
        ? resolveOcrModelId(systemRef.current.startupOcrLanguage)
        : "",
    );
    ocr.setIsOcrScanning(false);
    cancelOcrJob();

    system.setStartupImage({
      path: imageData.path,
      mimeType: "image/png",
      imageId: imageData.imageId,
    });

    try {
      const newChat = await createChat(
        "New Chat",
        imageData.imageId,
        systemRef.current.ocrEnabled
          ? resolveOcrModelId(systemRef.current.startupOcrLanguage)
          : null,
      );
      if (!systemRef.current.ocrEnabled) {
        await saveOcrData(newChat.id, AUTO_OCR_DISABLED_MODEL_ID, []);
      }
      chatHistory.setActiveSessionId(newChat.id);
      chatHistory.refreshChats();
      console.log("Created new chat:", newChat.id);
    } catch (e) {
      console.error("Failed to create chat:", e);
    }
  };

  const performSelectChat = useCallback(
    async (id: string) => {
      setIsNavigating(true);
      closeMediaViewer();

      // Hard-kill OCR auto-runs during navigation. OCR can only start
      // on fresh chat creation (when enabled) or manual model selection.
      cancelOcrJob();
      ocr.setIsOcrScanning(false);
      ocr.setOcrData(withNavigationOcrGuard({}));

      if (isOnboardingId(id)) {
        ocr.setSessionLensUrl(null);
        if (id === "__system_welcome") {
          system.setSessionChatTitle(`Welcome to ${system.appName}!`);
        } else if (id.startsWith("__system_update")) {
          system.setSessionChatTitle("Update Available");
        }
        chatHistory.setActiveSessionId(id);
        setTimeout(() => setIsNavigating(false), 300);
        return;
      }

      try {
        const chatData = await loadChat(id);
        const imagePath = await getImagePath(chatData.metadata.image_hash);

        system.setSessionChatTitle(chatData.metadata.title);

        const loadedOcrData = chatData.ocr_data || {};
        const navigationSafeOcrData = withNavigationOcrGuard(loadedOcrData);
        const chatOcrModel = getChatOcrModel(
          loadedOcrData,
          chatData.metadata.ocr_lang,
          resolveOcrModelId(chatData.metadata.ocr_lang, ""),
        );
        system.setSessionOcrLanguage(system.ocrEnabled ? chatOcrModel : "");

        const messages = chatData.messages.map((m, idx) => ({
          id: idx.toString(),
          role: m.role as "user" | "model",
          text: m.content,
          timestamp: new Date(m.timestamp).getTime(),
        }));

        chat.restoreState(
          {
            messages,
            streamingText: "",
            firstResponseId: null,
          },
          {
            path: imagePath,
            mimeType: "image/png",
            imageId: chatData.metadata.image_hash,
          },
        );

        ocr.setOcrData(navigationSafeOcrData);

        ocr.setSessionLensUrl(chatData.imgbb_url || null);

        system.setStartupImage({
          path: imagePath,
          mimeType: "image/png",
          imageId: chatData.metadata.image_hash,
          fromHistory: true,
        });

        chatHistory.setActiveSessionId(id);
      } catch (e) {
        console.error("Failed to load chat:", e);
      } finally {
        setTimeout(() => setIsNavigating(false), 300);
      }
    },
    [chat, chatHistory, closeMediaViewer, ocr, system],
  );

  const performNewSession = useCallback(() => {
    setIsNavigating(true);
    closeMediaViewer();
    system.resetSession();
    chatHistory.setActiveSessionId(null);
    chatHistory.setActiveSessionId(null);
    ocr.setOcrData({});
    ocr.setSessionLensUrl(null);
    setTimeout(() => setIsNavigating(false), 300);
  }, [chatHistory, closeMediaViewer, ocr, system]);

  const handleSelectChat = useCallback(
    (id: string) => {
      runWithBusyGuard(() => performSelectChat(id));
    },
    [performSelectChat, runWithBusyGuard],
  );

  const handleNewSession = useCallback(() => {
    runWithBusyGuard(performNewSession);
  }, [performNewSession, runWithBusyGuard]);

  const handleDeleteChatWrapper = async (id: string) => {
    const isActive = chatHistory.activeSessionId === id;
    await chatHistory.handleDeleteChat(id);
    if (isActive) {
      performNewSession();
    }
  };

  const handleDeleteChatsWrapper = async (ids: string[]) => {
    const isActiveIncluded =
      chatHistory.activeSessionId && ids.includes(chatHistory.activeSessionId);
    await chatHistory.handleDeleteChats(ids);
    if (isActiveIncluded) {
      performNewSession();
    }
  };

  const handleSwitchProfile = async (profileId: string) => {
    performNewSession();
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

          performNewSession();
          break;
      }
    },
    [performNewSession],
  );

  useEffect(() => {
    handleImageReadyRef.current = handleImageReady;
    handleSelectChatRef.current = performSelectChat;
  });

  useEffect(() => {
    if (!system.profileLoaded || !system.prefsLoaded || hasCheckedStartupImage)
      return;

    const initStartupImage = async () => {
      try {
        const initialImage = await commands.getInitialImage();
        if (initialImage) {
          console.log("Found CLI image in state, loading...");
          handleImageReady({
            imageId: initialImage.hash,
            path: initialImage.path,
          });
        }
      } catch (e) {
        console.error("Failed to check initial image:", e);
      } finally {
        setIsCheckingImage(false);
        setHasCheckedStartupImage(true);
      }
    };

    initStartupImage();
  }, [system.profileLoaded, system.prefsLoaded, hasCheckedStartupImage]);

  useEffect(() => {
    if (
      system.hasAgreed === false &&
      auth.authStage !== "LOADING" &&
      !isCheckingImage &&
      !system.activeProfile &&
      !chatHistory.activeSessionId &&
      !hasAutoSelectedWelcome
    ) {
      performSelectChat("__system_welcome");
      setHasAutoSelectedWelcome(true);
    }
  }, [
    system.hasAgreed,
    auth.authStage,
    isCheckingImage,
    system.activeProfile,
    chatHistory.activeSessionId,
    hasAutoSelectedWelcome,
  ]);

  useEffect(() => {
    const unlisten = listen<string>("image-path", async (event) => {
      const imagePath = event.payload;
      if (imagePath) {
        if (!activeProfileRef.current) {
          console.log(
            "CLI/External image drop attempted in guest mode - requiring login",
          );
          dialogs.setShowLoginRequiredDialog(true);
          return;
        }

        try {
          console.log("Event received for image:", imagePath);
          const result = await commands.processImagePath(imagePath);
          if (handleImageReadyRef.current) {
            handleImageReadyRef.current({
              imageId: result.hash,
              path: result.path,
            });
          }
        } catch (error) {
          console.error("Failed to process CLI image event:", error);
        }
      }
    });

    const unlistenLoadChat = listen<string>("load-chat", async (event) => {
      const chatId = event.payload;
      if (chatId) {
        console.log("Triggering frontend transition to new capture:", chatId);
        if (handleSelectChatRef.current) {
          await handleSelectChatRef.current(chatId);
        }
      }
    });

    const unlistenCaptureRequested = listen("capture-requested", () => {
      runWithBusyGuardRef.current(() => invoke("spawn_capture"));
    });

    const unlistenCapture = listen<{ chatId: string; imageHash: string }>(
      "capture-complete",
      async (event) => {
        const { chatId, imageHash } = event.payload;
        console.log(
          "[capture-complete] chatId:",
          chatId,
          "imageHash:",
          imageHash,
        );

        try {
          if (!activeProfileRef.current) {
            console.log(
              "Capture upload attempted in guest mode - requiring login",
            );
            dialogs.setShowLoginRequiredDialog(true);
            return;
          }

          const imagePath = await getImagePath(imageHash);

          systemRef.current.setSessionChatTitle(null);
          systemRef.current.setSessionOcrLanguage(
            systemRef.current.ocrEnabled
              ? resolveOcrModelId(systemRef.current.startupOcrLanguage)
              : "",
          );
          ocr.setOcrData({});
          ocr.setSessionLensUrl(null);
          ocr.setIsOcrScanning(false);
          cancelOcrJob();

          systemRef.current.setStartupImage({
            path: imagePath,
            mimeType: "image/png",
            imageId: imageHash,
          });

          chatHistoryRef.current.setActiveSessionId(null);

          await new Promise((resolve) => setTimeout(resolve, 10));

          chatHistoryRef.current.setActiveSessionId(chatId);
          if (!systemRef.current.ocrEnabled) {
            await saveOcrData(chatId, AUTO_OCR_DISABLED_MODEL_ID, []);
          }
          chatHistoryRef.current.refreshChats();
        } catch (error) {
          console.error("[capture-complete] Failed:", error);
        }
      },
    );

    const unlistenCaptureFailed = listen<{ reason: string }>(
      "capture-failed",
      (event) => {
        const { reason } = event.payload;
        if (reason === "User denied screen capture permission.") {
          dialogs.setShowCaptureDeniedDialog(true);
        } else {
          console.error("[capture-failed]", reason);
        }
      },
    );

    const unlistenAuthSuccess = listen<any>("auth-success", async (event) => {
      if (
        activeProfileRef.current &&
        event.payload &&
        activeProfileRef.current.id === event.payload.id
      ) {
        return;
      }

      const alreadyAgreed = await hasAgreedFlag();
      if (!alreadyAgreed) {
        if (agreedToTermsRef.current) {
          system.setAgreementCompleted();
        }
      }

      performNewSession();
      auth.login();
    });

    return () => {
      unlisten.then((f) => f());
      unlistenLoadChat.then((f) => f());
      unlistenCaptureRequested.then((f) => f());
      unlistenCapture.then((f) => f());
      unlistenCaptureFailed.then((f) => f());
      unlistenAuthSuccess.then((f) => f());
    };
  }, []);

  const containerRef = useRef<HTMLDivElement>(null);

  return {
    system,
    auth,
    chat,
    chatHistory,
    isSidePanelOpen: panel.isSidePanelOpen,
    enablePanelAnimation: panel.enablePanelAnimation,
    showGeminiAuthDialog: dialogs.showGeminiAuthDialog,
    showLoginRequiredDialog: dialogs.showLoginRequiredDialog,
    showCaptureDeniedDialog: dialogs.showCaptureDeniedDialog,
    sessionLensUrl: ocr.sessionLensUrl,
    ocrData: ocr.ocrData,
    input: drafts.input,
    imageInput: drafts.imageInput,
    inputModel: drafts.inputModel,
    attachments: attachments.attachments,
    setInputModel: drafts.setInputModel,
    pendingUpdate,
    showUpdate,
    contextMenu: contextMenuState.contextMenu,
    isLoadingState,
    isAgreementPending,
    isImageMissing,
    chatTitle,
    agreedToTerms,
    busyDialog,
    mediaViewer,

    toggleSidePanel: panel.toggleSidePanel,
    isNavigating,
    setShowGeminiAuthDialog: dialogs.setShowGeminiAuthDialog,
    setShowLoginRequiredDialog: dialogs.setShowLoginRequiredDialog,
    setShowCaptureDeniedDialog: dialogs.setShowCaptureDeniedDialog,
    performLogout,
    handleUpdateLensUrl: ocr.handleUpdateLensUrl,
    handleUpdateOCRData: ocr.handleUpdateOCRData,
    handleImageReady,
    handleSelectChat,
    handleNewSession,
    handleAddAccount,
    setInput: drafts.setInput,
    setImageInput: drafts.setImageInput,
    setAttachments: attachments.setAttachments,
    addAttachmentFromPath: attachments.addFromPath,
    clearAttachments: attachments.clearAttachments,
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
    handleBusyDialogAction,
    openMediaViewer,
    closeMediaViewer,
    getAttachmentSourcePath,
    containerRef,
    isOcrScanning: ocr.isOcrScanning,
    setIsOcrScanning: ocr.setIsOcrScanning,
  };
};
