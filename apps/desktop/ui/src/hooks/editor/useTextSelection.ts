/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { generateSearchUrl, generateTranslateUrl } from "@/lib";

const MENU_HEIGHT = 48;
const NOTCH_OFFSET = 12;

interface UseTextSelectionOptions {
  containerRef: React.RefObject<HTMLElement | null>;
  onSelectAll?: () => void;
  onMenuShow?: () => void;
  onMenuHide?: () => void;
  positioningStrategy?: "visual" | "simple";
}

export const useTextSelection = ({
  containerRef,
  onSelectAll,
  onMenuShow,
  onMenuHide,
  positioningStrategy = "visual",
}: UseTextSelectionOptions) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const sliderRef = useRef<HTMLDivElement>(null);
  const page1Ref = useRef<HTMLDivElement>(null);
  const page2Ref = useRef<HTMLDivElement>(null);
  const pageFlatRef = useRef<HTMLDivElement>(null);

  const [menuActive, setMenuActive] = useState(false);
  const [isSelectAllMode, setIsSelectAllMode] = useState(false);
  const [containerElement, setContainerElement] = useState<HTMLElement | null>(
    null,
  );

  useEffect(() => {
    if (containerRef.current && containerRef.current !== containerElement) {
      setContainerElement(containerRef.current);
    }
  });

  const hookId = useRef(Math.random().toString(36).substr(2, 9)).current;

  const getSelectedText = useCallback(() => {
    return window.getSelection()?.toString().trim() || "";
  }, []);

  const hideMenu = useCallback(() => {
    if (menuRef.current) {
      menuRef.current.classList.remove("animating-layout");
      menuRef.current.classList.remove("active");
    }
    setMenuActive(false);
    setIsSelectAllMode(false);
    onMenuHide?.();
  }, [onMenuHide]);

  const renderPage = useCallback(
    (pageIndex: number, animateSlider: boolean = true) => {
      const slider = sliderRef.current;
      const menu = menuRef.current;
      if (
        !slider ||
        !menu ||
        !page1Ref.current ||
        !page2Ref.current ||
        !pageFlatRef.current
      )
        return;

      const p1W = page1Ref.current.offsetWidth;
      const p2W = page2Ref.current.offsetWidth;
      const pFlatW = pageFlatRef.current.offsetWidth;

      const widths = [p1W, p2W, pFlatW];
      let targetWidth = widths[0];
      let slideOffset = 0;

      if (pageIndex === 0) {
        targetWidth = widths[0];
        slideOffset = 0;
      } else if (pageIndex === 1) {
        targetWidth = widths[1];
        slideOffset = -widths[0];
      } else if (pageIndex === 2) {
        targetWidth = widths[2];
        slideOffset = -(widths[0] + widths[1]);
      }

      menu.style.width = `${targetWidth}px`;

      if (animateSlider) {
        slider.style.transition =
          "transform 0.4s cubic-bezier(0.25, 0.8, 0.25, 1)";
      } else {
        slider.style.transition = "none";
      }
      slider.style.transform = `translateX(${slideOffset}px)`;
    },
    [],
  );

  const positionMenu = useCallback(
    (
      selectionRectViewport: { left: number; width: number; top: number },
      _showNotch: boolean = true,
    ) => {
      const menu = menuRef.current;
      if (!menu) return;

      const selectionCenterViewport =
        selectionRectViewport.left + selectionRectViewport.width / 2;
      const selectionTopViewport = selectionRectViewport.top;

      const menuWidth = menu.offsetWidth || 180;

      let menuLeftViewport = selectionCenterViewport - menuWidth / 2;
      let menuTopViewport = selectionTopViewport - MENU_HEIGHT - NOTCH_OFFSET;

      const margin = 10;

      if (menuLeftViewport < margin) {
        menuLeftViewport = margin;
      }
      if (menuLeftViewport + menuWidth > window.innerWidth - margin) {
        menuLeftViewport = window.innerWidth - menuWidth - margin;
      }

      if (menuTopViewport < margin) {
        menuTopViewport = margin;
      }

      menu.style.left = `${menuLeftViewport}px`;
      menu.style.top = `${menuTopViewport}px`;
    },
    [],
  );

  const showStandardMenu = useCallback(
    (
      selection: Selection,
      overrideRect?: { left: number; width: number; top: number },
    ) => {
      setMenuActive(true);
      onMenuShow?.();

      window.dispatchEvent(
        new CustomEvent("global-inline-menu-show", { detail: { id: hookId } }),
      );

      const menu = menuRef.current;
      if (!menu) return;

      menu.classList.remove("animating-layout");
      renderPage(0, false);

      let targetRect;
      if (overrideRect) {
        targetRect = overrideRect;
      } else {
        const range = selection.getRangeAt(0);

        if (positioningStrategy === "simple") {
          const r = range.getBoundingClientRect();
          targetRect = {
            left: r.left,
            top: r.top,
            width: r.width,
          };
        } else {
          const rects = Array.from(range.getClientRects()).filter(
            (r) => r.width > 0 && r.height > 0,
          );

          if (rects.length === 0) {
            const r = range.getBoundingClientRect();
            targetRect = {
              left: r.left,
              top: r.top,
              width: r.width,
            };
          } else {
            const topY = Math.min(...rects.map((r) => r.top));

            const minLeft = Math.min(...rects.map((r) => r.left));
            const maxRight = Math.max(...rects.map((r) => r.right));
            const width = maxRight - minLeft;

            targetRect = {
              left: minLeft,
              top: topY,
              width,
            };
          }
        }
      }

      if (targetRect.width > 0) {
        positionMenu(targetRect, true);
      }

      void menu.offsetWidth;
      requestAnimationFrame(() => {
        if (menuRef.current) menuRef.current.classList.add("active");
      });
    },
    [positionMenu, renderPage, onMenuShow, hookId, positioningStrategy],
  );

  const showFlatMenu = useCallback(
    (targetRect: { left: number; width: number; top: number }) => {
      setMenuActive(true);
      setIsSelectAllMode(true);
      onMenuShow?.();

      window.dispatchEvent(
        new CustomEvent("global-inline-menu-show", { detail: { id: hookId } }),
      );

      const menu = menuRef.current;
      if (!menu) return;

      menu.classList.remove("animating-layout");
      renderPage(2, false);

      positionMenu(targetRect, false);
      void menu.offsetWidth;
      requestAnimationFrame(() => {
        if (menuRef.current) menuRef.current.classList.add("active");
      });
    },
    [positionMenu, renderPage, onMenuShow, hookId],
  );

  const switchPage = useCallback(
    (targetIndex: number) => {
      const menu = menuRef.current;
      if (!menu) return;

      menu.classList.add("animating-layout");

      const oldWidth = parseFloat(menu.style.width) || menu.offsetWidth;
      const p1W = page1Ref.current?.offsetWidth || 0;
      const p2W = page2Ref.current?.offsetWidth || 0;

      let newWidth = 0;
      if (targetIndex === 0) newWidth = p1W;
      if (targetIndex === 1) newWidth = p2W;

      const widthDiff = newWidth - oldWidth;
      const currentLeft = parseFloat(menu.style.left) || 0;
      const newLeft = currentLeft - widthDiff / 2;

      const margin = 10;
      let clampedLeft = newLeft;
      if (clampedLeft < margin) clampedLeft = margin;
      if (clampedLeft + newWidth > window.innerWidth - margin) {
        clampedLeft = window.innerWidth - newWidth - margin;
      }

      menu.style.width = `${newWidth}px`;
      menu.style.left = `${clampedLeft}px`;

      renderPage(targetIndex, true);
    },
    [renderPage],
  );

  const handleAction = useCallback(
    (action: string) => {
      if (action === "selectAll") {
        if (onSelectAll) {
          onSelectAll();
        } else {
          const text = getSelectedText();
          if (text) navigator.clipboard.writeText(text);
          hideMenu();
        }
      } else {
        const text = getSelectedText();
        if (action === "copy") {
          if (text) navigator.clipboard.writeText(text);
        } else if (action === "search") {
          if (text)
            invoke("open_external_url", { url: generateSearchUrl(text) });
        } else if (action === "translate") {
          if (text)
            invoke("open_external_url", { url: generateTranslateUrl(text) });
        }
        hideMenu();
      }
    },
    [hideMenu, onSelectAll, getSelectedText],
  );

  const handleSelection = useCallback(() => {
    if (isSelectAllMode) return;

    const activeEl = document.activeElement;
    if (
      activeEl &&
      (activeEl.tagName === "TEXTAREA" || activeEl.tagName === "INPUT")
    ) {
      return;
    }

    const selection = window.getSelection();
    const text = selection?.toString().trim();

    if (
      containerRef.current &&
      selection?.anchorNode &&
      !containerRef.current.contains(selection.anchorNode)
    ) {
      return;
    }

    if (!text || !selection?.rangeCount) {
      if (menuActive) hideMenu();
      return;
    }

    showStandardMenu(selection);
  }, [isSelectAllMode, menuActive, hideMenu, showStandardMenu, containerRef]);

  useEffect(() => {
    const onGlobalShow = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail && detail.id !== hookId && menuActive) {
        hideMenu();
      }
    };
    window.addEventListener("global-inline-menu-show", onGlobalShow);

    if (!containerElement) {
      return () =>
        window.removeEventListener("global-inline-menu-show", onGlobalShow);
    }

    const onMouseUp = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (menuRef.current && menuRef.current.contains(target)) return;
      setTimeout(() => handleSelection(), 10);
    };

    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key.includes("Arrow")) {
        setTimeout(() => handleSelection(), 10);
      }
    };

    const onResize = () => hideMenu();

    const onMouseDown = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (menuRef.current && !menuRef.current.contains(target)) {
        if (!target.classList.contains("selectable-text")) {
          if (menuActive) {
            hideMenu();
            if (containerElement.contains(target)) {
              window.getSelection()?.removeAllRanges();
            }
          }
        }
      }
    };

    containerElement.addEventListener("mouseup", onMouseUp);
    document.addEventListener("keyup", onKeyUp);
    window.addEventListener("resize", onResize);
    document.addEventListener("mousedown", onMouseDown);

    const onSelectionChange = () => {
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed) {
        if (menuActive) hideMenu();
        return;
      }

      if (
        containerElement &&
        selection.anchorNode &&
        !containerElement.contains(selection.anchorNode)
      ) {
        if (menuActive) hideMenu();
      }
    };
    document.addEventListener("selectionchange", onSelectionChange);

    return () => {
      window.removeEventListener("global-inline-menu-show", onGlobalShow);
      containerElement.removeEventListener("mouseup", onMouseUp);
      document.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("resize", onResize);
      document.removeEventListener("mousedown", onMouseDown);
      document.removeEventListener("selectionchange", onSelectionChange);
    };
  }, [
    menuActive,
    isSelectAllMode,
    handleSelection,
    hideMenu,
    containerElement,
    hookId,
  ]);

  return {
    menuRef,
    sliderRef,
    page1Ref,
    page2Ref,
    pageFlatRef,
    menuActive,
    isSelectAllMode,
    hideMenu,
    showStandardMenu,
    showFlatMenu,
    handleAction,
    switchPage,
    getSelectedText,
    renderPage,
    positionMenu,
  };
};

export const useInlineMenu = useTextSelection;
