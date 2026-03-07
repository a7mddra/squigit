/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { createContext, useContext, ReactNode } from "react";
import { useApp } from "@/hooks";

type AppState = ReturnType<typeof useApp>;

const AppContext = createContext<AppState | null>(null);

export const useAppContext = () => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useAppContext must be used within a AppProvider");
  }
  return context;
};

export const AppProvider: React.FC<{ children: ReactNode }> = ({
  children,
}) => {
  const app = useApp();

  return <AppContext.Provider value={app}>{children}</AppContext.Provider>;
};
