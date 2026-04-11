/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { cancelOcrJob, DialogContent, getAppBusyDialog } from "@/core";

type GuardedAction = () => void | Promise<void>;

export const useAppBusyGuard = ({
  chat,
  ocr,
  system,
  getSafeOcrModel,
}: {
  chat: any;
  ocr: any;
  system: any;
  getSafeOcrModel: () => string;
}) => {
  const [busyDialog, setBusyDialog] = useState<DialogContent | null>(null);
  const pendingBusyActionRef = useRef<GuardedAction | null>(null);
  const runWithBusyGuardRef = useRef<(action: GuardedAction) => void>(() => {});

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
  }, [chat.isAiTyping, chat.isAnalyzing, chat.isGenerating, ocr.isOcrScanning]);

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
    system.setSessionOcrLanguage(getSafeOcrModel());

    if (chat.isAnalyzing || chat.isGenerating || chat.isAiTyping) {
      chat.handleStopGeneration();
    }
  }, [
    chat.handleStopGeneration,
    chat.isAiTyping,
    chat.isAnalyzing,
    chat.isGenerating,
    getSafeOcrModel,
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

  return {
    busyDialog,
    handleBusyDialogAction,
    runWithBusyGuard,
    runWithBusyGuardRef,
  };
};
