/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { LoadingSpinner } from "@/components";
import type { ChatSearchResult } from "@/lib";
import {
  type ChatGroup,
  formatSearchRowDate,
  renderSnippetWithHighlights,
} from "../../search.utils";
import { ChatRow } from "../ChatRow/ChatRow";
import styles from "./ChatsList.module.css";

interface ChatsListProps {
  hasQuery: boolean;
  isLoading: boolean;
  results: ChatSearchResult[];
  groups: ChatGroup[];
  highlightTokens: string[];
  onSelectResult: (result: ChatSearchResult) => void;
  onSelectChat: (chatId: string) => void;
}

export const ChatsList: React.FC<ChatsListProps> = ({
  hasQuery,
  isLoading,
  results,
  groups,
  highlightTokens,
  onSelectResult,
  onSelectChat,
}) => {
  return (
    <div className={styles.list}>
      {hasQuery ? (
        <>
          {isLoading ? (
            <div className={`${styles.status} ${styles.loadingStatus}`}>
              <LoadingSpinner />
            </div>
          ) : null}

          {!isLoading && results.length === 0 ? (
            <div className={styles.status}>No matching messages found</div>
          ) : null}

          {!isLoading
            ? results.map((result) => (
                <ChatRow
                  key={`${result.chat_id}:${result.message_index}`}
                  title={result.chat_title || "Untitled Chat"}
                  snippet={renderSnippetWithHighlights(
                    result.snippet,
                    highlightTokens,
                    styles.highlight,
                  )}
                  dateLabel={formatSearchRowDate(result.chat_updated_at)}
                  onClick={() => onSelectResult(result)}
                />
              ))
            : null}
        </>
      ) : (
        <>
          {groups.length === 0 ? (
            <div className={styles.status}>No chats to show yet</div>
          ) : null}

          {groups.map((group) => (
            <div key={`${group.key}:${group.label}`} className={styles.group}>
              <div className={styles.groupHeader}>{group.label}</div>
              {group.chats.map((chat) => (
                <ChatRow
                  key={chat.id}
                  compact
                  title={chat.title || "Untitled Chat"}
                  onClick={() => onSelectChat(chat.id)}
                />
              ))}
            </div>
          ))}
        </>
      )}
    </div>
  );
};
