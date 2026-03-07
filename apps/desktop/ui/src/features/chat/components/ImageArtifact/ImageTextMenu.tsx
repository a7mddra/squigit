/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useState,
  useRef,
  useCallback,
  useEffect,
  forwardRef,
  useImperativeHandle,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { generateSearchUrl, generateTranslateUrl } from "@/lib";
import { InlineMenu } from "@/components";

interface OCRBox {
  text: string;
  box: number[][];
}

interface ImageTextMenuProps {
  data: OCRBox[];
  size: { w: number; h: number };
  imgRef: React.RefObject<HTMLImageElement | null>;
  imgWrapRef: React.RefObject<HTMLDivElement | null>;
  viewerRef: React.RefObject<HTMLDivElement | null>;
}

export interface ImageTextMenuHandle {
  showStandardMenu: (selection: Selection) => void;
  hideMenu: () => void;
}

export const ImageTextMenu = forwardRef<
  ImageTextMenuHandle,
  ImageTextMenuProps
>(({ data, size, imgRef, imgWrapRef, viewerRef }, ref) => {
  const [menuActive, setMenuActive] = useState(false);
  const [isSelectAllMode, setIsSelectAllMode] = useState(false);

  const menuRef = useRef<HTMLDivElement>(null);
  const sliderRef = useRef<HTMLDivElement>(null);
  const page1Ref = useRef<HTMLDivElement>(null);
  const page2Ref = useRef<HTMLDivElement>(null);
  const pageFlatRef = useRef<HTMLDivElement>(null);

  const MENU_HEIGHT = 48;

  const getSelectedText = () => {
    return window.getSelection()?.toString().trim() || "";
  };

  const hideMenu = useCallback(() => {
    if (menuRef.current) {
      menuRef.current.classList.remove("animating-layout");
      menuRef.current.classList.remove("active");
    }
    setMenuActive(false);
    setIsSelectAllMode(false);
  }, []);

  const positionMenu = useCallback(
    (
      selectionRectViewport: {
        left: number;
        width: number;
        top: number;
      },
      _isFlatMode: boolean,
    ) => {
      const menu = menuRef.current;
      if (!menu) return;

      const selectionCenterViewport =
        selectionRectViewport.left + selectionRectViewport.width / 2;
      const selectionTopViewport = selectionRectViewport.top;

      const menuWidth = menu.offsetWidth || 180;

      let menuLeftViewport = selectionCenterViewport - menuWidth / 2;
      let menuTopViewport = selectionTopViewport - MENU_HEIGHT - 12;

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

  const showStandardMenu = useCallback(
    (selection: Selection) => {
      setMenuActive(true);

      window.dispatchEvent(
        new CustomEvent("global-inline-menu-show", {
          detail: { id: "image-text-menu" },
        }),
      );

      const menu = menuRef.current;
      if (!menu) return;

      menu.classList.remove("animating-layout");
      renderPage(0, false);

      const range = selection.getRangeAt(0);
      const getElementRect = (node: Node | null): DOMRect | null => {
        if (!node) return null;
        if (node.nodeType === Node.TEXT_NODE) {
          return node.parentElement?.getBoundingClientRect() || null;
        }
        return (node as Element).getBoundingClientRect();
      };

      const anchorRect = getElementRect(selection.anchorNode);
      const focusRect = getElementRect(selection.focusNode);
      let topY: number;
      if (anchorRect && focusRect) {
        topY = Math.min(anchorRect.top, focusRect.top);
      } else if (anchorRect) {
        topY = anchorRect.top;
      } else if (focusRect) {
        topY = focusRect.top;
      } else {
        topY = range.getBoundingClientRect().top;
      }
      const rects = Array.from(range.getClientRects());
      const tolerance = 50;
      const validRects = rects.filter((r) => {
        if (r.width === 0 || r.height === 0) return false;
        return (
          Math.abs(r.top - topY) < tolerance ||
          (r.top >= topY - tolerance && r.bottom <= topY + tolerance + 100)
        );
      });

      let left: number, width: number;
      if (validRects.length > 0) {
        left = Math.min(...validRects.map((r) => r.left));
        const right = Math.max(...validRects.map((r) => r.right));
        width = right - left;
      } else {
        const r = range.getBoundingClientRect();
        left = r.left;
        width = r.width;
      }

      const targetRect = {
        left,
        top: topY,
        width,
      };

      positionMenu(targetRect, false);
      void menu.offsetWidth;
      requestAnimationFrame(() => {
        if (menuRef.current) menuRef.current.classList.add("active");
      });
    },
    [positionMenu, renderPage],
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

  const triggerSelectAll = useCallback(() => {
    const svg = document.querySelector("[data-text-layer]");
    if (svg) {
      const selection = window.getSelection();
      if (selection) {
        selection.removeAllRanges();
        const range = document.createRange();
        range.selectNodeContents(svg);
        selection.addRange(range);
      }
    }

    setIsSelectAllMode(true);

    window.dispatchEvent(
      new CustomEvent("global-inline-menu-show", {
        detail: { id: "image-text-menu" },
      }),
    );

    const menu = menuRef.current;
    const wrap = imgWrapRef.current;
    if (!menu || !wrap) return;

    menu.classList.remove("animating-layout");
    renderPage(2, false);

    const wrapRect = wrap.getBoundingClientRect();
    const menuWidth = pageFlatRef.current?.offsetWidth || 250;

    let targetTopViewport = wrapRect.top - MENU_HEIGHT - 20;
    if (data.length > 0) {
      const firstBoxY = data.reduce(
        (min, item) => Math.min(min, item.box[0][1]),
        Infinity,
      );
      const imgRect = imgRef.current?.getBoundingClientRect();
      if (imgRect && size.h > 0) {
        const scale = imgRect.height / size.h;
        targetTopViewport = imgRect.top + firstBoxY * scale - MENU_HEIGHT - 20;
      }
    }
    if (targetTopViewport < 10) targetTopViewport = 10;

    const viewerRect = viewerRef.current?.getBoundingClientRect();
    let targetLeftViewport = window.innerWidth / 2 - menuWidth / 2;
    if (viewerRect) {
      targetLeftViewport =
        viewerRect.left + viewerRect.width / 2 - menuWidth / 2;
    }

    if (targetLeftViewport < 10) targetLeftViewport = 10;
    if (targetLeftViewport + menuWidth > window.innerWidth - 10) {
      targetLeftViewport = window.innerWidth - menuWidth - 10;
    }

    menu.style.left = `${targetLeftViewport}px`;
    menu.style.top = `${targetTopViewport}px`;

    void menu.offsetWidth;
    requestAnimationFrame(() => {
      if (menuRef.current) menuRef.current.classList.add("active");
    });
  }, [data, size.h, imgRef, imgWrapRef, viewerRef, renderPage]);

  const handleAction = useCallback(
    (action: string) => {
      if (action === "selectAll") {
        triggerSelectAll();
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
    [triggerSelectAll, hideMenu],
  );

  useImperativeHandle(
    ref,
    () => ({
      showStandardMenu,
      hideMenu,
    }),
    [showStandardMenu, hideMenu],
  );

  useEffect(() => {
    const hookId = "image-text-menu";
    const onGlobalShow = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail && detail.id !== hookId && menuActive) {
        hideMenu();
      }
    };
    window.addEventListener("global-inline-menu-show", onGlobalShow);

    const onMouseDown = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (menuRef.current && !menuRef.current.contains(target)) {
        if (imgWrapRef.current && imgWrapRef.current.contains(target)) {
          if (!target.hasAttribute("data-selectable-text")) {
            window.getSelection()?.removeAllRanges();
          }
        }
        hideMenu();
      }
    };

    const onResize = () => hideMenu();

    document.addEventListener("mousedown", onMouseDown);
    window.addEventListener("resize", onResize);

    const onSelectionChange = () => {
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed) {
        if (menuActive) hideMenu();
        return;
      }

      if (
        imgWrapRef.current &&
        selection.anchorNode &&
        !imgWrapRef.current.contains(selection.anchorNode)
      ) {
        if (menuActive) hideMenu();
      }
    };
    document.addEventListener("selectionchange", onSelectionChange);

    return () => {
      window.removeEventListener("global-inline-menu-show", onGlobalShow);
      document.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("resize", onResize);
      document.removeEventListener("selectionchange", onSelectionChange);
    };
  }, [menuActive, isSelectAllMode, hideMenu]);

  return (
    <InlineMenu
      id="image-text-menu"
      className="image-text-menu"
      menuRef={menuRef}
      sliderRef={sliderRef}
      page1Ref={page1Ref}
      page2Ref={page2Ref}
      pageFlatRef={pageFlatRef}
      onAction={handleAction}
      onSwitchPage={switchPage}
    />
  );
});

ImageTextMenu.displayName = "ImageTextMenu";

export const OCRMenu = ImageTextMenu;
export type OCRMenuHandle = ImageTextMenuHandle;
