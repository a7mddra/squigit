/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import {
  AUTO_OCR_DISABLED_MODEL_ID,
  OcrFrame,
  loadChat,
  getImagePath,
  updateChatMetadata,
  cancelOcrJob,
  SUPPORTED_OCR_MODEL_IDS,
  resolveOcrModelId,
} from "@/core";
import { type Citation, type Message, type ToolStep } from "@/features";
import type {
  ChatCitation,
  ChatToolStep,
} from "@/core";

const SYSTEM_GALLERY_ID = "__system_gallery";
const isOnboardingId = (id: string) => id.startsWith("__system_");

type SearchRevealTarget = {
  chatId: string;
  messageIndex: number;
  requestedAt: number;
};

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

const withNavigationOcrGuard = (frame: OcrFrame): OcrFrame => ({
  ...frame,
  [AUTO_OCR_DISABLED_MODEL_ID]: [],
});

function normalizeStoredCitations(
  citations: ChatCitation[] | undefined,
): Citation[] {
  return Array.isArray(citations) ? citations : [];
}

function normalizeStoredToolSteps(
  toolSteps: ChatToolStep[] | undefined,
): ToolStep[] {
  if (!Array.isArray(toolSteps)) {
    return [];
  }

  return toolSteps.map((step) => ({
    ...step,
    status:
      step.status === "running" ||
      step.status === "done" ||
      step.status === "error"
        ? step.status
        : typeof step.endedAtMs === "number"
          ? "done"
          : "running",
    message: step.message ?? undefined,
  }));
}

function mapStoredMessage(message: {
  role: "user" | "assistant";
  content: string;
  timestamp: string;
  citations?: ChatCitation[];
  tool_steps?: ChatToolStep[];
}): Message {
  return {
    id: "",
    role: message.role === "user" ? "user" : "model",
    text: message.content,
    timestamp: new Date(message.timestamp).getTime(),
    alreadyStreamed: true,
    citations: normalizeStoredCitations(message.citations),
    toolSteps: normalizeStoredToolSteps(message.tool_steps),
  };
}

function waitForNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

export const useAppNavigation = ({
  chat,
  chatHistory,
  ocr,
  system,
  chatTitle,
  isActiveChatBusy,
  closeMediaViewer,
  runWithBusyGuard,
}: {
  chat: any;
  chatHistory: any;
  ocr: any;
  system: any;
  chatTitle: string;
  isActiveChatBusy: boolean;
  closeMediaViewer: () => void;
  runWithBusyGuard: (action: () => void | Promise<void>) => void;
}) => {
  const [isNavigating, setIsNavigating] = useState(false);
  const [isChatContentReady, setIsChatContentReady] = useState(true);
  const [showChatShellDuringNavigation, setShowChatShellDuringNavigation] =
    useState(false);
  const [isSearchOverlayOpen, setIsSearchOverlayOpen] = useState(false);
  const [pendingSearchReveal, setPendingSearchReveal] =
    useState<SearchRevealTarget | null>(null);

  const navigationRequestIdRef = useRef(0);
  const busyTouchStateRef = useRef<{ chatId: string | null; isBusy: boolean }>({
    chatId: null,
    isBusy: false,
  });

  const finalizeNavigationState = useCallback((requestId: number) => {
    if (navigationRequestIdRef.current !== requestId) {
      return;
    }

    flushSync(() => {
      setIsNavigating(false);
      setShowChatShellDuringNavigation(false);
      setIsChatContentReady(true);
    });
  }, []);

  useEffect(() => {
    const activeId = chatHistory.activeSessionId;
    const isBusy = !!activeId && !isOnboardingId(activeId) && isActiveChatBusy;
    const lastState = busyTouchStateRef.current;
    const wasBusyForSameChat =
      lastState.chatId === activeId && lastState.isBusy;

    if (isBusy && !wasBusyForSameChat && activeId) {
      chatHistory.touchChat(activeId).catch(console.error);
    }

    busyTouchStateRef.current = {
      chatId: activeId,
      isBusy,
    };
  }, [chatHistory, isActiveChatBusy]);

  useEffect(() => {
    if (isNavigating) return;

    const activeId = chatHistory.activeSessionId;
    if (activeId && chatTitle && chatTitle !== "New thread") {
      const currentChat = chatHistory.chats.find((c: any) => c.id === activeId);
      if (currentChat && currentChat.title !== chatTitle) {
        updateChatMetadata({
          ...currentChat,
          title: chatTitle,
        }).then(() => {
          chatHistory.handleRenameChat(activeId, chatTitle);
        });
      }
    }
  }, [chatHistory, chatTitle, isNavigating]);

  useEffect(() => {
    if (isNavigating) return;

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
    }).then(() => {
      console.log(
        "Automatically saved OCR language to chat metadata:",
        targetOcrLang,
      );
    });
  }, [chatHistory, isNavigating, system.sessionOcrLanguage, system.ocrEnabled]);

  const openSearchOverlay = useCallback(() => {
    setIsSearchOverlayOpen(true);
  }, []);

  const closeSearchOverlay = useCallback(() => {
    setIsSearchOverlayOpen(false);
  }, []);

  const clearSearchReveal = useCallback(() => {
    setPendingSearchReveal(null);
  }, []);

  const performSelectChat = useCallback(
    async (id: string) => {
      const requestId = navigationRequestIdRef.current + 1;
      navigationRequestIdRef.current = requestId;
      const metadata = chatHistory.chats.find((chatMeta: any) => chatMeta.id === id);

      flushSync(() => {
        setIsNavigating(true);
        setIsChatContentReady(false);
        setShowChatShellDuringNavigation(!isOnboardingId(id));
        setPendingSearchReveal(null);
        closeMediaViewer();
        ocr.setSessionLensUrl(null);

        if (!isOnboardingId(id) && metadata?.title) {
          system.setSessionChatTitle(metadata.title);
        }
      });

      cancelOcrJob();
      flushSync(() => {
        ocr.setIsOcrScanning(false);
        ocr.setOcrData(withNavigationOcrGuard({}));
      });

      await waitForNextPaint();
      if (navigationRequestIdRef.current !== requestId) {
        return;
      }

      if (isOnboardingId(id)) {
        flushSync(() => {
          if (id === "__system_welcome") {
            system.setSessionChatTitle(`Welcome to ${system.appName}!`);
          } else if (id.startsWith("__system_update")) {
            system.setSessionChatTitle("Update Available");
          } else if (id === SYSTEM_GALLERY_ID) {
            system.setSessionChatTitle("Gallery");
          }
          chatHistory.setActiveSessionId(id);
        });
        finalizeNavigationState(requestId);
        return;
      }

      try {
        const imagePathPromise = metadata?.image_hash
          ? getImagePath(metadata.image_hash)
          : null;
        const chatDataPromise = loadChat(id);

        if (imagePathPromise && metadata?.image_hash) {
          const imagePath = await imagePathPromise;
          if (navigationRequestIdRef.current !== requestId) {
            return;
          }

          flushSync(() => {
            system.setStartupImage({
              path: imagePath,
              mimeType: "image/png",
              imageId: metadata.image_hash,
              fromHistory: true,
              tone: metadata.image_tone ?? undefined,
            });
            chatHistory.setActiveSessionId(id);
          });

          chat.restoreState(
            {
              messages: [],
              streamingText: "",
              firstResponseId: null,
            },
            {
              path: imagePath,
              mimeType: "image/png",
              imageId: metadata.image_hash,
            },
            null,
          );

          await waitForNextPaint();
          if (navigationRequestIdRef.current !== requestId) {
            return;
          }
        }

        const chatData = await chatDataPromise;
        if (navigationRequestIdRef.current !== requestId) {
          return;
        }

        const imagePath = imagePathPromise
          ? await imagePathPromise
          : await getImagePath(chatData.metadata.image_hash);
        if (navigationRequestIdRef.current !== requestId) {
          return;
        }

        const loadedOcrData = chatData.ocr_data || {};
        const navigationSafeOcrData = withNavigationOcrGuard(loadedOcrData);
        const chatOcrModel = getChatOcrModel(
          loadedOcrData,
          chatData.metadata.ocr_lang,
        );
        flushSync(() => {
          system.setSessionChatTitle(chatData.metadata.title);
          system.setSessionOcrLanguage(system.ocrEnabled ? chatOcrModel : "");
          ocr.setOcrData(navigationSafeOcrData);
          ocr.setSessionLensUrl(chatData.imgbb_url || null);
          system.setStartupImage({
            path: imagePath,
            mimeType: "image/png",
            imageId: chatData.metadata.image_hash,
            fromHistory: true,
            tone: chatData.metadata.image_tone ?? undefined,
          });
          chatHistory.setActiveSessionId(id);
        });
        await waitForNextPaint();
        if (navigationRequestIdRef.current !== requestId) {
          return;
        }

        const messages = chatData.messages.map((message, idx) => ({
          ...mapStoredMessage(message),
          id: idx.toString(),
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
          chatData.rolling_summary,
          chatData.image_brief,
        );
      } catch (e) {
        console.error("Failed to load chat:", e);
      } finally {
        finalizeNavigationState(requestId);
      }
    },
    [chat, chatHistory, closeMediaViewer, finalizeNavigationState, ocr, system],
  );

  const performNewSession = useCallback(async () => {
    const requestId = navigationRequestIdRef.current + 1;
    navigationRequestIdRef.current = requestId;

    flushSync(() => {
      setIsNavigating(true);
      setIsChatContentReady(false);
      setShowChatShellDuringNavigation(false);
      setPendingSearchReveal(null);
      closeMediaViewer();
    });

    system.resetSession();
    chatHistory.setActiveSessionId(null);
    chatHistory.setActiveSessionId(null);
    ocr.setOcrData({});
    ocr.setSessionLensUrl(null);
    finalizeNavigationState(requestId);
  }, [chatHistory, closeMediaViewer, finalizeNavigationState, ocr, system]);

  const handleSelectChat = useCallback(
    (id: string) => {
      runWithBusyGuard(() => performSelectChat(id));
    },
    [performSelectChat, runWithBusyGuard],
  );

  const revealSearchMatch = useCallback(
    (payload: { chatId: string; messageIndex: number }) => {
      runWithBusyGuard(async () => {
        closeSearchOverlay();
        await performSelectChat(payload.chatId);
        setPendingSearchReveal({
          chatId: payload.chatId,
          messageIndex: payload.messageIndex,
          requestedAt: Date.now(),
        });
      });
    },
    [closeSearchOverlay, performSelectChat, runWithBusyGuard],
  );

  const handleNewSession = useCallback(() => {
    runWithBusyGuard(performNewSession);
  }, [performNewSession, runWithBusyGuard]);

  return {
    isNavigating,
    isChatContentReady,
    showChatShellDuringNavigation,
    searchOverlay: {
      isOpen: isSearchOverlayOpen,
      pendingReveal: pendingSearchReveal,
    },
    openSearchOverlay,
    closeSearchOverlay,
    clearSearchReveal,
    performSelectChat,
    performNewSession,
    handleSelectChat,
    revealSearchMatch,
    handleNewSession,
  };
};
