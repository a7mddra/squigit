/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { MoreHorizontal, Pin, Trash2, Check, X, Search } from "lucide-react";

import { ChatMetadata } from "@/lib";
import { updateIcon, welcomeIcon } from "@/assets";
import { Dialog } from "@/components";
import { getDeleteMultipleChatsDialog } from "@/lib";
import { PanelContextMenu } from "@/layout";
import { useAppContext } from "@/providers/AppProvider";
import { useKeyDown, getPendingUpdate } from "@/hooks";
import styles from "./SidePanel.module.css";

const Checkbox: React.FC<{ checked: boolean; onChange: () => void }> = ({
  checked,
  onChange,
}) => (
  <div
    className={`${styles.checkbox} ${checked ? styles.checked : ""}`}
    onClick={(e) => {
      e.stopPropagation();
      onChange();
    }}
  >
    {checked && (
      <Check size={10} className={styles.checkboxInner} strokeWidth={4} />
    )}
  </div>
);

interface ChatItemProps {
  chat: ChatMetadata;
  isActive: boolean;
  isSelectionMode: boolean;
  isSelected: boolean;
  onSelect: () => void;
  onToggleSelection: () => void;
  onDelete: () => void;
  onRename: (newTitle: string) => void;
  onTogglePin: () => void;
  activeContextMenu: { id: string; x: number; y: number } | null;
  onOpenContextMenu: (id: string, x: number, y: number) => void;
  onCloseContextMenu: () => void;
  onEnableSelectionMode: () => void;
}

const ChatItem: React.FC<ChatItemProps> = ({
  chat,
  isActive,
  isSelectionMode,
  isSelected,
  onSelect,
  onToggleSelection,
  onDelete,
  onRename,
  onTogglePin,
  activeContextMenu,
  onOpenContextMenu,
  onCloseContextMenu,
  onEnableSelectionMode,
}) => {
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(chat.title);
  const inputRef = useRef<HTMLInputElement>(null);

  const showMenu = activeContextMenu?.id === chat.id;

  useEffect(() => {
    if (isRenaming && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isRenaming]);

  const handleMenuClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    if (showMenu) {
      onCloseContextMenu();
      return;
    }
    onOpenContextMenu(chat.id, e.clientX, e.clientY);
  };

  const handleRenameSubmit = () => {
    if (renameValue.trim() && renameValue !== chat.title) {
      onRename(renameValue.trim());
    }
    setIsRenaming(false);
  };

  const handleRenameKeyDown = useKeyDown({
    Enter: handleRenameSubmit,
    Escape: () => {
      setRenameValue(chat.title);
      setIsRenaming(false);
    },
  });

  return (
    <>
      <div
        className={`${styles.chatRow} ${isActive ? styles.active : ""} ${showMenu ? styles.menuOpen : ""}`}
        onClick={isSelectionMode ? onToggleSelection : onSelect}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onOpenContextMenu(chat.id, e.clientX, e.clientY);
        }}
      >
        {isSelectionMode && (
          <Checkbox checked={isSelected} onChange={onToggleSelection} />
        )}

        {isRenaming ? (
          <input
            ref={inputRef}
            className={styles.chatTitleInput}
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onBlur={handleRenameSubmit}
            onKeyDown={handleRenameKeyDown}
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span
            className={styles.chatTitle}
            onDoubleClick={(e) => {
              e.stopPropagation();
              e.preventDefault();
              setIsRenaming(true);
            }}
          >
            {chat.title}
          </span>
        )}

        {!isSelectionMode && (
          <div className={styles.chatActions}>
            <div className={styles.statusRow}>
              <button
                className={`${styles.starBtn} ${chat.is_pinned ? styles.pinned : ""}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onTogglePin();
                }}
                title={chat.is_pinned ? "Unpin" : "Pin"}
              >
                {chat.is_pinned ? (
                  <Pin
                    size={14}
                    style={{ transform: "rotate(45deg)" }}
                    fill="currentColor"
                  />
                ) : (
                  <Pin size={14} style={{ transform: "rotate(45deg)" }} />
                )}
              </button>

              <button
                className={styles.menuBtn}
                onMouseDown={(e) => {
                  e.stopPropagation();
                  e.preventDefault();
                }}
                onClick={handleMenuClick}
              >
                <MoreHorizontal size={14} />
              </button>
            </div>
          </div>
        )}
      </div>

      {showMenu && activeContextMenu && (
        <PanelContextMenu
          x={activeContextMenu.x}
          y={activeContextMenu.y}
          onClose={onCloseContextMenu}
          onRename={() => {
            setIsRenaming(true);
          }}
          onToggleSelection={() => {
            onEnableSelectionMode();
            if (!isSelected) {
              onToggleSelection();
            }
          }}
          onDelete={() => {
            onDelete();
          }}
          isSelected={isSelected}
        />
      )}
    </>
  );
};

export const SidePanel: React.FC = () => {
  const app = useAppContext();
  const chats = app.chatHistory.chats;
  const activeSessionId = app.chatHistory.activeSessionId;

  const [isSelectionMode, setIsSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [activeContextMenu, setActiveContextMenu] = useState<{
    id: string;
    x: number;
    y: number;
  } | null>(null);

  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [showBulkDelete, setShowBulkDelete] = useState(false);

  // Directly filter out system chats, no grouping needed
  const displayChats = useMemo(
    () =>
      chats
        .filter((c: any) => !c.id.startsWith("__system_"))
        // Sort pinned chats first, then by updated_at descending
        .sort((a, b) => {
          if (a.is_pinned && !b.is_pinned) return -1;
          if (!a.is_pinned && b.is_pinned) return 1;
          return (
            new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
          );
        }),
    [chats],
  );

  const update = getPendingUpdate();
  const showWelcome =
    !app.system.activeProfile && app.system.hasAgreed === false;

  const handleOpenContextMenu = useCallback(
    (id: string, x: number, y: number) => {
      const xPos = x + 180 > window.innerWidth ? x - 180 : x;
      setActiveContextMenu((prev) => {
        if (prev && prev.id === id && prev.x === xPos && prev.y === y) {
          return prev;
        }
        return { id, x: xPos, y };
      });
    },
    [],
  );

  const handleCloseContextMenu = useCallback(() => {
    setActiveContextMenu((prev) => (prev === null ? prev : null));
  }, []);

  const toggleSelectionMode = () => {
    setIsSelectionMode(!isSelectionMode);
    setSelectedIds([]);
  };

  const toggleChatSelection = (id: string) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((i) => i !== id) : [...prev, id],
    );
  };

  const selectAll = () => {
    setSelectedIds(
      selectedIds.length === chats.length ? [] : chats.map((c: any) => c.id),
    );
  };

  const handleDeleteChat = () => {
    if (deleteId) {
      app.handleDeleteChatWrapper(deleteId);
      setDeleteId(null);
    }
  };

  const handleBulkDelete = () => {
    app.handleDeleteChatsWrapper(selectedIds);
    setSelectedIds([]);
    setIsSelectionMode(false);
    setShowBulkDelete(false);
  };

  return (
    <div className={styles.panel}>
      {isSelectionMode ? (
        <div className={styles.selectionHeader}>
          <div className={styles.selectionLeft}>
            <Checkbox
              checked={selectedIds.length === chats.length && chats.length > 0}
              onChange={selectAll}
            />
            <span className={styles.labelAll}>All</span>
          </div>

          <div className={styles.selectionCenter}>
            <span className={styles.selectionCount}>
              {selectedIds.length} selected
            </span>
          </div>

          <div className={styles.selectionRight}>
            <button
              className={`${styles.iconBtn} ${styles.danger}`}
              onClick={() => selectedIds.length > 0 && setShowBulkDelete(true)}
              style={{ color: "var(--c-raw-015)" }}
              disabled={selectedIds.length === 0}
            >
              <Trash2 size={16} />
            </button>
            <button className={styles.iconBtn} onClick={toggleSelectionMode}>
              <X size={18} />
            </button>
          </div>
        </div>
      ) : (
        <div className={styles.headerArea}>
          <div className={styles.groupContent}>
            <div className={styles.groupInner}>
              <div className={styles.chatRow} onClick={app.handleNewSession}>
                <div className={styles.chatIconMain}>
                  <svg
                    width="18"
                    height="18"
                    viewBox="0 0 121 118"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                    aria-hidden="true"
                  >
                    <path
                      d="M104.48 72.7959V91.8586C104.48 103.457 95.078 112.859 83.4801 112.859H25.4675C13.8696 112.859 4.46753 103.457 4.46753 91.8586V37.3047C4.46753 25.7067 13.8695 16.3047 25.4675 16.3047H47.4124"
                      stroke="currentColor"
                      strokeWidth="8.93484"
                      strokeLinecap="round"
                    />
                    <path
                      d="M43.1669 53.6424L81.7635 16.6424C87.9474 10.7144 97.7666 10.9223 103.695 17.1063C109.623 23.2902 109.415 33.1088 103.232 39.0369L64.7774 75.9004C61.9576 78.6034 58.3471 80.3334 54.4737 80.8374L35.8215 83.2635C34.9464 83.3772 34.2154 82.6027 34.3799 81.7356L37.8515 63.4429C38.5612 59.7029 40.4188 56.2767 43.1669 53.6424Z"
                      stroke="currentColor"
                      strokeWidth="8.3584"
                    />
                  </svg>
                </div>
                <span className={styles.chatTitle}>New thread</span>
              </div>

              <div
                className={`${styles.chatRow} ${app.searchOverlay.isOpen ? styles.active : ""}`}
                onClick={app.openSearchOverlay}
              >
                <div className={styles.chatIconMain}>
                  <Search size={19} />
                </div>
                <span className={styles.chatTitle}>Search chats</span>
              </div>

              <div
                className={`${styles.chatRow} ${activeSessionId === "__system_gallery" ? styles.active : ""}`}
                onClick={() => app.handleSelectChat("__system_gallery")}
              >
                <div className={styles.chatIconMain}>
                  <svg
                    width="22"
                    height="22"
                    viewBox="0 0 138 149"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                    aria-hidden="true"
                  >
                    <path
                      d="M19.4877 37.4318L84.6124 33.8414C92.9872 33.3798 100.151 39.795 100.613 48.1698L103.748 105.039C104.21 113.414 97.7947 120.578 89.4199 121.04L24.2951 124.63C15.9203 125.092 8.75652 118.677 8.2946 110.302L5.15924 53.4324C4.69768 45.0575 11.1129 37.8937 19.4877 37.4318Z"
                      stroke="currentColor"
                      stroke-width="8.20785"
                    />
                    <path
                      d="M31.7285 25.4662L31.7285 23.3972C31.7285 11.9698 41.6062 3.04716 52.9748 4.20517L115.808 10.6054C126.407 11.685 134.125 21.1528 133.045 31.7523L126.381 97.1711C125.778 103.098 120.787 107.606 114.829 107.606"
                      stroke="currentColor"
                      stroke-width="8.20785"
                      strokeLinecap="round"
                    />
                    <path
                      d="M98.5279 117.328L49.9828 74.7361C42.6208 68.2769 31.587 68.3591 24.3221 74.9275L6.88965 90.6885"
                      stroke="currentColor"
                      stroke-width="8.20785"
                    />
                    <circle
                      cx="73.0312"
                      cy="59.4555"
                      r="8.72703"
                      fill="currentColor"
                    />
                  </svg>
                </div>
                <span className={styles.chatTitle}>Images</span>
              </div>

              {showWelcome && (
                <div
                  className={`${styles.chatRow} ${activeSessionId === "__system_welcome" ? styles.active : ""}`}
                  onClick={() => app.handleSelectChat("__system_welcome")}
                >
                  <div className={styles.chatIconMain}>
                    <img
                      src={welcomeIcon}
                      alt="Welcome"
                      className="w-5 h-5 object-contain"
                    />
                  </div>
                  <span className={styles.chatTitle}>
                    Welcome to {app.system.appName}!
                  </span>
                </div>
              )}

              {update && (
                <div
                  className={`${styles.chatRow} ${activeSessionId && activeSessionId.startsWith("__system_update") ? styles.active : ""}`}
                  onClick={() =>
                    app.handleSelectChat(`__system_update_${update.version}`)
                  }
                >
                  <div className={styles.chatIconMain}>
                    <img
                      src={updateIcon}
                      alt="Update"
                      className="w-5 h-5 object-contain"
                    />
                  </div>
                  <span className={styles.chatTitle}>
                    Update Available: {update.version}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      <div className={styles.scrollArea}>
        <div className={styles.groupInner}>
          {displayChats.length > 0 ? (
            displayChats.map((chat) => (
              <ChatItem
                key={chat.id}
                chat={chat}
                isActive={chat.id === activeSessionId}
                isSelectionMode={isSelectionMode}
                isSelected={selectedIds.includes(chat.id)}
                onSelect={() => {
                  if (chat.id !== activeSessionId) {
                    app.handleSelectChat(chat.id);
                  }
                }}
                onToggleSelection={() => toggleChatSelection(chat.id)}
                onDelete={() => setDeleteId(chat.id)}
                onRename={(newTitle) =>
                  app.chatHistory.handleRenameChat(chat.id, newTitle)
                }
                onTogglePin={() => app.chatHistory.handleTogglePinChat(chat.id)}
                activeContextMenu={activeContextMenu}
                onOpenContextMenu={handleOpenContextMenu}
                onCloseContextMenu={handleCloseContextMenu}
                onEnableSelectionMode={() => setIsSelectionMode(true)}
              />
            ))
          ) : (
            <div
              style={{
                padding: "20px 20px",
                textAlign: "center",
                color: "var(--c-raw-073)",
                fontSize: "0.9rem",
              }}
            >
              No chats found
            </div>
          )}
        </div>
      </div>

      <Dialog
        isOpen={!!deleteId}
        type="DELETE_CHAT"
        appName={app.system.appName}
        onAction={(key) => {
          if (key === "confirm") handleDeleteChat();
          else setDeleteId(null);
        }}
      />

      <Dialog
        isOpen={showBulkDelete}
        type={getDeleteMultipleChatsDialog(selectedIds.length)}
        appName={app.system.appName}
        onAction={(key) => {
          if (key === "confirm") handleBulkDelete();
          else setShowBulkDelete(false);
        }}
      />
    </div>
  );
};
