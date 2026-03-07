/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState } from "react";
import styles from "./AuthButton.module.css";

interface AuthButtonProps {
  onLogin: () => void;
  onCancel?: () => void;
  isLoading?: boolean;
  disabled?: boolean;
  disabledTitle?: string;
}

export const AuthButton: React.FC<AuthButtonProps> = ({
  onLogin,
  onCancel,
  isLoading = false,
  disabled = false,
  disabledTitle,
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const [hasLeftSinceLoading, setHasLeftSinceLoading] = useState(false);

  const handleClick = () => {
    if (disabled) return;
    if (isLoading && onCancel && hasLeftSinceLoading && isHovered) {
      onCancel();
    } else if (!isLoading) {
      onLogin();
    }
  };

  const handleMouseEnter = () => {
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    setIsHovered(false);
    if (isLoading) {
      setHasLeftSinceLoading(true);
    }
  };

  React.useEffect(() => {
    if (!isLoading) {
      setHasLeftSinceLoading(false);
    }
  }, [isLoading]);

  const showCancel = isLoading && isHovered && hasLeftSinceLoading;

  return (
    <button
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      className={`${styles.loginBtn} ${isLoading ? styles.loading : ""} ${disabled ? styles.disabled : ""}`}
      aria-disabled={disabled}
      title={disabled ? disabledTitle : undefined}
    >
      {isLoading &&
        (showCancel ? (
          <span style={{ fontSize: "0.8rem", color: "var(--c-raw-099)" }}>Cancel ✕</span>
        ) : (
          <>
            Redirecting
            <span>
              <span className={styles.dot}>.</span>
              <span className={styles.dot}>.</span>
              <span className={styles.dot}>.</span>
            </span>
          </>
        ))}
      <span>
        {!isLoading && (
          <>
            Sign in with <span className={styles.google}>Google</span>
          </>
        )}
      </span>
    </button>
  );
};
