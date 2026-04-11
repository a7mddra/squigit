/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  AUTO_OCR_DISABLED_MODEL_ID,
  cancelOcrJob,
  commands,
  createChat,
  getImagePath,
  hasAgreedFlag,
  resolveOcrModelId,
  saveOcrData,
} from "@/core";

export const useAppCapture = ({
  system,
  auth,
  chatHistory,
  ocr,
  dialogs,
  activeProfileRef,
  systemRef,
  chatHistoryRef,
  performSelectChat,
  performNewSession,
  closeMediaViewer,
  runWithBusyGuardRef,
  agreedToTermsRef,
  hasShownCaptureTerminalHintRef,
}: {
  system: any;
  auth: any;
  chatHistory: any;
  ocr: any;
  dialogs: any;
  activeProfileRef: React.MutableRefObject<any>;
  systemRef: React.MutableRefObject<any>;
  chatHistoryRef: React.MutableRefObject<any>;
  performSelectChat: (id: string) => Promise<void>;
  performNewSession: () => Promise<void>;
  closeMediaViewer: () => void;
  runWithBusyGuardRef: React.MutableRefObject<
    (action: () => void | Promise<void>) => void
  >;
  agreedToTermsRef: React.MutableRefObject<boolean>;
  hasShownCaptureTerminalHintRef: React.MutableRefObject<boolean>;
}) => {
  const [isCheckingImage, setIsCheckingImage] = useState(true);
  const [hasCheckedStartupImage, setHasCheckedStartupImage] = useState(false);

  const handleImageReady = useCallback(
    async (imageData: { imageId: string; path: string }) => {
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
        const newThread = await createChat(
          "New thread",
          imageData.imageId,
          systemRef.current.ocrEnabled
            ? resolveOcrModelId(systemRef.current.startupOcrLanguage)
            : null,
        );
        if (!systemRef.current.ocrEnabled) {
          await saveOcrData(newThread.id, AUTO_OCR_DISABLED_MODEL_ID, []);
        }
        chatHistory.setActiveSessionId(newThread.id);
        chatHistory.refreshChats();
        console.log("Created new thread:", newThread.id);
      } catch (e) {
        console.error("Failed to create chat:", e);
      }
    },
    [
      activeProfileRef,
      chatHistory,
      closeMediaViewer,
      dialogs,
      ocr,
      system,
      systemRef,
    ],
  );

  const handleImageReadyRef = useRef(handleImageReady);
  const handleSelectChatRef = useRef(performSelectChat);

  useEffect(() => {
    handleImageReadyRef.current = handleImageReady;
    handleSelectChatRef.current = performSelectChat;
  }, [handleImageReady, performSelectChat]);

  useEffect(() => {
    if (!system.profileLoaded || !system.prefsLoaded || hasCheckedStartupImage) {
      return;
    }

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
  }, [handleImageReady, hasCheckedStartupImage, system.prefsLoaded, system.profileLoaded]);

  useEffect(() => {
    const unlisten = listen<string>("image-path", async (event) => {
      const imagePath = event.payload;
      if (!imagePath) return;

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
        handleImageReadyRef.current({
          imageId: result.hash,
          path: result.path,
        });
      } catch (error) {
        console.error("Failed to process CLI image event:", error);
      }
    });

    const unlistenLoadChat = listen<string>("load-chat", async (event) => {
      const chatId = event.payload;
      if (!chatId) return;

      console.log("Triggering frontend transition to new capture:", chatId);
      await handleSelectChatRef.current(chatId);
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
            console.log("Capture upload attempted in guest mode - requiring login");
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
          if (!hasShownCaptureTerminalHintRef.current) {
            hasShownCaptureTerminalHintRef.current = true;
            console.warn(
              "[capture-failed] See VS Code terminal for detailed backend capture logs.",
            );
          }
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
      if (!alreadyAgreed && agreedToTermsRef.current) {
        system.setAgreementCompleted();
      }

      await performNewSession();
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
  }, [
    activeProfileRef,
    agreedToTermsRef,
    auth,
    chatHistoryRef,
    dialogs,
    hasShownCaptureTerminalHintRef,
    ocr,
    performNewSession,
    runWithBusyGuardRef,
    system,
    systemRef,
  ]);

  return {
    handleImageReady,
    isCheckingImage,
    hasCheckedStartupImage,
  };
};
