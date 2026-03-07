/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useCallback } from "react";

export const useCopyToClipboard = (resetDelay = 2500) => {
  const [isCopied, setIsCopied] = useState(false);

  const copy = useCallback(
    (text: string) => {
      navigator.clipboard.writeText(text).then(() => {
        setIsCopied(true);
        setTimeout(() => setIsCopied(false), resetDelay);
      });
    },
    [resetDelay],
  );

  return { isCopied, copy };
};
