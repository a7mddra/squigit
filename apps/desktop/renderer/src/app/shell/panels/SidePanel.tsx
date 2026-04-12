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
  Milestone,
  ArrowUpRight,
} from "lucide-react";

import {
  SidePanelNewThreadIcon,
  SidePanelSquigitsIcon,
} from "@/components/icons";
import { Dialog, LoadingSpinner } from "@/components/ui";
import { useKeyDown, usePlatform } from "@/hooks";
import { ChatMetadata, getDeleteMultipleChatsDialog } from "@/core";
import { PanelContextMenu } from "../menus/PanelContextMenu";
import { useAppContext } from "../../providers/AppProvider";
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

const ChatItem: React.FC<ChatItemProps> = React.memo(
  ({
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
            <div className={styles.chatLeading} style={{ paddingLeft: "2px" }}>
              {isBusy ? (
                <span className={styles.rowSpinner} aria-hidden="true">
                  <span className={styles.rowSpinnerInner}>
                    <LoadingSpinner />
                  </span>
                </span>
              ) : (
                <>
                  <FolderOpen
                    size={20}
                    strokeWidth={1.5}
                    className={styles.chatBubbleIcon}
                  />
                  <button
                    type="button"
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
                <span className={styles.chatDate} title={lastActivityTitle}>
                  {lastActivityLabel}
                </span>
              )}

              <button
                type="button"
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
  },
);

export const SidePanel: React.FC = () => {
  const app = useAppContext();
  const platform = usePlatform();
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

  const searchShortcutLabel = useMemo(() => {
    if (platform.isMac) {
      return `${platform.modSymbol}K`;
    }
    return `${platform.modSymbol} + K`;
  }, [platform.isMac, platform.modSymbol]);

  const newThreadShortcutLabel = useMemo(() => {
    if (platform.isMac) {
      return `${platform.modSymbol}${platform.shiftSymbol}O`;
    }
    return `${platform.modSymbol} + Shift + O`;
  }, [platform.isMac, platform.modSymbol, platform.shiftSymbol]);

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

  const handleTogglePin = useCallback(async (chatId: string) => {
    if (pinToggleLockRef.current.has(chatId)) return;

    pinToggleLockRef.current.add(chatId);
    try {
      await togglePinRef.current(chatId);
    } finally {
      setTimeout(() => {
        pinToggleLockRef.current.delete(chatId);
      }, 220);
    }
  }, []);

  const handleSelectChat = useCallback(
    (chatId: string) => {
      if (chatId === activeSessionId) return;
      selectChatRef.current(chatId);
    },
    [activeSessionId],
  );

  const handleRenameChat = useCallback((chatId: string, newTitle: string) => {
    renameChatRef.current(chatId, newTitle);
  }, []);

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
              type="button"
              className={`${styles.iconBtn} ${styles.danger}`}
              onClick={() => selectedIds.length > 0 && setShowBulkDelete(true)}
              disabled={selectedIds.length === 0}
            >
              <Trash2 size={16} />
            </button>
            <button
              type="button"
              className={styles.iconBtn}
              onClick={toggleSelectionMode}
            >
              <X size={18} />
            </button>
          </div>
        </div>
      ) : (
        <div className={styles.headerArea}>
          <div className={styles.groupContent}>
            <div className={styles.groupInner}>
              <div className={styles.chatRow} onClick={app.handleNewSession}>
                <div
                  className={styles.chatIconMain}
                  style={{ paddingLeft: "2px" }}
                >
                  <SidePanelNewThreadIcon size={18} />
                </div>
                <span className={styles.chatTitle}>New thread</span>
                <div className={styles.rowShortcut} aria-hidden="true">
                  <span className={styles.rowShortcutText}>
                    {newThreadShortcutLabel}
                  </span>
                </div>
              </div>

              <div
                className={`${styles.chatRow} ${app.searchOverlay.isOpen ? styles.active : ""}`}
                onClick={app.openSearchOverlay}
              >
                <div
                  className={styles.chatIconMain}
                  style={{ paddingLeft: "1px" }}
                >
                  <Search size={19} />
                </div>
                <span className={styles.chatTitle}>Search chats</span>
                <div className={styles.rowShortcut} aria-hidden="true">
                  <span className={styles.rowShortcutText}>
                    {searchShortcutLabel}
                  </span>
                </div>
              </div>

              <div
                className={`${styles.chatRow} ${activeSessionId === "__system_gallery" ? styles.active : ""}`}
                onClick={() => app.handleSelectChat("__system_gallery")}
              >
                <div className={styles.chatIconMain}>
                  <SidePanelSquigitsIcon size={22} />
                </div>
                <span className={styles.chatTitle}>Your squigits</span>
                <div className={styles.rowShortcut} aria-hidden="true">
                  <ArrowUpRight size={14} className={styles.rowShortcutIcon} />
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      <div className={styles.scrollArea}>
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
                <div className={styles.chatIconMain}>
                  <Milestone size={20} />
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
            <div className={styles.emptyState}>No threads yet.</div>
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
