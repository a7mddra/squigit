/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import type { ChatMetadata } from "@/lib";

const SYSTEM_PREFIX = "__system_";

export type ChatGroup = {
  key: number;
  label: string;
  chats: ChatMetadata[];
};

const isOnboardingId = (id: string) => id.startsWith(SYSTEM_PREFIX);

const escapeRegex = (value: string) =>
  value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const isRegexQuery = (query: string) => {
  const trimmed = query.trim();
  return (
    trimmed.startsWith("re:") ||
    (trimmed.startsWith("/") && trimmed.endsWith("/") && trimmed.length > 2)
  );
};

const startOfDay = (date: Date) =>
  new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();

const formatGroupHeader = (isoDate: string): string => {
  const parsed = new Date(isoDate);
  if (Number.isNaN(parsed.getTime())) return "Unknown";

  const now = new Date();
  const diffDays = Math.round(
    (startOfDay(now) - startOfDay(parsed)) / 86_400_000,
  );

  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";

  const options: Intl.DateTimeFormatOptions =
    parsed.getFullYear() === now.getFullYear()
      ? { month: "short", day: "numeric" }
      : { month: "short", day: "numeric", year: "numeric" };
  return parsed.toLocaleDateString(undefined, options);
};

export const formatSearchRowDate = (isoDate: string): string => {
  const parsed = new Date(isoDate);
  if (Number.isNaN(parsed.getTime())) return "";

  const now = new Date();
  const diffDays = Math.round(
    (startOfDay(now) - startOfDay(parsed)) / 86_400_000,
  );

  if (diffDays === 0) return "today";
  if (diffDays === 1) return "yesterday";

  const options: Intl.DateTimeFormatOptions =
    parsed.getFullYear() === now.getFullYear()
      ? { month: "short", day: "numeric" }
      : { month: "short", day: "numeric", year: "numeric" };
  return parsed.toLocaleDateString(undefined, options).toLowerCase();
};

export const buildChatGroups = (chats: ChatMetadata[]): ChatGroup[] => {
  const sorted = chats
    .filter((chat) => !isOnboardingId(chat.id))
    .slice()
    .sort(
      (a, b) =>
        new Date(b.updated_at || b.created_at).getTime() -
        new Date(a.updated_at || a.created_at).getTime(),
    );

  const groups: ChatGroup[] = [];
  for (const chat of sorted) {
    const anchor = chat.updated_at || chat.created_at;
    const parsed = new Date(anchor);
    const key = Number.isNaN(parsed.getTime()) ? -1 : startOfDay(parsed);
    const label = formatGroupHeader(anchor);
    const last = groups[groups.length - 1];

    if (!last || last.key !== key) {
      groups.push({ key, label, chats: [chat] });
    } else {
      last.chats.push(chat);
    }
  }

  return groups;
};

export const highlightTokensFromQuery = (query: string): string[] => {
  if (!query.trim() || isRegexQuery(query)) return [];

  const tokens = query
    .split(/\s+/)
    .map((segment) => segment.replace(/^[-+]/, ""))
    .map((segment) => segment.toLowerCase().replace(/[^a-z0-9]/g, ""))
    .filter((segment) => segment.length >= 2);

  return [...new Set(tokens)].sort((a, b) => b.length - a.length).slice(0, 8);
};

export const renderSnippetWithHighlights = (
  snippet: string,
  tokens: string[],
  highlightClassName: string,
): React.ReactNode => {
  if (!snippet || tokens.length === 0) {
    return snippet;
  }

  const pattern = tokens.map(escapeRegex).join("|");
  if (!pattern) return snippet;

  const matcher = new RegExp(`(${pattern})`, "gi");
  const parts = snippet.split(matcher);

  return parts.map((part, index) =>
    index % 2 === 1 ? (
      <strong key={`match-${index}`} className={highlightClassName}>
        {part}
      </strong>
    ) : (
      <React.Fragment key={`text-${index}`}>{part}</React.Fragment>
    ),
  );
};
