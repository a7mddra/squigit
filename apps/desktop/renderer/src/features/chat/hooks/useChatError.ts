/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect } from "react";
import { parseGeminiError } from "@/lib";
import { invoke } from "@tauri-apps/api/core";

export function useChatError(
  error: string | null,
  onOpenSettings: (section: any) => void,
) {
  const [isErrorDismissed, setIsErrorDismissed] = useState(false);

  useEffect(() => {
    setIsErrorDismissed(false);
  }, [error]);

  if (!error) {
    return { isErrorOpen: false, parsedError: null, errorActions: [] };
  }

  const parsedError = parseGeminiError(error);
  const isErrorOpen = !!error && !isErrorDismissed;

  const errorActions: any[] = [];
  errorActions.push({
    label: "Dismiss",
    onClick: () => setIsErrorDismissed(true),
    variant: "secondary",
  });

  if (parsedError.actionType === "RETRY_OR_SETTINGS") {
    errorActions.push({
      label: "Change API Key",
      onClick: () => {
        onOpenSettings("apikeys");
        setIsErrorDismissed(true);
      },
      variant: "secondary",
    });
  }

  if (parsedError.actionType === "RETRY_OR_LINK" && parsedError.meta?.link) {
    errorActions.push({
      label: parsedError.meta.linkLabel || "Open Link",
      onClick: () => {
        invoke("open_external_url", {
          url: parsedError.meta?.link,
        });
        setIsErrorDismissed(true);
      },
      variant: "secondary",
    });
  }

  return { isErrorOpen, parsedError, errorActions };
}
