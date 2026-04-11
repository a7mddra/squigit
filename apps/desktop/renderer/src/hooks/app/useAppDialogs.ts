/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from "react";

export const useAppDialogs = () => {
  const [showProviderAuthDialog, setShowProviderAuthDialog] = useState(false);
  const [showLoginRequiredDialog, setShowLoginRequiredDialog] = useState(false);
  const [showCaptureDeniedDialog, setShowCaptureDeniedDialog] = useState(false);

  return {
    showProviderAuthDialog,
    setShowProviderAuthDialog,
    showLoginRequiredDialog,
    setShowLoginRequiredDialog,
    showCaptureDeniedDialog,
    setShowCaptureDeniedDialog,
  };
};
