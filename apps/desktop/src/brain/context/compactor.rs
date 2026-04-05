// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Memory module — Builds summarization prompts for context compaction.
//!
//! Inspired by OpenAI Codex's compaction architecture but adapted for
//! Squigit's chat UX: rolling summary + verbatim window instead of
//! full-history replacement.

/// Approximate bytes per token (same heuristic as OpenAI Codex).
/// Used for lightweight token estimation without a tokenizer dependency.
pub const APPROX_BYTES_PER_TOKEN: usize = 4;

/// Estimate token count from a string using byte-length heuristic.
pub fn approx_token_count(text: &str) -> usize {
    text.len()
        .saturating_add(APPROX_BYTES_PER_TOKEN - 1)
        / APPROX_BYTES_PER_TOKEN
}

/// Build the summarization prompt that compresses older conversation turns
/// into a concise rolling summary.
///
/// The prompt instructs the model to preserve:
/// - Key decisions and conclusions
/// - User's original intent and preferences
/// - Technical context (errors, code, tools mentioned)
/// - Unresolved questions or pending items
///
/// # Arguments
/// * `image_brief` — Short description of the analyzed image (context anchor)
/// * `history_to_compress` — Formatted older turns to compress
pub fn build_summary_prompt(image_brief: &str, history_to_compress: &str) -> String {
    format!(
        r#"You are performing a CONTEXT CHECKPOINT COMPACTION for a screen analysis conversation.

## Image Context
The conversation is about this screen capture: {image_brief}

## Conversation History to Compress
{history_to_compress}

## Instructions
Create a concise bullet-point summary that captures:
- Key decisions and conclusions reached
- User's original intent and any preference changes
- Technical context: errors, code snippets, tools, or URLs mentioned
- Unresolved questions or pending items
- Any specific data the user asked to remember

## Rules
- Use 4-8 bullet points maximum
- Be factual and specific — no vague summaries
- Preserve exact values: file paths, error codes, version numbers
- Do NOT include greetings or conversational filler
- Write as if handing off to another assistant who will continue the conversation
- Output ONLY the bullet points, no headers or extra formatting"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_token_count() {
        // "hello" = 5 bytes → ceil(5/4) = 2 tokens
        assert_eq!(approx_token_count("hello"), 2);
        // empty
        assert_eq!(approx_token_count(""), 0);
        // 4 bytes exactly = 1 token
        assert_eq!(approx_token_count("abcd"), 1);
        // 8 bytes = 2 tokens
        assert_eq!(approx_token_count("abcdefgh"), 2);
    }

    #[test]
    fn test_build_summary_prompt() {
        let prompt = build_summary_prompt(
            "VS Code with a Rust file open",
            "**User**: Fix this bug\n**Assistant**: I see the issue...",
        );
        assert!(prompt.contains("VS Code with a Rust file open"));
        assert!(prompt.contains("Fix this bug"));
        assert!(prompt.contains("bullet"));
    }
}
