/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { AppProvider, useAppContext } from "./providers/AppProvider";
import { SplashScreen } from "./shell/screens/SplashScreen";
import { AppRouter } from "./router/AppRouter";

const AppHostContent: React.FC = () => {
  const app = useAppContext();

  if (app.isLoadingState) {
    return <SplashScreen onContextMenu={app.handleContextMenu} />;
  }

  return <AppRouter />;
};

export const AppHost: React.FC = () => {
  return (
    <AppProvider>
      <AppHostContent />
    </AppProvider>
  );
};
