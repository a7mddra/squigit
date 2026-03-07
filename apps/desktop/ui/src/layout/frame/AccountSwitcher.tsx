/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useRef, useEffect, useState } from "react";
import { Profile } from "@/lib";
import { Dialog } from "@/components";
import { Avatar } from "./Avatar";
import {
  ChevronDown,
  UserPlus,
  LogOut,
  Check,
  Loader2,
  Trash2,
} from "lucide-react";
import styles from "./AccountSwitcher.module.css";

interface AccountSwitcherProps {
  activeProfile: Profile | null;
  profiles: Profile[];
  onSwitchProfile: (profileId: string) => void;
  onNewSession: () => void;
  onAddAccount: () => void;
  onLogout: () => void;
  onDeleteProfile: (profileId: string) => Promise<void>;
  switchingProfileId?: string | null;
}

export const AccountSwitcher: React.FC<AccountSwitcherProps> = ({
  activeProfile,
  profiles,
  onSwitchProfile,
  onAddAccount,
  onLogout,
  onDeleteProfile,
  switchingProfileId,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [profileToDelete, setProfileToDelete] = useState<string | null>(null);
  const [deletingProfileId, setDeletingProfileId] = useState<string | null>(
    null,
  );
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (profileToDelete) return;

      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen, profileToDelete]);

  const handleConfirmDelete = () => {
    if (profileToDelete) {
      const id = profileToDelete;
      setProfileToDelete(null);
      setDeletingProfileId(id);

      setTimeout(async () => {
        try {
          await onDeleteProfile(id);
        } finally {
          setDeletingProfileId(null);
        }
      }, 50);
    }
  };

  const isSwitching = !!switchingProfileId;

  return (
    <div className={styles.accountSwitcher} ref={containerRef}>
      <button
        className={`${styles.trigger} ${styles.triggerGlobal} ${isOpen ? styles.active : ""}`}
        onClick={() => !isSwitching && setIsOpen(!isOpen)}
        disabled={isSwitching}
      >
        <div className={styles.avatar}>
          <Avatar
            src={activeProfile?.avatar}
            fallbackSrc={activeProfile?.original_avatar}
            name={activeProfile?.name ?? "User"}
            size="100%"
            profileId={activeProfile?.id}
          />
        </div>
        {isSwitching ? (
          <Loader2
            size={18}
            className={`${styles.chevron} ${styles.chevronClr} ${styles.spin}`}
          />
        ) : (
          <ChevronDown
            size={18}
            className={`${styles.chevron} ${styles.chevronClr} ${isOpen ? styles.rotate : ""}`}
          />
        )}
      </button>

      <div className={`${styles.dropdown} ${isOpen ? styles.open : ""}`}>
        <div className={styles.sectionTitle}>Switch Account</div>
        <div className={styles.accountList}>
          {profiles.map((profile) => {
            const isActive = activeProfile?.id === profile.id;
            const isDeleting = deletingProfileId === profile.id;
            const isConfirming = profileToDelete === profile.id;
            const showLoader = isDeleting || isConfirming;

            return (
              <div
                key={profile.id}
                className={`${styles.accountItem} ${
                  isActive ? styles.activeAccount : ""
                }`}
                style={{
                  pointerEvents: showLoader ? "none" : "auto",
                  opacity: isDeleting ? 0.7 : 1,
                }}
                onClick={() => {
                  if (activeProfile?.id !== profile.id && !showLoader) {
                    setIsOpen(false);
                    onSwitchProfile(profile.id);
                  }
                }}
              >
                <div className={styles.itemAvatar}>
                  <Avatar
                    src={profile.avatar}
                    fallbackSrc={profile.original_avatar}
                    name={profile.name}
                    size="100%"
                    profileId={profile.id}
                  />
                </div>
                <div className={styles.accountInfo}>
                  <span className={styles.accountName}>{profile.name}</span>
                  <span className={styles.accountEmail}>{profile.email}</span>
                </div>
                {isActive ? (
                  <Check size={14} className={styles.checkIcon} />
                ) : (
                  <button
                    className={`${styles.deleteButton} ${
                      showLoader ? styles.loading : ""
                    }`}
                    onClick={(e) => {
                      e.stopPropagation();
                      if (!showLoader) {
                        setProfileToDelete(profile.id);
                      }
                    }}
                    title={showLoader ? "Removing..." : "Remove account"}
                    disabled={showLoader}
                  >
                    {showLoader ? (
                      <Loader2 size={14} className={styles.spin} />
                    ) : (
                      <Trash2 size={14} />
                    )}
                  </button>
                )}
              </div>
            );
          })}
        </div>

        <div className={styles.divider} />

        <div className={styles.actions}>
          <button
            className={styles.actionButton}
            onClick={() => {
              onAddAccount();
              setIsOpen(false);
            }}
          >
            <UserPlus size={16} />
            <span>Add another account</span>
          </button>

          <button
            className={styles.actionButton}
            onClick={() => {
              onLogout();
              setIsOpen(false);
            }}
          >
            <LogOut size={16} />
            <span>Sign out</span>
          </button>
        </div>
      </div>

      <Dialog
        isOpen={!!profileToDelete}
        type="REMOVE_ACCOUNT"
        onAction={(key) => {
          if (key === "confirm") handleConfirmDelete();
          else setProfileToDelete(null);
        }}
      />
    </div>
  );
};
