/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";

export const useAppPanel = (isLoadingState: boolean) => {
  const [isSidePanelOpen, setIsSidePanelOpen] = useState(true);
  const [enablePanelAnimation, setEnablePanelAnimation] = useState(false);
  const toggleSidePanel = () => setIsSidePanelOpen((prev) => !prev);

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
