/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useState, useEffect, useRef } from "react";
import { Check, Copy, RotateCcw, Pencil } from "lucide-react";
import { CodeBlock, TextShimmer } from "@/components";
import { useAppContext } from "@/providers/AppProvider";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import {
  parseMarkdownToSegments,
  tokenizeSegments,
  preprocessMarkdown,
  remarkDisableIndentedCode,
  type StreamSegment,
} from "@/lib";
import {
  parseAttachmentPaths,
  stripAttachmentMentions,
  attachmentFromPath,
  AttachmentStrip,
  Message,
} from "@/features";
import katex from "katex";
import styles from "./ChatBubble.module.css";

interface ChatBubbleProps {
  message: Message;
  isStreamed?: boolean;
  onStreamComplete?: () => void;
  onTypingChange?: (isTyping: boolean) => void;
  stopRequested?: boolean;
  onStopGeneration?: (truncatedText: string) => void;
  onRetry?: () => void;
  isRetrying?: boolean;
  onUndo?: () => void;
  onAction?: (actionId: string, value?: string) => void;
}

const katexCache = new Map<string, string>();
function renderKatex(latex: string, displayMode: boolean): string {
  const key = `${displayMode ? "D" : "I"}:${latex}`;
  const cached = katexCache.get(key);
  if (cached !== undefined) return cached;
  const html = katex.renderToString(latex, {
    displayMode,
    throwOnError: false,
  });
  katexCache.set(key, html);
  return html;
}

function renderTextWithInlineMath(content: string, keyPrefix: string) {
  const nodes: React.ReactNode[] = [];
  let cursor = 0;
  let keyIndex = 0;

  while (cursor < content.length) {
    const open = content.indexOf("$", cursor);
    if (open === -1) break;

    const isEscaped = open > 0 && content[open - 1] === "\\";
    const isBlockFence = open + 1 < content.length && content[open + 1] === "$";
    if (isEscaped || isBlockFence) {
      cursor = open + 1;
      continue;
    }

    let close = open + 1;
    while (close < content.length) {
      if (content[close] === "$" && content[close - 1] !== "\\") break;
      close += 1;
    }
    if (close >= content.length) break;

    if (open > cursor) {
      nodes.push(
        <React.Fragment key={`${keyPrefix}-t-${keyIndex++}`}>
          {content.slice(cursor, open)}
        </React.Fragment>,
      );
    }

    const latex = content.slice(open + 1, close).trim();
    if (latex) {
      try {
        const html = renderKatex(latex, false);
        nodes.push(
          <span
            key={`${keyPrefix}-m-${keyIndex++}`}
            dangerouslySetInnerHTML={{ __html: html }}
          />,
        );
      } catch {
        nodes.push(
          <code
            key={`${keyPrefix}-c-${keyIndex++}`}
            className={styles.inlineCode}
          >
            {`$${latex}$`}
          </code>,
        );
      }
    }

    cursor = close + 1;
  }

  if (cursor < content.length) {
    nodes.push(
      <React.Fragment key={`${keyPrefix}-tail`}>
        {content.slice(cursor)}
      </React.Fragment>,
    );
  }

  return nodes.length > 0 ? nodes : content;
}

function getBaseName(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

const ChatBubbleComponent: React.FC<ChatBubbleProps> = ({
  message,
  isStreamed = false,
  onStreamComplete,
  onTypingChange,
  stopRequested,
  onStopGeneration,
  onRetry,
  isRetrying,
  onUndo,
}) => {
  const app = useAppContext();
  const isUser = message.role === "user";
  const [isCopied, setIsCopied] = useState(false);

  const attachments = useMemo(() => {
    const paths = parseAttachmentPaths(message.text);
    return paths.map((p) => {
      const sourcePath = app.getAttachmentSourcePath(p) || undefined;
      const originalName = sourcePath ? getBaseName(sourcePath) : undefined;

      return attachmentFromPath(p, undefined, originalName, sourcePath);
    });
  }, [app.getAttachmentSourcePath, message.text]);

  const displayText = useMemo(() => {
    return stripAttachmentMentions(message.text);
  }, [message.text]);
  const isRichMarkdownUserMessage = useMemo(() => {
    if (!isUser) return false;
    return /(^|\n)\s*(#{1,6}\s|[-+*]\s|\d+\.\s|>\s|```|~~~|\$\$|\|.+\|)/m.test(
      displayText,
    );
  }, [displayText, isUser]);

  const [revealedCount, setRevealedCount] = useState(0);
  const [isStreamingComplete, setIsStreamingComplete] = useState(!isStreamed);
  const timeoutRef = useRef<number | null>(null);
  const shouldRenderStreaming = !isUser && isStreamed;

  const handleCopy = () => {
    navigator.clipboard.writeText(displayText).then(() => {
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    });
  };

  const { segments, tokens } = useMemo(() => {
    if (isUser) return { segments: [], tokens: [] };

    const backtickCount = (displayText.match(/```/g) || []).length;
    const isUnclosedCodeBlock = backtickCount % 2 === 1;

    const textToParse = isUnclosedCodeBlock
      ? displayText + "\n```"
      : displayText;

    const segs = parseMarkdownToSegments(textToParse);
    const toks = tokenizeSegments(segs);
    return { segments: segs, tokens: toks };
  }, [displayText, isUser]);

  useEffect(() => {
    if (isStreamed) {
      if (message.stopped) {
        setIsStreamingComplete(true);
        return;
      }
      setRevealedCount(0);
      setIsStreamingComplete(false);
    } else {
      setIsStreamingComplete(true);
    }
  }, [isStreamed, message.stopped]);

  useEffect(() => {
    if (isStreamingComplete || !isStreamed) {
      if (isStreamed && !isStreamingComplete) {
        setIsStreamingComplete(true);
        onStreamComplete?.();
      }
      return;
    }

    if (revealedCount >= tokens.length) {
      setIsStreamingComplete(true);
      onStreamComplete?.();
      return;
    }

    const currentToken = tokens[revealedCount];
    const delay = currentToken?.delay ?? 25;

    timeoutRef.current = window.setTimeout(() => {
      setRevealedCount((prev) => prev + 1);
    }, delay);

    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [
    revealedCount,
    tokens,
    isStreamingComplete,
    isStreamed,
    onStreamComplete,
  ]);

  const isTyping = isStreamed && !isStreamingComplete && !message.stopped;
  const prevIsTypingRef = useRef(false);

  useEffect(() => {
    if (prevIsTypingRef.current !== isTyping) {
      prevIsTypingRef.current = isTyping;
      onTypingChange?.(isTyping);
    }
  }, [isTyping, onTypingChange]);

  const stoppedRef = useRef(false);

  useEffect(() => {
    if (!stopRequested || !isTyping || stoppedRef.current) return;
    stoppedRef.current = true;

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    const revealed = tokens.slice(0, revealedCount);
    const grouped = new Map<number, string[]>();
    revealed.forEach((token) => {
      if (!grouped.has(token.segmentIndex)) {
        grouped.set(token.segmentIndex, []);
      }
      grouped.get(token.segmentIndex)!.push(token.text);
    });

    let truncated = "";
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      const texts = grouped.get(i);
      if (!texts) break;

      if (seg.type === "codeblock") {
        let isEffectiveLast = true;
        for (let j = i + 1; j < segments.length; j++) {
          if (grouped.has(j)) {
            isEffectiveLast = false;
            break;
          }
        }
        if (isEffectiveLast) {
          break;
        }
        const lang = seg.meta?.language || "";
        truncated += "```" + lang + "\n" + seg.content + "\n```\n";
      } else if (seg.type === "mathblock") {
        let isEffectiveLast = true;
        for (let j = i + 1; j < segments.length; j++) {
          if (grouped.has(j)) {
            isEffectiveLast = false;
            break;
          }
        }
        if (isEffectiveLast) {
          break;
        }
        truncated += "$$\n" + seg.content + "\n$$\n";
      } else if (seg.type === "math") {
        truncated += "$" + texts.join("") + "$";
      } else if (seg.type === "listItem") {
        const prefix = seg.meta?.ordered
          ? `${seg.meta.listIndex ?? 1}. `
          : "- ";
        truncated += prefix + texts.join("") + "\n";
      } else {
        truncated += texts.join("");
      }
    }

    onStopGeneration?.(truncated.trimEnd());
  }, [stopRequested]);

  const revealedSegments = useMemo(() => {
    if (isUser) return new Map();

    const count =
      !isStreamed || isStreamingComplete ? tokens.length : revealedCount;

    const revealed = tokens.slice(0, count);
    const grouped = new Map<number, string[]>();

    revealed.forEach((token) => {
      if (!grouped.has(token.segmentIndex)) {
        grouped.set(token.segmentIndex, []);
      }
      grouped.get(token.segmentIndex)!.push(token.text);
    });

    return grouped;
  }, [tokens, revealedCount, isStreamingComplete, isStreamed, isUser]);

  const renderStreamSegment = (segment: StreamSegment, index: number) => {
    const revealedTexts = revealedSegments.get(index);
    if (!revealedTexts && !(!isStreamed || isStreamingComplete)) return null;

    if (segment.type === "codeblock") {
      const lang = (segment.meta?.language || "").toLowerCase();
      const isMathLang = [
        "latex",
        "tex",
        "math",
        "katex",
        "markdown",
        "md",
      ].includes(lang);

      const isTyping = isStreamed && !isStreamingComplete;

      let isEffectiveLast = true;
      for (let i = index + 1; i < segments.length; i++) {
        if (segments[i].content.trim().length > 0) {
          isEffectiveLast = false;
          break;
        }
      }

      if (isTyping && isEffectiveLast) {
        return (
          <div key={index} className={styles.typingShimmer}>
            <TextShimmer
              text={isMathLang ? "Rendering Math" : "Writing Code"}
            />
          </div>
        );
      }

      if (isMathLang) {
        try {
          const html = renderKatex(segment.content, true);
          return (
            <div
              key={index}
              className={`${styles.mathBlock} ${isStreamed ? styles.popIn : ""}`}
              dangerouslySetInnerHTML={{ __html: html }}
            />
          );
        } catch {
          return <pre key={index}>{segment.content}</pre>;
        }
      }

      return (
        <div key={index} className={isStreamed ? styles.popIn : undefined}>
          <CodeBlock
            language={segment.meta?.language || ""}
            value={segment.content}
          />
        </div>
      );
    }

    if (segment.type === "mathblock") {
      const isTyping = isStreamed && !isStreamingComplete;

      let isEffectiveLast = true;
      for (let i = index + 1; i < segments.length; i++) {
        if (segments[i].content.trim().length > 0) {
          isEffectiveLast = false;
          break;
        }
      }

      if (isTyping && isEffectiveLast) {
        return (
          <div key={index} className={styles.typingShimmer}>
            <TextShimmer text="Rendering Math" />
          </div>
        );
      }

      try {
        const html = renderKatex(segment.content, true);
        return (
          <div
            key={index}
            className={`${styles.mathBlock} ${isStreamed ? styles.popIn : ""}`}
            dangerouslySetInnerHTML={{ __html: html }}
          />
        );
      } catch {
        return <pre key={index}>{segment.content}</pre>;
      }
    }

    if (segment.type === "math") {
      try {
        const html = renderKatex(
          revealedTexts ? revealedTexts.join("") : segment.content,
          false,
        );
        return <span key={index} dangerouslySetInnerHTML={{ __html: html }} />;
      } catch {
        return (
          <code key={index} className={styles.inlineCode}>
            {segment.content}
          </code>
        );
      }
    }

    const content = revealedTexts ? revealedTexts.join("") : segment.content;

    switch (segment.type) {
      case "text": {
        if (!content.trim()) {
          if (content.includes("\n\n")) {
            return <div key={index} className={styles.paragraphGap} />;
          }
          return null;
        }
        return <span key={index}>{content}</span>;
      }

      case "bold":
        return <strong key={index}>{content}</strong>;

      case "italic":
        return <em key={index}>{content}</em>;

      case "code":
        return (
          <code key={index} className={styles.inlineCode}>
            {content}
          </code>
        );

      case "heading": {
        const level = segment.meta?.level || 1;
        const Tag =
          `h${Math.min(level, 6)}` as keyof React.JSX.IntrinsicElements;
        const sizes = [1.5, 1.3, 1.15, 1.05, 1, 0.95];
        const topMargins = [
          "1.25em",
          "1em",
          "0.875em",
          "0.75em",
          "0.625em",
          "0.5em",
        ];
        const bottomMargins = [
          "0.5em",
          "0.4em",
          "0.375em",
          "0.3em",
          "0.25em",
          "0.25em",
        ];
        return (
          <Tag
            key={index}
            className={styles.heading}
            style={{
              fontSize: `${sizes[level - 1]}em`,
              marginTop: topMargins[level - 1],
              marginBottom: bottomMargins[level - 1],
            }}
          >
            {renderTextWithInlineMath(content, `heading-${index}`)}
          </Tag>
        );
      }

      case "listItem":
        return (
          <div
            key={index}
            className={styles.listItem}
            style={{
              listStyleType: segment.meta?.ordered ? "decimal" : "disc",
            }}
          >
            {renderTextWithInlineMath(content, `list-${index}`)}
          </div>
        );

      case "blockquote":
        return (
          <blockquote key={index} className={styles.blockquote}>
            {renderTextWithInlineMath(content, `quote-${index}`)}
          </blockquote>
        );

      case "link":
        return (
          <a key={index} href={segment.meta?.href} className={styles.link}>
            {renderTextWithInlineMath(content, `link-${index}`)}
          </a>
        );

      case "break":
        return <br key={index} />;

      default:
        return <span key={index}>{content}</span>;
    }
  };

  const markdownComponents = useMemo(
    () => ({
      code({ node, className, children, ...props }: any) {
        const match = /language-(\w+)/.exec(className || "");
        const isInline = !match && !String(children).includes("\n");

        if (!isInline) {
          return (
            <CodeBlock
              language={match ? match[1] : ""}
              value={String(children).replace(/\n$/, "")}
            />
          );
        }
        return (
          <code className={styles.inlineCode} {...props}>
            {children}
          </code>
        );
      },

      a: ({ node, ...props }: any) => (
        <a
          {...props}
          target="_blank"
          rel="noopener noreferrer"
          className={styles.link}
        />
      ),
    }),
    [],
  );

  return (
    <>
      <div
        className={`${styles.wrapper} ${
          isUser ? styles.userAlign : styles.botAlign
        }`}
      >
        <div
          className={`${styles.container} ${
            isUser ? styles.userContainer : styles.botContainer
          } ${isUser && isRichMarkdownUserMessage ? styles.userRichContainer : ""}`}
        >
          <div
            className={`${styles.contentColumn} ${
              isUser ? styles.userContent : styles.botContent
            } ${isUser && isRichMarkdownUserMessage ? styles.userRichContent : ""}`}
          >
            <div
              dir="auto"
              data-component="chat-bubble"
              data-message-index={message.id}
              data-message-role={isUser ? "user" : "assistant"}
              className={`${styles.bubble} ${
                isUser ? styles.userBubble : styles.botBubble
              } ${
                isUser && isRichMarkdownUserMessage ? styles.userRichBubble : ""
              }`}
            >
              {attachments.length > 0 && (
                <div
                  style={{
                    marginBottom: displayText.length > 0 ? "8px" : "0",
                  }}
                >
                  <AttachmentStrip
                    attachments={attachments}
                    onClick={app.openMediaViewer}
                    readOnly
                  />
                </div>
              )}
              {isRetrying ? (
                <TextShimmer text="Regenerating response..." />
              ) : !isUser ? (
                shouldRenderStreaming ? (
                  <div className={styles.streamingContainer}>
                    {segments.map((segment, index) =>
                      renderStreamSegment(segment, index),
                    )}
                    {!isStreamingComplete && isStreamed && (
                      <span className={styles.cursor}>▋</span>
                    )}
                  </div>
                ) : (
                  <div className={styles.markdownContent}>
                    <ReactMarkdown
                      remarkPlugins={[
                        remarkGfm,
                        remarkMath,
                        remarkDisableIndentedCode,
                      ]}
                      rehypePlugins={[rehypeKatex]}
                      components={markdownComponents}
                    >
                      {preprocessMarkdown(displayText)}
                    </ReactMarkdown>
                  </div>
                )
              ) : (
                <div
                  className={`${styles.markdownContent} ${styles.userMarkdownContent}`}
                >
                  <ReactMarkdown
                    remarkPlugins={[
                      remarkGfm,
                      remarkMath,
                      remarkDisableIndentedCode,
                    ]}
                    rehypePlugins={[rehypeKatex]}
                    components={markdownComponents}
                  >
                    {preprocessMarkdown(displayText, {
                      doubleNewlines: isUser,
                    })}
                  </ReactMarkdown>
                </div>
              )}
            </div>

            {message.image && (
              <div className={styles.imageWrapper}>
                <img
                  src={
                    message.image.startsWith("data:")
                      ? message.image
                      : `data:image/jpeg;base64,${message.image}`
                  }
                  alt="Analyzed content"
                  className={styles.image}
                />
                <div className={styles.imageCaption}>Analyzed image</div>
              </div>
            )}

            <div
              className={`${styles.footer} ${
                isUser ? styles.userFooter : styles.botFooter
              }`}
            >
              {isUser && (
                <span className={styles.timestamp}>
                  {new Date(message.timestamp).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </span>
              )}

              {!isStreamingComplete && isStreamed ? null : isRetrying ||
                message.role === "system" ? null : (
                <>
                  {!isUser && onRetry && (
                    <button onClick={onRetry} title="Retry" aria-label="Retry">
                      <RotateCcw size={14} />
                    </button>
                  )}
                  <button onClick={handleCopy} title="Copy" aria-label="Copy">
                    {isCopied ? <Check size={14} /> : <Copy size={14} />}
                  </button>
                  {isUser && onUndo && (
                    <button
                      onClick={onUndo}
                      title="Undo and Edit"
                      aria-label="Undo and Edit"
                    >
                      <Pencil size={14} />
                    </button>
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
};

export const ChatBubble = React.memo(
  ChatBubbleComponent,
  (prevProps, nextProps) => {
    return (
      prevProps.message.id === nextProps.message.id &&
      prevProps.message.text === nextProps.message.text &&
      prevProps.isStreamed === nextProps.isStreamed &&
      prevProps.stopRequested === nextProps.stopRequested &&
      !!prevProps.onRetry === !!nextProps.onRetry &&
      prevProps.isRetrying === nextProps.isRetrying &&
      !!prevProps.onUndo === !!nextProps.onUndo
    );
  },
);
