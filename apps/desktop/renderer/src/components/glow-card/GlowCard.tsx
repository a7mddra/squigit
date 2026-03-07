/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef } from "react";
import styles from "./GlowCard.module.css";

interface GlowCardProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
}

export const GlowCard: React.FC<GlowCardProps> = ({
  children,
  className,
  ...props
}) => {
  const divRef = useRef<HTMLButtonElement>(null);

  const handleMouseMove = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (!divRef.current) return;

    const rect = divRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    divRef.current.style.setProperty("--mouse-x", `${x}px`);
    divRef.current.style.setProperty("--mouse-y", `${y}px`);
  };

  return (
    <button
      ref={divRef}
      onMouseMove={handleMouseMove}
      className={`${styles.container} ${className || ""}`}
      {...props}
    >
      <span className={styles.inner}>
        <div className={styles.content}>{children}</div>
      </span>
    </button>
  );
};
