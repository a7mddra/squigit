// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_chat_storage::{ChatMessage, ChatMetadata, ChatStorage};
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct ChatSearchResult {
    pub chat_id: String,
    pub chat_title: String,
    pub chat_created_at: String,
    pub chat_updated_at: String,
    pub message_index: usize,
    pub message_role: String,
    pub message_timestamp: String,
    pub snippet: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
struct ParsedQuery {
    raw: String,
    normalized: String,
    include_tokens: Vec<String>,
    exclude_tokens: Vec<String>,
    regex: Option<Regex>,
    full_query_regex: Option<Regex>,
    token_regexes: Vec<Regex>,
    is_regex_mode: bool,
}

impl ParsedQuery {
    fn from_raw(query: &str) -> Self {
        let raw = query.trim().to_string();
        let normalized = normalize_text(&raw);
        let (regex, is_regex_mode) = parse_regex_mode(&raw);

        let mut include_tokens = Vec::new();
        let mut exclude_tokens = Vec::new();

        for segment in raw.split_whitespace() {
            let (target, token) = if let Some(rest) = segment.strip_prefix('-') {
                (&mut exclude_tokens, rest)
            } else if let Some(rest) = segment.strip_prefix('+') {
                (&mut include_tokens, rest)
            } else {
                (&mut include_tokens, segment)
            };

            let normalized_segment = normalize_text(token);
            if normalized_segment.is_empty() {
                continue;
            }

            target.extend(
                normalized_segment
                    .split_whitespace()
                    .map(std::string::ToString::to_string),
            );
        }

        if include_tokens.is_empty() && !normalized.is_empty() {
            include_tokens.extend(
                normalized
                    .split_whitespace()
                    .map(std::string::ToString::to_string),
            );
        }

        include_tokens = dedupe(include_tokens);
        exclude_tokens = dedupe(exclude_tokens);

        let full_query_regex = if raw.is_empty() || is_regex_mode {
            None
        } else {
            RegexBuilder::new(&regex::escape(raw.trim_matches('"')))
                .case_insensitive(true)
                .build()
                .ok()
        };

        let token_regexes = include_tokens
            .iter()
            .filter_map(|token| {
                RegexBuilder::new(&regex::escape(token))
                    .case_insensitive(true)
                    .build()
                    .ok()
            })
            .collect();

        Self {
            raw,
            normalized,
            include_tokens,
            exclude_tokens,
            regex,
            full_query_regex,
            token_regexes,
            is_regex_mode,
        }
    }

    fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

#[derive(Debug, Clone)]
struct ScoredMessage {
    message_index: usize,
    message_role: String,
    message_timestamp: String,
    snippet: String,
    score: f64,
}

pub fn search_local_chats(
    storage: &ChatStorage,
    query: &str,
    limit: usize,
) -> Result<Vec<ChatSearchResult>, String> {
    let max_results = limit.clamp(1, 200);
    let parsed = ParsedQuery::from_raw(query);
    let now_unix = now_unix_seconds();

    let mut rows = Vec::new();
    let mut chats = storage.list_chats().map_err(|e| e.to_string())?;
    chats.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    for metadata in chats
        .into_iter()
        .filter(|chat| !chat.id.starts_with("__system_"))
    {
        let chat = match storage.load_chat(&metadata.id) {
            Ok(chat) => chat,
            Err(_) => continue,
        };

        if chat.messages.is_empty() {
            continue;
        }

        let title_normalized = normalize_text(&metadata.title);
        let mut best: Option<ScoredMessage> = None;
        let total_messages = chat.messages.len();

        for (message_index, message) in chat.messages.iter().enumerate() {
            let candidate = score_message(
                &parsed,
                &metadata,
                &title_normalized,
                message,
                message_index,
                total_messages,
                now_unix,
            );

            if let Some(candidate) = candidate {
                let replace = best
                    .as_ref()
                    .map(|existing| candidate.score > existing.score)
                    .unwrap_or(true);
                if replace {
                    best = Some(candidate);
                }
            }
        }

        if let Some(best) = best {
            rows.push(ChatSearchResult {
                chat_id: metadata.id.clone(),
                chat_title: metadata.title.clone(),
                chat_created_at: metadata.created_at.to_rfc3339(),
                chat_updated_at: metadata.updated_at.to_rfc3339(),
                message_index: best.message_index,
                message_role: best.message_role,
                message_timestamp: best.message_timestamp,
                snippet: best.snippet,
                score: best.score,
            });
        }
    }

    rows.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| b.chat_updated_at.cmp(&a.chat_updated_at))
            .then_with(|| a.chat_id.cmp(&b.chat_id))
    });
    rows.truncate(max_results);

    Ok(rows)
}

fn score_message(
    parsed: &ParsedQuery,
    metadata: &ChatMetadata,
    title_normalized: &str,
    message: &ChatMessage,
    message_index: usize,
    total_messages: usize,
    now_unix: i64,
) -> Option<ScoredMessage> {
    let body_normalized = normalize_text(&message.content);
    if body_normalized.is_empty() {
        return None;
    }

    let body_tokens: Vec<&str> = body_normalized.split_whitespace().collect();
    if body_tokens.is_empty() {
        return None;
    }
    let body_token_set: HashSet<&str> = body_tokens.iter().copied().collect();

    if parsed
        .exclude_tokens
        .iter()
        .any(|token| body_token_set.contains(token.as_str()))
    {
        return None;
    }

    let recency = recency_score(now_unix, metadata.updated_at.timestamp());
    let position_bias = if total_messages <= 1 {
        1.0
    } else {
        message_index as f64 / (total_messages as f64 - 1.0)
    };

    if parsed.is_empty() {
        return Some(ScoredMessage {
            message_index,
            message_role: message.role.clone(),
            message_timestamp: message.timestamp.to_rfc3339(),
            snippet: build_snippet(&message.content, None),
            score: round_score(recency * 70.0 + position_bias * 30.0),
        });
    }

    let title_tokens: Vec<&str> = title_normalized.split_whitespace().collect();
    let title_set: HashSet<&str> = title_tokens.iter().copied().collect();

    let regex_bounds = parsed
        .regex
        .as_ref()
        .and_then(|compiled| compiled.find(&message.content))
        .map(|m| (m.start(), m.end()));

    if parsed.is_regex_mode && regex_bounds.is_none() {
        return None;
    }

    let phrase_match =
        !parsed.normalized.is_empty() && body_normalized.contains(&parsed.normalized);

    let matched_token_count = parsed
        .include_tokens
        .iter()
        .filter(|token| body_token_set.contains(token.as_str()))
        .count();
    let token_coverage = if parsed.include_tokens.is_empty() {
        0.0
    } else {
        matched_token_count as f64 / parsed.include_tokens.len() as f64
    };

    let title_token_hits = parsed
        .include_tokens
        .iter()
        .filter(|token| title_set.contains(token.as_str()))
        .count();
    let title_coverage = if parsed.include_tokens.is_empty() {
        0.0
    } else {
        title_token_hits as f64 / parsed.include_tokens.len() as f64
    };
    let title_phrase_match =
        !parsed.normalized.is_empty() && title_normalized.contains(&parsed.normalized);

    let shortest_window = shortest_covering_window(&body_tokens, &parsed.include_tokens);
    let window_score = shortest_window
        .map(|window| (1.0 / (window as f64).sqrt()).clamp(0.0, 1.0))
        .unwrap_or(0.0);

    let fuzzy_similarity = fuzzy_token_similarity(&parsed.include_tokens, &body_tokens);
    let lnp_similarity = longest_normalized_prefix_similarity(
        parsed
            .include_tokens
            .first()
            .map(std::string::String::as_str)
            .unwrap_or(parsed.normalized.as_str()),
        &body_tokens,
    );

    let has_signal = phrase_match
        || token_coverage > 0.0
        || regex_bounds.is_some()
        || title_coverage > 0.0
        || fuzzy_similarity >= 0.80
        || lnp_similarity >= 0.80;
    if !has_signal {
        return None;
    }

    let mut score = 0.0;
    if phrase_match {
        score += 34.0;
    }
    if regex_bounds.is_some() {
        score += 38.0;
    }
    score += token_coverage * 30.0;
    score += window_score * 16.0;
    score += fuzzy_similarity * 16.0;
    score += lnp_similarity * 7.0;
    score += title_coverage * 10.0;
    if title_phrase_match {
        score += 8.0;
    }
    score += recency * 6.0;
    score += position_bias * 4.0;

    if score < 8.0 {
        return None;
    }

    let bounds = regex_bounds.or_else(|| find_literal_match_bounds(&message.content, parsed));

    Some(ScoredMessage {
        message_index,
        message_role: message.role.clone(),
        message_timestamp: message.timestamp.to_rfc3339(),
        snippet: build_snippet(&message.content, bounds),
        score: round_score(score),
    })
}

fn parse_regex_mode(raw: &str) -> (Option<Regex>, bool) {
    let trimmed = raw.trim();
    let pattern = if let Some(rest) = trimmed.strip_prefix("re:") {
        Some(rest.trim())
    } else if trimmed.starts_with('/') && trimmed.ends_with('/') && trimmed.len() > 2 {
        Some(&trimmed[1..trimmed.len() - 1])
    } else {
        None
    };

    match pattern {
        Some(pattern) if !pattern.is_empty() => {
            let compiled = RegexBuilder::new(pattern)
                .case_insensitive(true)
                .multi_line(true)
                .build()
                .ok();
            if compiled.is_some() {
                (compiled, true)
            } else {
                (None, false)
            }
        }
        _ => (None, false),
    }
}

fn normalize_text(input: &str) -> String {
    let lowered = input.to_lowercase();
    let normalized: String = lowered
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect();

    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            output.push(value);
        }
    }

    output
}

fn shortest_covering_window(message_tokens: &[&str], query_tokens: &[String]) -> Option<usize> {
    if message_tokens.is_empty() || query_tokens.is_empty() {
        return None;
    }

    let unique_query_tokens = dedupe(query_tokens.to_vec());
    if unique_query_tokens.is_empty() {
        return None;
    }

    // Classic shortest-path-in-array trick (two pointers): smallest window
    // that contains every query token at least once.
    let mut need: HashMap<&str, usize> = HashMap::new();
    for token in &unique_query_tokens {
        *need.entry(token.as_str()).or_insert(0) += 1;
    }

    let mut have: HashMap<&str, usize> = HashMap::new();
    let mut satisfied = 0usize;
    let target = need.len();
    let mut left = 0usize;
    let mut best = usize::MAX;

    for right in 0..message_tokens.len() {
        let token = message_tokens[right];
        if let Some(required) = need.get(token) {
            let count = have.entry(token).or_insert(0);
            *count += 1;
            if *count == *required {
                satisfied += 1;
            }
        }

        while satisfied == target && left <= right {
            best = best.min(right - left + 1);
            let left_token = message_tokens[left];
            if let Some(required) = need.get(left_token) {
                if let Some(count) = have.get_mut(left_token) {
                    *count -= 1;
                    if *count < *required {
                        satisfied -= 1;
                    }
                }
            }
            left += 1;
        }
    }

    if best == usize::MAX {
        None
    } else {
        Some(best)
    }
}

fn fuzzy_token_similarity(query_tokens: &[String], message_tokens: &[&str]) -> f64 {
    if query_tokens.is_empty() || message_tokens.is_empty() {
        return 0.0;
    }

    let sum: f64 = query_tokens
        .iter()
        .map(|query_token| {
            message_tokens
                .iter()
                .map(|message_token| normalized_edit_similarity(query_token, message_token))
                .fold(0.0, f64::max)
        })
        .sum();

    (sum / query_tokens.len() as f64).clamp(0.0, 1.0)
}

fn normalized_edit_similarity(left: &str, right: &str) -> f64 {
    let left_chars: Vec<char> = left.chars().take(48).collect();
    let right_chars: Vec<char> = right.chars().take(48).collect();

    if left_chars.is_empty() || right_chars.is_empty() {
        return 0.0;
    }

    let width = right_chars.len();
    let mut prev: Vec<usize> = (0..=width).collect();
    let mut curr = vec![0usize; width + 1];

    for (i, left_char) in left_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            let delete_cost = prev[j + 1] + 1;
            let insert_cost = curr[j] + 1;
            let substitute_cost = prev[j] + substitution_cost;
            curr[j + 1] = delete_cost.min(insert_cost).min(substitute_cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    let distance = prev[width] as f64;
    let baseline = left_chars.len().max(right_chars.len()) as f64;
    (1.0 - distance / baseline).clamp(0.0, 1.0)
}

fn longest_normalized_prefix_similarity(query: &str, message_tokens: &[&str]) -> f64 {
    if query.is_empty() || message_tokens.is_empty() {
        return 0.0;
    }

    let query_head = query.split_whitespace().next().unwrap_or(query);
    if query_head.is_empty() {
        return 0.0;
    }

    let query_len = query_head.chars().count().max(1);
    let longest = message_tokens
        .iter()
        .map(|token| {
            query_head
                .chars()
                .zip(token.chars())
                .take_while(|(a, b)| a == b)
                .count()
        })
        .max()
        .unwrap_or(0);

    (longest as f64 / query_len as f64).clamp(0.0, 1.0)
}

fn find_literal_match_bounds(content: &str, parsed: &ParsedQuery) -> Option<(usize, usize)> {
    if let Some(full_query_regex) = &parsed.full_query_regex {
        if let Some(m) = full_query_regex.find(content) {
            return Some((m.start(), m.end()));
        }
    }

    parsed
        .token_regexes
        .iter()
        .filter_map(|matcher| matcher.find(content))
        .min_by(|a, b| {
            a.start()
                .cmp(&b.start())
                .then_with(|| b.end().cmp(&a.end()))
        })
        .map(|m| (m.start(), m.end()))
}

fn floor_boundary(input: &str, index: usize) -> usize {
    let mut idx = index.min(input.len());
    while idx > 0 && !input.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_boundary(input: &str, index: usize) -> usize {
    let mut idx = index.min(input.len());
    while idx < input.len() && !input.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn build_snippet(content: &str, bounds: Option<(usize, usize)>) -> String {
    let line = content.replace('\n', " ");
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return "...(no message text)...".to_string();
    }

    let excerpt = if let Some((start, end)) = bounds {
        let start_window = floor_boundary(&line, start.saturating_sub(110));
        let end_window = ceil_boundary(&line, (end + 160).min(line.len()));
        line[start_window..end_window].to_string()
    } else {
        line.chars().take(220).collect::<String>()
    };

    let compact = excerpt.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("...{}...", compact)
}

fn recency_score(now_unix: i64, updated_at_unix: i64) -> f64 {
    let age_seconds = (now_unix - updated_at_unix).max(0) as f64;
    // 48-hour decay constant keeps fresh chats at the top without hard cutoffs.
    (1.0 / (1.0 + age_seconds / (48.0 * 3600.0))).clamp(0.0, 1.0)
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn round_score(score: f64) -> f64 {
    (score * 1000.0).round() / 1000.0
}
