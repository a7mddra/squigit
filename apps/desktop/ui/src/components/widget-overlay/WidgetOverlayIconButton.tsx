/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef, useState } from "react";
import { Tooltip } from "../tooltip/Tooltip";
import styles from "./WidgetOverlay.module.css";

interface WidgetOverlayIconButtonProps {
  icon: React.ReactNode;
  label: string;
  isActive?: boolean;
  onClick: () => void;
  className?: string;
  activeClassName?: string;
}

const buildClassName = ({
  isActive,
  className,
  activeClassName,
}: {
  isActive: boolean;
  className?: string;
  activeClassName?: string;
}) =>
  [
    styles.sidebarButton,
    isActive ? styles.active : "",
    className || "",
    isActive ? activeClassName || "" : "",
  ]
    .filter(Boolean)
    .join(" ");

export const WidgetOverlayIconButton: React.FC<WidgetOverlayIconButtonProps> = ({
  icon,
  label,
  isActive = false,
  onClick,
  className,
  activeClassName,
}) => {
  const [hover, setHover] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);

  return (
    <>
      <button
        ref={btnRef}
        className={buildClassName({ isActive, className, activeClassName })}
        onClick={onClick}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
      >
        {icon}
      </button>
      <Tooltip text={label} parentRef={btnRef} show={hover} />
    </>
  );
};
