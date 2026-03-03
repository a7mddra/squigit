/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useRef,
  useLayoutEffect,
  useState,
  ReactNode,
  useEffect,
} from "react";
import { createPortal } from "react-dom";
import styles from "./ContextMenu.module.css";

interface ContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  children: ReactNode;
  width?: number;
}

const EDGE_PADDING = 8;
const CURSOR_OFFSET = 4;

export const ContextMenu: React.FC<ContextMenuProps> = ({
  x,
  y,
  onClose,
  children,
  width,
}) => {
  const [position, setPosition] = useState({ x, y });
  const [isPositioned, setIsPositioned] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (!menuRef.current) return;

    const menuRect = menuRef.current.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    let safeX = x + CURSOR_OFFSET;
    let safeY = y + CURSOR_OFFSET;

    if (x + menuRect.width + EDGE_PADDING > viewportWidth) {
      safeX = x - menuRect.width - CURSOR_OFFSET;
    }

    if (safeX < EDGE_PADDING) {
      safeX = EDGE_PADDING;
    }

    if (y + menuRect.height + EDGE_PADDING > viewportHeight) {
      safeY = y - menuRect.height - CURSOR_OFFSET;
    }

    if (safeY < EDGE_PADDING) {
      safeY = EDGE_PADDING;
    }

    setPosition({ x: safeX, y: safeY });
    setIsPositioned(true);
  }, [x, y]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [onClose]);

  return createPortal(
    <div
      ref={menuRef}
      className={styles.contextMenu}
      style={{
        position: "fixed",
        left: `${position.x}px`,
        top: `${position.y}px`,
        opacity: isPositioned ? 1 : 0,
        zIndex: 10000,
        ...(width ? { width } : {}),
      }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => {
        e.preventDefault();
      }}
      data-is-context-menu="true"
    >
      {children}
    </div>,
    document.body,
  );
};

interface ContextMenuItemProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: ReactNode;
  variant?: "default" | "danger";
  shortcut?: ReactNode;
}

export const ContextMenuItem: React.FC<ContextMenuItemProps> = ({
  children,
  icon,
  className,
  variant = "default",
  shortcut,
  ...props
}) => {
  return (
    <button
      className={`${styles.contextMenuItem} ${variant === "danger" ? styles.danger : ""} ${className || ""}`}
      {...props}
    >
      {icon}
      {children}
      {shortcut && <span className={styles.shortcut}>{shortcut}</span>}
    </button>
  );
};

export const ContextMenuSeparator: React.FC = () => (
  <div className={styles.separator} />
);
