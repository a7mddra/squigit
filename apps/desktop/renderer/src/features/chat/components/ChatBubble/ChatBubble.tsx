/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useMemo, useState, useEffect, useRef } from "react";
import { Check, Copy, RotateCcw, Pencil, ChevronRight } from "lucide-react";
import { CitationTip, CodeBlock, TextShimmer } from "@/components";
import { useAppContext } from "@/providers/AppProvider";
import { convertFileSrc } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import rehypeRaw from "rehype-raw";
import { preprocessMarkdown, remarkDisableIndentedCode } from "@/lib";
import {
  parseAttachmentPaths,
  useSmoothStream,
  stripAttachmentMentions,
  attachmentFromPath,
  AttachmentStrip,
  Message,
  ToolStep,
} from "@/features";
import styles from "./ChatBubble.module.css";
import mdStyles from "./BubbleMD.module.css";

interface ChatBubbleProps {
  message: Message;
  isStreamed?: boolean;
  onStreamComplete?: () => void;
  onTypingChange?: (isTyping: boolean) => void;
  stopRequested?: boolean;
  onStopGeneration?: (truncatedText: string) => void;
  onRetry?: () => void;
  onUndo?: () => void;
  onAction?: (actionId: string, value?: string) => void;
  enableInternalLinks?: boolean;
}

function getBaseName(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

const CITATION_PREVIEW_TOKENS = 28;

function getCitationDomain(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

function toCitationPreview(summary: string): string {
  const tokens = summary
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (tokens.length === 0) {
    return "No summary available...";
  }
  return `${tokens.slice(0, CITATION_PREVIEW_TOKENS).join(" ")}...`;
}

function resolveCitationFavicon(
  favicon: string | undefined,
  domain: string,
): string {
  const fallback = `https://www.google.com/s2/favicons?domain=${domain}&sz=32`;
  const candidate = favicon?.trim();
  if (!candidate) {
    return fallback;
  }

  if (
    /^https?:\/\//i.test(candidate) ||
    /^data:/i.test(candidate) ||
    /^asset:/i.test(candidate) ||
    /^tauri:/i.test(candidate)
  ) {
    return candidate;
  }

  try {
    return convertFileSrc(candidate);
  } catch {
    return fallback;
  }
}

function getToolStepThoughtSeconds(step: ToolStep): number {
  if (
    typeof step.startedAtMs === "number" &&
    typeof step.endedAtMs === "number" &&
    step.endedAtMs >= step.startedAtMs
  ) {
    return Math.max(1, Math.round((step.endedAtMs - step.startedAtMs) / 1000));
  }

  const match = step.message?.trim().match(/^Thought for (\d+)s$/i);
  if (!match) return 0;
  const value = Number.parseInt(match[1], 10);
  return Number.isFinite(value) && value > 0 ? value : 0;
}

function getCombinedThoughtSeconds(steps: ToolStep[]): number {
  return steps.reduce((sum, step) => sum + getToolStepThoughtSeconds(step), 0);
}

const CitationChip: React.FC<{
  citation: NonNullable<Message["citations"]>[number];
}> = ({
  citation,
}) => {
  const [showTip, setShowTip] = useState(false);
  const anchorRef = useRef<HTMLAnchorElement>(null);

  const domain = useMemo(() => getCitationDomain(citation.url), [citation.url]);
  const faviconUrl = useMemo(
    () => resolveCitationFavicon(citation.favicon, domain),
    [citation.favicon, domain],
  );
  const preview = useMemo(
    () => toCitationPreview(citation.summary || ""),
    [citation.summary],
  );

  return (
    <>
      <a
        ref={anchorRef}
        href={citation.url}
        target="_blank"
        rel="noopener noreferrer"
        className={styles.citationChip}
        onMouseEnter={() => setShowTip(true)}
        onMouseLeave={() => setShowTip(false)}
        onFocus={() => setShowTip(true)}
        onBlur={() => setShowTip(false)}
      >
        <img
          src={faviconUrl}
          alt=""
          className={styles.citationIcon}
          onError={(e) => {
            e.currentTarget.style.display = "none";
          }}
        />
        <span className={styles.citationTitle}>{citation.title || domain}</span>
      </a>
      <CitationTip
        parentRef={anchorRef}
        show={showTip}
        headerUrl={citation.url}
        headerIconUrl={faviconUrl}
        body={preview}
      />
    </>
  );
};

const ChatBubbleComponent: React.FC<ChatBubbleProps> = ({
  message,
  isStreamed = false,
  onStreamComplete,
  onTypingChange,
  stopRequested,
  onStopGeneration,
  onRetry,
  onUndo,

  onAction,
  enableInternalLinks = false,
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

  const toolSteps = message.toolSteps || [];
  const citations = message.citations || [];
  const thoughtBadgeText = useMemo(() => {
    if (isUser || message.role !== "model") return null;

    if (
      typeof message.thoughtSeconds === "number" &&
      Number.isFinite(message.thoughtSeconds) &&
      message.thoughtSeconds > 0
    ) {
      return `Thought for ${Math.round(message.thoughtSeconds)}s`;
    }

    const combined = getCombinedThoughtSeconds(toolSteps);
    if (combined <= 0) return null;
    return `Thought for ${combined}s`;
  }, [isUser, message.role, message.thoughtSeconds, toolSteps]);

  const displayText = useMemo(() => {
    return stripAttachmentMentions(message.text);
  }, [message.text]);

  const isRichMarkdownUserMessage = useMemo(() => {
    if (!isUser) return false;
    return /(^|\n)\s*(#{1,6}\s|[-+*]\s|\d+\.\s|>\s|```|~~~|\$\$|\|.+\|)/m.test(
      displayText,
    );
  }, [displayText, isUser]);

  const handleCopy = () => {
    let copyText = displayText;
    if (!isUser && citations.length > 0) {
      const uniqueByUrl = new Map<string, (typeof citations)[number]>();
      for (const citation of citations) {
        if (!citation.url || uniqueByUrl.has(citation.url)) continue;
        uniqueByUrl.set(citation.url, citation);
      }

      if (uniqueByUrl.size > 0) {
        const sourceLines = Array.from(uniqueByUrl.values()).map((citation) => {
          const title = (citation.title || getCitationDomain(citation.url))
            .replace(/\r?\n/g, " ")
            .trim();
          return `- [${title}](${citation.url})`;
        });
        copyText =
          displayText.trim().length > 0
            ? `${displayText}\n\nSources:\n${sourceLines.join("\n")}`
            : `Sources:\n${sourceLines.join("\n")}`;
      }
    }

    navigator.clipboard.writeText(copyText).then(() => {
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    });
  };

  const isBotStreaming = !isUser && isStreamed && !message.stopped;

  const { text: smoothText, isWritingCode } = useSmoothStream(
    displayText,
    isBotStreaming,
  );

  const prevIsTypingRef = useRef(false);

  useEffect(() => {
    if (prevIsTypingRef.current !== isBotStreaming) {
      prevIsTypingRef.current = isBotStreaming;
      onTypingChange?.(isBotStreaming);
      if (!isBotStreaming) {
        onStreamComplete?.();
      }
    }
  }, [isBotStreaming, onTypingChange, onStreamComplete]);

  useEffect(() => {
    return () => {
      if (prevIsTypingRef.current) {
        prevIsTypingRef.current = false;
        onTypingChange?.(false);
      }
    };
  }, [onTypingChange]);

  const stoppedRef = useRef(false);

  useEffect(() => {
    if (!stopRequested) {
      stoppedRef.current = false;
    }
  }, [stopRequested]);

  useEffect(() => {
    if (!stopRequested || !isBotStreaming || stoppedRef.current) return;
    stoppedRef.current = true;
    onStopGeneration?.(smoothText.trimEnd());
  }, [stopRequested, isBotStreaming, smoothText, onStopGeneration]);

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
          <code className={mdStyles.inlineCode} {...props}>
            {children}
          </code>
        );
      },

      a: ({ node, href, onClick, children, ...props }: any) => {
        const isInternalSettingsLink =
          enableInternalLinks &&
          typeof href === "string" &&
          href.startsWith("#settings-");

        if (isInternalSettingsLink) {
          const section = href
            .replace("#settings-", "")
            .split(/[?#]/)[0]
            .trim()
            .toLowerCase();

          return (
            <span
              role="button"
              tabIndex={0}
              className={mdStyles.link}
              style={{
                cursor: "pointer",
                fontWeight: 700,
                color: "var(--c-raw-011)",
              }}
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                if (section) onAction?.("open_settings", section);
              }}
              onKeyDown={(e: React.KeyboardEvent) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  if (section) onAction?.("open_settings", section);
                }
              }}
            >
              {children}
            </span>
          );
        }

        return (
          <a
            {...props}
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className={mdStyles.link}
            onClick={onClick}
          >
            {children}
          </a>
        );
      },

      li({ checked, children, ...props }: any) {
        if (typeof checked === "boolean") {
          return (
            <li className={mdStyles.taskListItem} {...props}>
              <input type="checkbox" checked={checked} disabled readOnly />
              {children}
            </li>
          );
        }
        return <li {...props}>{children}</li>;
      },

      details: ({ children }: any) => (
        <details className={mdStyles.details}>{children}</details>
      ),

      summary: ({ children }: any) => (
        <summary className={mdStyles.summary}>
          <ChevronRight className={mdStyles.chevron} size={18} />
          {children}
        </summary>
      ),
    }),
    [enableInternalLinks, onAction],
  );

  const markdownToRender = isBotStreaming ? smoothText : displayText;

  const shouldDoubleNewlines = isUser && !isRichMarkdownUserMessage;
  const showThoughtLabel =
    !isUser && !!thoughtBadgeText && displayText.trim().length > 0;

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
            {!isUser && showThoughtLabel && (
              <div className={styles.thoughtLabelRow}>
                <span className={styles.thoughtBadge}>{thoughtBadgeText}</span>
              </div>
            )}
            <div
              dir="auto"
              data-component="chat-bubble"
              data-message-index={message.id}
              data-message-role={isUser ? "user" : "assistant"}
              className={`${styles.bubble} ${
                isUser ? styles.userBubble : styles.botBubble
              } ${
                isUser && isRichMarkdownUserMessage ? styles.userRichBubble : ""
              } ${isBotStreaming ? mdStyles.liveStream : ""}`}
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
              <div
                className={
                  isUser
                    ? `${mdStyles.markdownContent} ${mdStyles.userMarkdownContent}`
                    : mdStyles.markdownContent
                }
              >
                <ReactMarkdown
                  remarkPlugins={[
                    remarkGfm,
                    remarkMath,
                    remarkDisableIndentedCode,
                  ]}
                  rehypePlugins={[rehypeKatex, rehypeRaw]}
                  components={markdownComponents}
                >
                  {preprocessMarkdown(markdownToRender, {
                    doubleNewlines: shouldDoubleNewlines,
                  })}
                </ReactMarkdown>
                {isWritingCode && <TextShimmer text="Writing code..." />}
              </div>
              {!isUser && citations.length > 0 && (
                <div className={styles.citationFooter}>
                  {citations.map((citation, index) => (
                    <CitationChip
                      key={`${citation.url}-${index}`}
                      citation={citation}
                    />
                  ))}
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

              {isBotStreaming || message.role === "system" ? null : (
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
      prevProps.message.thoughtSeconds === nextProps.message.thoughtSeconds &&
      prevProps.message.citations === nextProps.message.citations &&
      prevProps.message.toolSteps === nextProps.message.toolSteps &&
      prevProps.isStreamed === nextProps.isStreamed &&
      prevProps.stopRequested === nextProps.stopRequested &&
      !!prevProps.onRetry === !!nextProps.onRetry &&
      !!prevProps.onUndo === !!nextProps.onUndo &&
      prevProps.enableInternalLinks === nextProps.enableInternalLinks
    );
  },
);
