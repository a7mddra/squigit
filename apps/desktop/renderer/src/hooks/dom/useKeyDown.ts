/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useCallback } from "react";
import { usePlatform } from "@/hooks";

type KeyHandler = (e: React.KeyboardEvent) => void;
type KeyMap = Record<string, KeyHandler>;

interface UseKeyDownOptions {
  preventDefault?: boolean;
  stopPropagation?: boolean;
}

export function useKeyDown(keyMap: KeyMap, options: UseKeyDownOptions = {}) {
  const { isMac } = usePlatform();

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const modifiers: string[] = [];
      if (e.metaKey && isMac) modifiers.push("Mod");
      if (e.ctrlKey && !isMac) modifiers.push("Mod");
      if (e.ctrlKey && isMac) modifiers.push("Ctrl");
      if (e.metaKey && !isMac) modifiers.push("Meta");
      if (e.altKey) modifiers.push("Alt");
      if (e.shiftKey) modifiers.push("Shift");

      const key = e.key;

      const combo = [...modifiers, key].join("+");

      if (keyMap[combo]) {
        if (options.preventDefault !== false) e.preventDefault();
        if (options.stopPropagation) e.stopPropagation();
        keyMap[combo](e);
        return;
      }

      if (modifiers.length === 0 && keyMap[key]) {
        if (options.preventDefault !== false) e.preventDefault();
        if (options.stopPropagation) e.stopPropagation();
        keyMap[key](e);
      }
    },
    [keyMap, isMac, options.preventDefault, options.stopPropagation],
  );

  return handleKeyDown;
}
