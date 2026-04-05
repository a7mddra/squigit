/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useCallback, useMemo, useRef, useState } from "react";
import { Check, Copy, RotateCcw, Pencil, ChevronRight } from "lucide-react";
import { CitationTip, CodeBlock } from "@/components";
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
  stripAttachmentMentions,
  attachmentFromPath,
  AttachmentStrip,
  Message,
  PendingAssistantTurn,
  ToolStep,
} from "@/features";
import styles from "./ChatBubble.module.css";
import mdStyles from "./BubbleMD.module.css";
import { ImageCollage } from "./ImageCollage";

interface ChatBubbleProps {
  message: Message;
  pendingTurn?: PendingAssistantTurn | null;
  onRetry?: () => void;
  retryDisabled?: boolean;
  copyDisabled?: boolean;
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
    return new URL(url).hostname.replace(/^www\./u, "");
  } catch {
    return url;
  }
}

function toCitationPreview(summary: string): string {
  const tokens = summary
    .trim()
    .split(/\s+/u)
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
    /^https?:\/\//iu.test(candidate) ||
    /^data:/iu.test(candidate) ||
    /^asset:/iu.test(candidate) ||
    /^tauri:/iu.test(candidate)
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

  const match = step.message?.trim().match(/^Thought for (\d+)s$/iu);
  if (!match) return 0;
  const value = Number.parseInt(match[1], 10);
  return Number.isFinite(value) && value > 0 ? value : 0;
}

function getCombinedThoughtSeconds(steps: ToolStep[]): number {
  return steps.reduce((sum, step) => sum + getToolStepThoughtSeconds(step), 0);
}

const CitationChip: React.FC<{
  citation: NonNullable<Message["citations"]>[number];
  animate?: boolean;
}> = ({ citation, animate = false }) => {
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
        className={`${styles.citationChip} ${
          animate ? styles.citationChipReveal : ""
        }`}
        onMouseEnter={() => setShowTip(true)}
        onMouseLeave={() => setShowTip(false)}
        onFocus={() => setShowTip(true)}
        onBlur={() => setShowTip(false)}
      >
        <img
          src={faviconUrl}
          alt=""
          className={styles.citationIcon}
          onError={(event) => {
            event.currentTarget.style.display = "none";
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
  pendingTurn = null,
  onRetry,
  retryDisabled = false,
  copyDisabled = false,
  onUndo,
  onAction,
  enableInternalLinks = false,
}) => {
  const app = useAppContext();
  const isUser = message.role === "user";
  const isPendingAssistant = !!pendingTurn && message.role === "model";
  const [isCopied, setIsCopied] = useState(false);

  const attachments = useMemo(() => {
    const paths = parseAttachmentPaths(message.text);
    return paths.map((path) => {
      const sourcePath = app.getAttachmentSourcePath(path) || undefined;
      const originalName = sourcePath ? getBaseName(sourcePath) : undefined;

      return attachmentFromPath(path, undefined, originalName, sourcePath);
    });
  }, [app.getAttachmentSourcePath, message.text]);
  const imageAttachments = useMemo(
    () => attachments.filter((attachment) => attachment.type === "image"),
    [attachments],
  );
  const fileAttachments = useMemo(
    () => attachments.filter((attachment) => attachment.type !== "image"),
    [attachments],
  );

  const toolSteps = pendingTurn?.toolSteps || message.toolSteps || [];
  const citations = pendingTurn?.visibleCitations || message.citations || [];
  const displayText = useMemo(() => {
    if (isPendingAssistant) {
      return pendingTurn?.displayText || "";
    }
    return stripAttachmentMentions(message.text);
  }, [isPendingAssistant, message.text, pendingTurn]);

  const thoughtBadgeText = useMemo(() => {
    if (isUser || message.role !== "model") return null;

    const explicitThoughtSeconds = pendingTurn?.thoughtSeconds ?? message.thoughtSeconds;
    if (
      typeof explicitThoughtSeconds === "number" &&
      Number.isFinite(explicitThoughtSeconds) &&
      explicitThoughtSeconds > 0
    ) {
      return `Thought for ${Math.round(explicitThoughtSeconds)}s`;
    }

    const combined = getCombinedThoughtSeconds(toolSteps);
    if (combined <= 0) return null;
    return `Thought for ${combined}s`;
  }, [isUser, message.role, message.thoughtSeconds, pendingTurn?.thoughtSeconds, toolSteps]);

  const isRichMarkdownUserMessage = useMemo(() => {
    if (!isUser) return false;
    return /(^|\n)\s*(#{1,6}\s|[-+*]\s|\d+\.\s|>\s|```|~~~|\$\$|\|.+\|)/mu.test(
      displayText,
    );
  }, [displayText, isUser]);

  const showThoughtLabel =
    !isUser &&
    !!thoughtBadgeText &&
    (isPendingAssistant || displayText.trim().length > 0);
  const isPendingStreaming =
    isPendingAssistant &&
    pendingTurn?.phase !== "complete" &&
    pendingTurn?.phase !== "stopped";
  const showCitations =
    !isUser &&
    citations.length > 0 &&
    (!isPendingAssistant ||
      pendingTurn?.phase === "complete" ||
      pendingTurn?.phase === "stopped");
  const hasDisplayText = displayText.trim().length > 0;
  const hasImageAttachments = imageAttachments.length > 0;
  const hasFileAttachments = fileAttachments.length > 0;
  const hasPendingShellContent = isPendingAssistant && !hasDisplayText;
  const isImageOnlyEmptyCaption =
    hasImageAttachments &&
    !hasDisplayText &&
    !hasFileAttachments &&
    !showCitations &&
    !hasPendingShellContent;
  const shouldRenderBubble = !isImageOnlyEmptyCaption;
  const shouldDoubleNewlines = isUser && !isRichMarkdownUserMessage;
  const canCopy = !copyDisabled && hasDisplayText;
  const shouldShowRetryButton = !isUser && message.role !== "system" && !!onRetry;
  const shouldShowCopyButton = message.role !== "system";
  const isPendingEmpty = isPendingAssistant && !hasDisplayText;
  const animateCitations = isPendingAssistant;

  const handleImageClick = useCallback(
    (attachment: (typeof imageAttachments)[number], index: number) => {
      void app.openMediaViewer(attachment, {
        isGallery: true,
        chatId: app.chatHistory.activeSessionId || undefined,
        galleryAttachments: imageAttachments,
        initialIndex: index,
        openedFromChat: true,
      });
    },
    [app, imageAttachments],
  );

  const handleCopy = () => {
    if (!canCopy) return;

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
            .replace(/\r?\n/gu, " ")
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
      window.setTimeout(() => setIsCopied(false), 2000);
    });
  };

  const markdownComponents = useMemo(
    () => ({
      code({ className, children, ...props }: any) {
        const match = /language-(\w+)/u.exec(className || "");
        const isInline = !match && !String(children).includes("\n");

        if (!isInline) {
          return (
            <CodeBlock
              language={match ? match[1] : ""}
              value={String(children).replace(/\n$/u, "")}
            />
          );
        }
        return (
          <code className={mdStyles.inlineCode} {...props}>
            {children}
          </code>
        );
      },

      a: ({ href, onClick, children, ...props }: any) => {
        const isInternalSettingsLink =
          enableInternalLinks &&
          typeof href === "string" &&
          href.startsWith("#settings-");

        if (isInternalSettingsLink) {
          const section = href
            .replace("#settings-", "")
            .split(/[?#]/u)[0]
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
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                if (section) onAction?.("open_settings", section);
              }}
              onKeyDown={(event: React.KeyboardEvent) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
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

  return (
    <div
      className={`${styles.wrapper} ${isUser ? styles.userAlign : styles.botAlign}`}
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
            className={styles.messageAnchor}
            data-component="chat-bubble"
            data-message-index={message.id}
            data-message-role={isUser ? "user" : "assistant"}
          >
            {hasImageAttachments && (
              <ImageCollage
                images={imageAttachments}
                onImageClick={handleImageClick}
                className={`${styles.imageCollage} ${
                  isUser ? styles.imageCollageUserFrame : styles.imageCollageBotFrame
                } ${
                  shouldRenderBubble ? styles.imageCollageWithBubble : ""
                }`}
              />
            )}

            {shouldRenderBubble && (
              <div
                dir="auto"
                className={`${styles.bubble} ${
                  isUser ? styles.userBubble : styles.botBubble
                } ${
                  isUser && isRichMarkdownUserMessage ? styles.userRichBubble : ""
                } ${isPendingAssistant ? styles.pendingBubbleShell : ""} ${
                  isPendingEmpty ? styles.pendingBubbleEmpty : ""
                } ${
                  pendingTurn?.phase === "streaming" ||
                  pendingTurn?.phase === "finalizing"
                    ? styles.livePendingBubble
                    : ""
                }`}
              >
                {hasFileAttachments && (
                  <div
                    style={{
                      marginBottom: hasDisplayText ? "8px" : "0",
                    }}
                  >
                    <AttachmentStrip
                      attachments={fileAttachments}
                      onClick={app.openMediaViewer}
                      readOnly
                    />
                  </div>
                )}

                <div
                  className={`${styles.markdownFrame} ${
                    isPendingAssistant ? styles.pendingMarkdownFrame : ""
                  }`}
                >
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
                      {preprocessMarkdown(displayText, {
                        doubleNewlines: shouldDoubleNewlines,
                      })}
                    </ReactMarkdown>
                  </div>
                </div>

                {showCitations && (
                  <div
                    className={`${styles.citationFooter} ${
                      animateCitations ? styles.citationFooterReveal : ""
                    }`}
                  >
                    {citations.map((citation, index) => (
                      <CitationChip
                        key={`${citation.url}-${index}`}
                        citation={citation}
                        animate={animateCitations}
                      />
                    ))}
                  </div>
                )}
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

            {!isUser && shouldShowRetryButton && !isPendingStreaming && (
              <button
                onClick={onRetry}
                title="Retry"
                aria-label="Retry"
                disabled={retryDisabled}
              >
                <RotateCcw size={14} />
              </button>
            )}

            {shouldShowCopyButton && !isPendingStreaming && (
              <button
                onClick={handleCopy}
                title="Copy"
                aria-label="Copy"
                disabled={!canCopy}
              >
                {isCopied ? <Check size={14} /> : <Copy size={14} />}
              </button>
            )}

            {isUser && onUndo && (
              <button
                onClick={onUndo}
                title="Undo and Edit"
                aria-label="Undo and Edit"
              >
                <Pencil size={14} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export const ChatBubble = React.memo(
  ChatBubbleComponent,
  (prevProps, nextProps) => {
    return (
      prevProps.message === nextProps.message &&
      prevProps.pendingTurn === nextProps.pendingTurn &&
      prevProps.retryDisabled === nextProps.retryDisabled &&
      prevProps.copyDisabled === nextProps.copyDisabled &&
      !!prevProps.onRetry === !!nextProps.onRetry &&
      !!prevProps.onUndo === !!nextProps.onUndo &&
      prevProps.enableInternalLinks === nextProps.enableInternalLinks
    );
  },
);
