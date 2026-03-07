/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React from "react";
import { X } from "lucide-react";
import styles from "./SearchBar.module.css";

interface SearchBarProps {
  query: string;
  onQueryChange: (query: string) => void;
  onInputKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
}

export const SearchBar: React.FC<SearchBarProps> = ({
  query,
  onQueryChange,
  onInputKeyDown,
  inputRef,
}) => {
  return (
    <div className={styles.searchBar}>
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={onInputKeyDown}
        className={styles.input}
        placeholder="Search chats..."
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
      />
      {query ? (
        <button
          type="button"
          className={styles.clearButton}
          onClick={() => onQueryChange("")}
          aria-label="Clear search query"
        >
          <X size={15} />
        </button>
      ) : null}
    </div>
  );
};
