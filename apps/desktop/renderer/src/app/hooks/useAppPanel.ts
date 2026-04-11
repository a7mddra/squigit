/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback, useEffect, useState } from "react";

export const useAppPanel = (
  isLoadingState: boolean,
  closeOnTrigger: boolean = false,
) => {
  const [isSidePanelOpen, setIsSidePanelOpen] = useState(true);
  const [enablePanelAnimation, setEnablePanelAnimation] = useState(false);
  const toggleSidePanel = useCallback(() => {
    setIsSidePanelOpen((prev) => !prev);
  }, []);

  const [wasTriggered, setWasTriggered] = useState(false);
  useEffect(() => {
    if (closeOnTrigger && !wasTriggered) {
      setIsSidePanelOpen(false);
      setWasTriggered(true);
    }

    if (!closeOnTrigger && wasTriggered) {
      setWasTriggered(false);
    }
  }, [closeOnTrigger, wasTriggered]);

  useEffect(() => {
    if (!isLoadingState) {
      setTimeout(() => setEnablePanelAnimation(true), 50);
    }
  }, [isLoadingState]);

  return {
    isSidePanelOpen,
    enablePanelAnimation,
    toggleSidePanel,
  };
};
