/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef, useEffect, useState, ReactNode } from "react";
import { ChevronDown, Check } from "lucide-react";
import styles from "./Dropdown.module.css";

interface DropdownItemProps {
  label: string;
  onClick: () => void;
  isActive?: boolean;
}

export const DropdownItem: React.FC<DropdownItemProps> = ({
  label,
  onClick,
  isActive,
}) => (
  <button
    className={`${styles.item} ${isActive ? styles.itemActive : ""}`}
    onClick={onClick}
  >
    <span>{label}</span>
    {isActive && <Check size={14} className={styles.checkIcon} />}
  </button>
);

export const DropdownDivider: React.FC = () => (
  <div className={styles.divider} />
);

export const DropdownSectionTitle: React.FC<{ children: ReactNode }> = ({
  children,
}) => <div className={styles.sectionTitle}>{children}</div>;

interface DropdownActionProps {
  icon?: ReactNode;
  children: ReactNode;
  onClick: () => void;
}

export const DropdownAction: React.FC<DropdownActionProps> = ({
  icon,
  children,
  onClick,
}) => (
  <button className={styles.actionButton} onClick={onClick}>
    {icon}
    <span>{children}</span>
  </button>
);

interface DropdownProps {
  label: ReactNode;
  children: ReactNode;
  className?: string;
  width?: number | string;
  isOpen?: boolean;
  onOpenChange?: (isOpen: boolean) => void;
  hideChevron?: boolean;
  direction?: "up" | "down";
  align?: "left" | "right";
}

export const Dropdown: React.FC<DropdownProps> = ({
  label,
  children,
  className = "",
  width = 200,
  isOpen: controlledOpen,
  onOpenChange,
  hideChevron = false,
  direction = "down",
  align = "right",
}) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const isControlled = controlledOpen !== undefined;
  const isOpen = isControlled ? controlledOpen : internalOpen;

  const containerRef = useRef<HTMLDivElement>(null);

  const handleOpenChange = (newOpen: boolean) => {
    if (!isControlled) {
      setInternalOpen(newOpen);
    }
    onOpenChange?.(newOpen);
  };

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        handleOpenChange(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  const isUp = direction === "up";

  const chevron = !hideChevron && (
    <ChevronDown
      size={18}
      className={`${isUp ? styles.chevronRotate : "styles.chevronClr"} ${styles.chevron} ${isOpen ? (isUp ? styles.chevronReturn : styles.chevronRotate) : ""}`}
    />
  );

  return (
    <div className={`${styles.container} ${className}`} ref={containerRef}>
      <button
        className={`${styles.triggerGlobal} ${isUp ? styles.triggerUp : styles.trigger} ${isOpen ? styles.active : ""}`}
        onClick={() => handleOpenChange(!isOpen)}
      >
        {isUp && chevron}
        {<span className={`${isUp ? "" : styles.label}`}>{label}</span>}
        {!isUp && chevron}
      </button>
      <div
        className={`${styles.dropdown} ${isUp ? styles.dropdownUp : ""} ${isOpen ? styles.dropdownOpen : ""} ${align === "left" ? styles.alignLeft : styles.alignRight}`}
        style={{ minWidth: width }}
      >
        {children}
      </div>
    </div>
  );
};
