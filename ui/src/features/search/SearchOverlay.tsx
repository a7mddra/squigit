/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { WidgetOverlay } from "@/components";
import type { ChatSearchResult } from "@/lib";
import { useAppContext } from "@/providers/AppProvider";
import { ChatsList } from "./components/ChatsList/ChatsList";
import { SearchBar } from "./components/SearchBar/SearchBar";
import { buildChatGroups, highlightTokensFromQuery } from "./search.utils";
import styles from "./SearchOverlay.module.css";

const SEARCH_LIMIT = 80;
const SEARCH_DEBOUNCE_MS = 120;

interface SearchOverlayProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SearchOverlay: React.FC<SearchOverlayProps> = ({
  isOpen,
  onClose,
}) => {
  const app = useAppContext();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ChatSearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const requestIdRef = useRef(0);

  const normalizedQuery = query.trim();
  const hasQuery = normalizedQuery.length > 0;

  const highlightTokens = useMemo(
    () => highlightTokensFromQuery(normalizedQuery),
    [normalizedQuery],
  );

  const chatGroups = useMemo(
    () => buildChatGroups(app.chatHistory.chats),
    [app.chatHistory.chats],
  );

  const firstChatId = chatGroups[0]?.chats[0]?.id ?? null;

  useEffect(() => {
    if (!isOpen) {
      requestIdRef.current += 1;
      setQuery("");
      setResults([]);
      setIsLoading(false);
      return;
    }

    const raf = window.requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.select();
    });

    return () => window.cancelAnimationFrame(raf);
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;

    if (!hasQuery) {
      requestIdRef.current += 1;
      setResults([]);
      setIsLoading(false);
      return;
    }

    const currentRequestId = ++requestIdRef.current;
    const timer = window.setTimeout(async () => {
      setIsLoading(true);
      const hits = await app.chatHistory.searchChats(normalizedQuery, SEARCH_LIMIT);
      if (currentRequestId !== requestIdRef.current) return;
      setResults(hits);
      setIsLoading(false);
    }, SEARCH_DEBOUNCE_MS);

    return () => {
      window.clearTimeout(timer);
    };
  }, [app.chatHistory.searchChats, hasQuery, isOpen, normalizedQuery]);

  const handleSelectResult = useCallback(
    (result: ChatSearchResult) => {
      app.revealSearchMatch({
        chatId: result.chat_id,
        messageIndex: result.message_index,
      });
    },
    [app],
  );

  const handleSelectChat = useCallback(
    (chatId: string) => {
      onClose();
      app.handleSelectChat(chatId);
    },
    [app, onClose],
  );

  const handleInputKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }

      if (event.key !== "Enter") return;
      event.preventDefault();

      if (hasQuery) {
        if (results.length > 0) {
          handleSelectResult(results[0]);
        }
        return;
      }

      if (firstChatId) {
        handleSelectChat(firstChatId);
      }
    },
    [
      firstChatId,
      handleSelectChat,
      handleSelectResult,
      hasQuery,
      onClose,
      results,
    ],
  );

  return (
    <WidgetOverlay
      isOpen={isOpen}
      onClose={onClose}
      contentClassName={styles.content}
      sectionContentClassName={styles.section}
    >
      <div className={styles.root}>
        <SearchBar
          query={query}
          onQueryChange={setQuery}
          onInputKeyDown={handleInputKeyDown}
          inputRef={inputRef}
        />

        <ChatsList
          hasQuery={hasQuery}
          isLoading={isLoading}
          results={results}
          groups={chatGroups}
          highlightTokens={highlightTokens}
          onSelectResult={handleSelectResult}
          onSelectChat={handleSelectChat}
        />
      </div>
    </WidgetOverlay>
  );
};
