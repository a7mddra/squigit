/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useEffect, useCallback, useRef } from "react";
import {
  type ChatSearchResult,
  ChatMetadata,
  listChats,
  searchChats as searchChatsApi,
  deleteChat,
  updateChatMetadata as updateChatMeta,
} from "@/core";
const SYSTEM_PREFIX = "__system_";
const isOnboardingId = (id: string) => id.startsWith(SYSTEM_PREFIX);
const TOUCH_THROTTLE_MS = 1200;

export const useChatHistory = (activeProfileId: string | null = null) => {
  const [chats, setChats] = useState<ChatMetadata[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const chatsRef = useRef<ChatMetadata[]>([]);
  const lastTouchAtRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    chatsRef.current = chats;
  }, [chats]);

  const refreshChats = useCallback(async () => {
    if (!activeProfileId) {
      setChats([]);
      return;
    }

    setIsLoading(true);
    try {
      const chatList = await listChats();

      setChats(chatList.filter((c) => !isOnboardingId(c.id)));
    } catch (e) {
      console.error("Failed to load chats:", e);
      setChats([]);
    } finally {
      setIsLoading(false);
    }
  }, [activeProfileId]);

  useEffect(() => {
    refreshChats();
  }, [refreshChats]);

  useEffect(() => {
    setActiveSessionId(null);
  }, [activeProfileId]);

  const handleDeleteChat = async (id: string) => {
    if (isOnboardingId(id)) return;
    try {
      await deleteChat(id);
      setChats((prev) => prev.filter((c) => c.id !== id));
      if (activeSessionId === id) {
        setActiveSessionId(null);
      }
    } catch (e) {
      console.error("Failed to delete chat:", e);
    }
  };

  const handleDeleteChats = async (ids: string[]) => {
    const realIds = ids.filter((id) => !isOnboardingId(id));
    if (realIds.length === 0) return;
    try {
      await Promise.all(realIds.map((id) => deleteChat(id)));
      setChats((prev) => prev.filter((c) => !realIds.includes(c.id)));
      if (activeSessionId && realIds.includes(activeSessionId)) {
        setActiveSessionId(null);
      }
    } catch (e) {
      console.error("Failed to delete chats:", e);
    }
  };

  const handleRenameChat = async (id: string, newTitle: string) => {
    if (isOnboardingId(id)) return;
    const chat = chats.find((c) => c.id === id);
    if (!chat) return;

    const updated = {
      ...chat,
      title: newTitle,
    };
    setChats((prev) => prev.map((c) => (c.id === id ? updated : c)));
    try {
      await updateChatMeta(updated);
    } catch (e) {
      console.error("Failed to rename chat:", e);
      setChats((prev) => prev.map((c) => (c.id === id ? chat : c)));
    }
  };

  const handleTogglePinChat = async (id: string) => {
    if (isOnboardingId(id)) return;
    const chat = chats.find((c) => c.id === id);
    if (!chat) return;

    const newPinnedState = !chat.is_pinned;
    const updated = {
      ...chat,
      is_pinned: newPinnedState,
      pinned_at: newPinnedState ? new Date().toISOString() : null,
    };

    setChats((prev) => prev.map((c) => (c.id === id ? updated : c)));
    try {
      await updateChatMeta(updated);
    } catch (e) {
      console.error("Failed to toggle pin:", e);
      setChats((prev) => prev.map((c) => (c.id === id ? chat : c)));
    }
  };

  const handleToggleStarChat = async (
    id: string,
    overrides?: Partial<ChatMetadata>,
  ) => {
    if (isOnboardingId(id)) return;
    const chat = chats.find((c) => c.id === id);
    if (!chat) return;

    const newStarredState = !chat.is_starred;
    const updated = {
      ...chat,
      is_starred: newStarredState,

      is_pinned: false,
      pinned_at: null,
      ...overrides,
    };

    setChats((prev) => prev.map((c) => (c.id === id ? updated : c)));
    try {
      await updateChatMeta(updated);
    } catch (e) {
      console.error("Failed to toggle star:", e);
      setChats((prev) => prev.map((c) => (c.id === id ? chat : c)));
    }
  };

  const touchChat = useCallback(async (id: string) => {
    if (isOnboardingId(id)) return;

    const now = Date.now();
    const lastTouchedAt = lastTouchAtRef.current.get(id) || 0;
    if (now - lastTouchedAt < TOUCH_THROTTLE_MS) {
      return;
    }
    lastTouchAtRef.current.set(id, now);

    const chat = chatsRef.current.find((c) => c.id === id);
    if (!chat) return;

    const updated = {
      ...chat,
      updated_at: new Date(now).toISOString(),
    };

    setChats((prev) => prev.map((c) => (c.id === id ? updated : c)));

    try {
      await updateChatMeta(updated);
    } catch (e) {
      console.error("Failed to touch chat metadata:", e);
    }
  }, []);

  const searchChats = useCallback(
    async (query: string, limit = 60): Promise<ChatSearchResult[]> => {
      if (!activeProfileId) return [];
      try {
        return await searchChatsApi(query, limit);
      } catch (e) {
        console.error("Failed to search chats:", e);
        return [];
      }
    },
    [activeProfileId],
  );

  return {
    chats,
    activeSessionId,
    setActiveSessionId,
    isLoading,
    refreshChats,
    handleDeleteChat,
    handleDeleteChats,
    handleRenameChat,
    handleTogglePinChat,
    handleToggleStarChat,
    touchChat,
    searchChats,
  };
};
