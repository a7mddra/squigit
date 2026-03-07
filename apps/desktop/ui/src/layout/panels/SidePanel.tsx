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
import {
  MoreHorizontal,
  Pin,
  Trash2,
  Check,
  X,
  Search,
  FolderOpen,
} from "lucide-react";

import { ChatMetadata } from "@/lib";
import { Dialog, LoadingSpinner } from "@/components";
import { getDeleteMultipleChatsDialog } from "@/lib";
import { PanelContextMenu } from "@/layout";
import { useAppContext } from "@/providers/AppProvider";
import { useKeyDown } from "@/hooks";
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

const SYSTEM_PREFIX = "__system_";

const formatThreadAge = (isoDate: string): string => {
  const parsed = new Date(isoDate);
  if (Number.isNaN(parsed.getTime())) return "";

  const elapsedMs = Math.max(0, Date.now() - parsed.getTime());
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  const month = 30 * day;
  const year = 365 * day;

  if (elapsedMs >= year) return `${Math.floor(elapsedMs / year)}y`;
  if (elapsedMs >= month) return `${Math.floor(elapsedMs / month)}m`;
  if (elapsedMs >= day) return `${Math.floor(elapsedMs / day)}d`;
  if (elapsedMs >= hour) return `${Math.floor(elapsedMs / hour)}h`;
  return `${Math.max(1, Math.floor(elapsedMs / minute))}m`;
};

interface ChatItemProps {
  chat: ChatMetadata;
  isActive: boolean;
  isBusy: boolean;
  isSelectionMode: boolean;
  isSelected: boolean;
  menuState: { x: number; y: number } | null;
  onSelectChat: (chatId: string) => void;
  onToggleSelectionChat: (chatId: string) => void;
  onDeleteChat: (chatId: string) => void;
  onRenameChat: (chatId: string, newTitle: string) => void;
  onTogglePinChat: (chatId: string) => void;
  onOpenContextMenu: (id: string, x: number, y: number) => void;
  onCloseContextMenu: () => void;
  onEnableSelectionMode: () => void;
}

const ChatItem: React.FC<ChatItemProps> = React.memo(({
  chat,
  isActive,
  isBusy,
  isSelectionMode,
  isSelected,
  menuState,
  onSelectChat,
  onToggleSelectionChat,
  onDeleteChat,
  onRenameChat,
  onTogglePinChat,
  onOpenContextMenu,
  onCloseContextMenu,
  onEnableSelectionMode,
}) => {
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(chat.title);
  const inputRef = useRef<HTMLInputElement>(null);

  const showMenu = !!menuState;

  useEffect(() => {
    if (isRenaming && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isRenaming]);

  useEffect(() => {
    if (!isRenaming) {
      setRenameValue(chat.title);
    }
  }, [chat.title, isRenaming]);

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
      onRenameChat(chat.id, renameValue.trim());
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

  const lastActivityAt = chat.updated_at || chat.created_at;
  const lastActivityLabel = formatThreadAge(lastActivityAt);
  const lastActivityTitle = useMemo(
    () => new Date(lastActivityAt).toLocaleString(),
    [lastActivityAt],
  );

  return (
    <>
      <div
        className={`${styles.chatRow} ${chat.is_pinned ? styles.pinnedRow : ""} ${isActive ? styles.active : ""} ${showMenu ? styles.menuOpen : ""}`}
        onClick={
          isSelectionMode
            ? () => onToggleSelectionChat(chat.id)
            : () => onSelectChat(chat.id)
        }
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onOpenContextMenu(chat.id, e.clientX, e.clientY);
        }}
      >
        {isSelectionMode && (
          <Checkbox
            checked={isSelected}
            onChange={() => onToggleSelectionChat(chat.id)}
          />
        )}

        {!isSelectionMode && (
          <div className={styles.chatLeading}>
            {isBusy ? (
              <span className={styles.rowSpinner} aria-hidden="true">
                <span className={styles.rowSpinnerInner}>
                  <LoadingSpinner />
                </span>
              </span>
            ) : (
              <>
                <FolderOpen size={19} className={styles.chatBubbleIcon} />
                <button
                  className={`${styles.pinLeftBtn} ${chat.is_pinned ? styles.pinActive : ""}`}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    onTogglePinChat(chat.id);
                  }}
                  title={chat.is_pinned ? "Unpin" : "Pin"}
                >
                  <Pin size={17} style={{ transform: "rotate(45deg)" }} />
                </button>
              </>
            )}
          </div>
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
            {chat.is_pinned && (
              <span
                className={styles.chatDate}
                title={lastActivityTitle}
              >
                {lastActivityLabel}
              </span>
            )}

            <button
              className={`${styles.menuBtn} ${chat.is_pinned ? styles.pinnedMenuBtn : ""}`}
              onMouseDown={(e) => {
                e.stopPropagation();
                e.preventDefault();
              }}
              onClick={handleMenuClick}
            >
              <MoreHorizontal size={14} />
            </button>
          </div>
        )}
      </div>

      {showMenu && menuState && (
        <PanelContextMenu
          x={menuState.x}
          y={menuState.y}
          onClose={onCloseContextMenu}
          onRename={() => {
            setIsRenaming(true);
          }}
          onToggleSelection={() => {
            onEnableSelectionMode();
            if (!isSelected) {
              onToggleSelectionChat(chat.id);
            }
          }}
          onDelete={() => {
            onDeleteChat(chat.id);
          }}
          isSelected={isSelected}
        />
      )}
    </>
  );
});

export const SidePanel: React.FC = () => {
  const app = useAppContext();
  const chats = app.chatHistory.chats;
  const activeSessionId = app.chatHistory.activeSessionId;

  const [isSelectionMode, setIsSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const [activeContextMenu, setActiveContextMenu] = useState<{
    id: string;
    x: number;
    y: number;
  } | null>(null);

  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const pinToggleLockRef = useRef<Set<string>>(new Set());
  const selectChatRef = useRef(app.handleSelectChat);
  const renameChatRef = useRef(app.chatHistory.handleRenameChat);
  const togglePinRef = useRef(app.chatHistory.handleTogglePinChat);

  useEffect(() => {
    selectChatRef.current = app.handleSelectChat;
    renameChatRef.current = app.chatHistory.handleRenameChat;
    togglePinRef.current = app.chatHistory.handleTogglePinChat;
  }, [
    app.handleSelectChat,
    app.chatHistory.handleRenameChat,
    app.chatHistory.handleTogglePinChat,
  ]);

  const { pinnedChats, threadChats, allChats } = useMemo(() => {
    const sortedChats = chats
      .filter((c: any) => !c.id.startsWith(SYSTEM_PREFIX))
      .slice()
      .sort(
        (a, b) =>
          new Date(b.updated_at || b.created_at).getTime() -
          new Date(a.updated_at || a.created_at).getTime(),
      );

    return {
      pinnedChats: sortedChats.filter((chat) => chat.is_pinned),
      threadChats: sortedChats.filter((chat) => !chat.is_pinned),
      allChats: sortedChats,
    };
  }, [chats]);

  const showWelcome =
    !app.system.activeProfile && app.system.hasAgreed === false;
  const isChatBusy =
    app.chat.isAnalyzing ||
    app.chat.isGenerating ||
    app.chat.isAiTyping ||
    app.isOcrScanning;
  const busyChatId = isChatBusy ? activeSessionId : null;

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

  const handleTogglePin = useCallback(
    async (chatId: string) => {
      if (pinToggleLockRef.current.has(chatId)) return;

      pinToggleLockRef.current.add(chatId);
      try {
        await togglePinRef.current(chatId);
      } finally {
        setTimeout(() => {
          pinToggleLockRef.current.delete(chatId);
        }, 220);
      }
    },
    [],
  );

  const handleSelectChat = useCallback(
    (chatId: string) => {
      selectChatRef.current(chatId);
    },
    [],
  );

  const handleRenameChat = useCallback(
    (chatId: string, newTitle: string) => {
      renameChatRef.current(chatId, newTitle);
    },
    [],
  );

  const handleEnableSelectionMode = useCallback(() => {
    setIsSelectionMode(true);
  }, []);

  const toggleSelectionMode = useCallback(() => {
    setIsSelectionMode((prev) => !prev);
    setSelectedIds([]);
  }, []);

  const toggleChatSelection = useCallback((id: string) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((i) => i !== id) : [...prev, id],
    );
  }, []);

  const handleQueueDeleteChat = useCallback((chatId: string) => {
    setDeleteId(chatId);
  }, []);

  const selectAll = () => {
    setSelectedIds(
      selectedIds.length === allChats.length ? [] : allChats.map((c) => c.id),
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
              checked={
                selectedIds.length === allChats.length && allChats.length > 0
              }
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
            </div>
          </div>
        </div>
      )}

      <div className={styles.scrollArea}>
        <div className={styles.stickyThreadHeader}>
          {(pinnedChats.length > 0 || (showWelcome && !isSelectionMode)) && (
            <div className={styles.groupInner}>
              {pinnedChats.map((chat) => (
                <ChatItem
                  key={chat.id}
                  chat={chat}
                  isActive={chat.id === activeSessionId}
                  isBusy={busyChatId === chat.id}
                  isSelectionMode={isSelectionMode}
                  isSelected={selectedIdSet.has(chat.id)}
                  menuState={
                    activeContextMenu?.id === chat.id ? activeContextMenu : null
                  }
                  onSelectChat={handleSelectChat}
                  onToggleSelectionChat={toggleChatSelection}
                  onDeleteChat={handleQueueDeleteChat}
                  onRenameChat={handleRenameChat}
                  onTogglePinChat={handleTogglePin}
                  onOpenContextMenu={handleOpenContextMenu}
                  onCloseContextMenu={handleCloseContextMenu}
                  onEnableSelectionMode={handleEnableSelectionMode}
                />
              ))}

              {showWelcome && !isSelectionMode && (
                <div
                  className={`${styles.chatRow} ${activeSessionId === "__system_welcome" ? styles.active : ""}`}
                  onClick={() => app.handleSelectChat("__system_welcome")}
                >
                  <div className={styles.chatLeading}>
                    {busyChatId === "__system_welcome" ? (
                      <span className={styles.rowSpinner} aria-hidden="true">
                        <span className={styles.rowSpinnerInner}>
                          <LoadingSpinner />
                        </span>
                      </span>
                    ) : (
                      <span className={styles.welcomeDot} aria-hidden="true" />
                    )}
                  </div>
                  <span className={styles.chatTitle}>
                    Welcome to {app.system.appName}!
                  </span>
                </div>
              )}
            </div>
          )}
          <div className={styles.threadsDivider}>
            <span>Threads</span>
          </div>
        </div>

        <div className={styles.groupInner}>
          {threadChats.map((chat) => (
            <ChatItem
              key={chat.id}
              chat={chat}
              isActive={chat.id === activeSessionId}
              isBusy={busyChatId === chat.id}
              isSelectionMode={isSelectionMode}
              isSelected={selectedIdSet.has(chat.id)}
              menuState={
                activeContextMenu?.id === chat.id ? activeContextMenu : null
              }
              onSelectChat={handleSelectChat}
              onToggleSelectionChat={toggleChatSelection}
              onDeleteChat={handleQueueDeleteChat}
              onRenameChat={handleRenameChat}
              onTogglePinChat={handleTogglePin}
              onOpenContextMenu={handleOpenContextMenu}
              onCloseContextMenu={handleCloseContextMenu}
              onEnableSelectionMode={handleEnableSelectionMode}
            />
          ))}

          {threadChats.length === 0 && (
            <div
              style={{
                padding: "20px 20px",
                textAlign: "center",
                color: "var(--c-raw-073)",
                fontSize: "0.9rem",
              }}
            >
              No threads yet.
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
