/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ErrorInfo,
} from "react";
import { Check, Copy, RotateCcw, Pencil, ChevronRight } from "lucide-react";
import {
  CitationChip,
  CitationTip,
  CodeBlock,
  TextShimmer,
} from "@/components/ui";
import { useMediaContext } from "@/app/context/AppMedia";
import { convertFileSrc } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import rehypeRaw from "rehype-raw";
import {
  attachmentFromPath,
  unwrapMarkdownLinkDestination,
  isAttachmentPath,
  normalizeAttachmentMarkdownLinks,
  parseAttachmentPaths,
  stripImageAttachmentMentions,
  API_STATUS_TEXT,
} from "@/core";
import {
  Message,
  ToolStep,
  MessageCollapseMode,
  PendingAssistantTurn,
} from "../../chat.types";
import {
  preprocessMarkdown,
  splitMarkdownAfterLastClosedFence,
} from "../../utils/markdownUtils";
import { remarkDisableIndentedCode } from "../../utils/markdownPlugins";
import styles from "./ChatBubble.module.css";
import mdStyles from "./BubbleMD.module.css";
import { ImageCollage } from "./ImageCollage";

interface ChatBubbleProps {
  chatId?: string | null;
  message: Message;
  pendingTurn?: PendingAssistantTurn | null;
  onRetry?: () => void;
  retryDisabled?: boolean;
  copyDisabled?: boolean;
  onUndo?: () => void;
  onAction?: (actionId: string, value?: string) => void;
  enableInternalLinks?: boolean;
  collapseMode?: MessageCollapseMode;
  onToggleCollapse?: (messageId: string, nextExpanded: boolean) => void;
  hideCodeBlocksByDefault?: boolean;
  roleCodeVisibilityKey?: string | null;
}

interface MarkdownErrorBoundaryProps {
  children: React.ReactNode;
  fallback: React.ReactNode;
  resetKey: string;
}

interface MarkdownErrorBoundaryState {
  hasError: boolean;
}

type MarkdownComponents = React.ComponentProps<
  typeof ReactMarkdown
>["components"];

interface MarkdownRendererProps {
  markdown: string;
  isUser: boolean;
  components: MarkdownComponents;
}

const MarkdownRenderer = React.memo(
  ({ markdown, isUser, components }: MarkdownRendererProps) => (
    <ReactMarkdown
      remarkPlugins={[remarkGfm, remarkMath, remarkDisableIndentedCode]}
      rehypePlugins={isUser ? [rehypeKatex] : [rehypeKatex, rehypeRaw]}
      components={components}
    >
      {markdown}
    </ReactMarkdown>
  ),
  (prevProps, nextProps) =>
    prevProps.markdown === nextProps.markdown &&
    prevProps.isUser === nextProps.isUser &&
    prevProps.components === nextProps.components,
);

MarkdownRenderer.displayName = "MarkdownRenderer";

class MarkdownErrorBoundary extends React.Component<
  MarkdownErrorBoundaryProps,
  MarkdownErrorBoundaryState
> {
  state: MarkdownErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): MarkdownErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    console.error("[chat] Markdown render failed:", error, info);
  }

  componentDidUpdate(prevProps: MarkdownErrorBoundaryProps) {
    if (this.state.hasError && prevProps.resetKey !== this.props.resetKey) {
      this.setState({ hasError: false });
    }
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback;
    }

    return this.props.children;
  }
}

const LONG_MESSAGE_CHAR_THRESHOLD = 120_000;
const LONG_MESSAGE_LINE_THRESHOLD = 2_000;
const COLLAPSE_ELIGIBLE_CHAR_THRESHOLD = 3_500;
const COLLAPSE_ELIGIBLE_LINE_THRESHOLD = 120;
const COLLAPSE_PREVIEW_CHAR_LIMIT = 1_800;
const COLLAPSE_PREVIEW_LINE_LIMIT = 60;

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
  const tokens = summary.trim().split(/\s+/u).filter(Boolean);
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

function getNodeText(node: React.ReactNode): string {
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }

  if (Array.isArray(node)) {
    return node.map((item) => getNodeText(item)).join("");
  }

  if (React.isValidElement<{ children?: React.ReactNode }>(node)) {
    return getNodeText(node.props.children);
  }

  return "";
}

function normalizeAttachmentHref(href: string): string {
  const unwrapped = unwrapMarkdownLinkDestination(href);

  try {
    return decodeURI(unwrapped);
  } catch {
    return unwrapped;
  }
}

function getTextLineCount(text: string): number {
  if (!text) return 0;

  let lineCount = 1;
  for (let i = 0; i < text.length; i += 1) {
    if (text[i] === "\n") {
      lineCount += 1;
    }
  }
  return lineCount;
}

function getPreviewCutIndex(text: string): number {
  const charCutIndex = Math.min(text.length, COLLAPSE_PREVIEW_CHAR_LIMIT);
  let lineCutIndex = text.length;
  let lineCount = 1;

  for (let i = 0; i < text.length; i += 1) {
    if (text[i] !== "\n") continue;

    lineCount += 1;
    if (lineCount > COLLAPSE_PREVIEW_LINE_LIMIT) {
      lineCutIndex = i;
      break;
    }
  }

  return Math.min(charCutIndex, lineCutIndex);
}

function isInsideUnclosedFence(text: string): boolean {
  let inFence = false;
  let fenceChar = "";
  let fenceLength = 0;

  for (const line of text.split("\n")) {
    const match = line.match(/^\s*(`{3,}|~{3,})/u);
    if (!match) continue;

    const marker = match[1];
    if (!inFence) {
      inFence = true;
      fenceChar = marker[0];
      fenceLength = marker.length;
      continue;
    }

    if (marker[0] === fenceChar && marker.length >= fenceLength) {
      inFence = false;
      fenceChar = "";
      fenceLength = 0;
    }
  }

  return inFence;
}

function buildCollapsedPreview(text: string): {
  text: string;
  isRaw: boolean;
  truncated: boolean;
} {
  const cutIndex = getPreviewCutIndex(text);
  if (cutIndex >= text.length) {
    return {
      text,
      isRaw: false,
      truncated: false,
    };
  }

  const preview = text.slice(0, cutIndex).trimEnd();
  const isRaw = isInsideUnclosedFence(preview);

  return {
    text: isRaw ? preview : `${preview}...`,
    isRaw,
    truncated: true,
  };
}

const WebsiteCitationChip: React.FC<{
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
      <CitationChip
        ref={anchorRef}
        variant="site"
        label={citation.title || domain}
        href={citation.url}
        target="_blank"
        rel="noopener noreferrer"
        fullWidth
        animate={animate}
        visual={{
          kind: "favicon",
          src: faviconUrl,
        }}
        onMouseEnter={() => setShowTip(true)}
        onMouseLeave={() => setShowTip(false)}
        onFocus={() => setShowTip(true)}
        onBlur={() => setShowTip(false)}
      />
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
  chatId = null,
  message,
  pendingTurn = null,
  onRetry,
  retryDisabled = false,
  copyDisabled = false,
  onUndo,
  onAction,
  enableInternalLinks = false,
  collapseMode = "none",
  onToggleCollapse,
  hideCodeBlocksByDefault = false,
  roleCodeVisibilityKey = null,
}) => {
  const { getAttachmentSourcePath, openMediaViewer } = useMediaContext();
  const isUser = message.role === "user";
  const isPendingAssistant = !!pendingTurn && message.role === "model";
  const [isCopied, setIsCopied] = useState(false);
  const [revealedCodeBlockKeys, setRevealedCodeBlockKeys] = useState<
    Set<string>
  >(() => new Set());
  const revealCodeBlockHandlersRef = useRef<Map<string, () => void>>(new Map());
  const previousRoleCodeVisibilityKeyRef = useRef<string | null>(
    roleCodeVisibilityKey,
  );

  useEffect(() => {
    const previousKey = previousRoleCodeVisibilityKeyRef.current;
    previousRoleCodeVisibilityKeyRef.current = roleCodeVisibilityKey;

    if (!hideCodeBlocksByDefault) {
      return;
    }
    if (previousKey === roleCodeVisibilityKey) {
      return;
    }

    setRevealedCodeBlockKeys((previous) => {
      if (previous.size === 0) {
        return previous;
      }
      return new Set();
    });
  }, [hideCodeBlocksByDefault, roleCodeVisibilityKey]);

  useEffect(() => {
    revealCodeBlockHandlersRef.current.clear();
  }, [message.id]);

  const attachments = useMemo(() => {
    const paths = parseAttachmentPaths(message.text);
    return paths.map((path) => {
      const sourcePath = getAttachmentSourcePath(path) || undefined;
      const originalName = sourcePath ? getBaseName(sourcePath) : undefined;

      return attachmentFromPath(path, undefined, originalName, sourcePath);
    });
  }, [getAttachmentSourcePath, message.text]);
  const imageAttachments = useMemo(
    () => attachments.filter((attachment) => attachment.type === "image"),
    [attachments],
  );

  const toolSteps = pendingTurn?.toolSteps || message.toolSteps || [];
  const citations = pendingTurn?.visibleCitations || message.citations || [];
  const displayText = useMemo(() => {
    if (isPendingAssistant) {
      return pendingTurn?.displayText || "";
    }
    return stripImageAttachmentMentions(message.text);
  }, [isPendingAssistant, message.text, pendingTurn]);

  const thoughtBadgeText = useMemo(() => {
    if (isUser || message.role !== "model") return null;

    const explicitThoughtSeconds =
      pendingTurn?.thoughtSeconds ?? message.thoughtSeconds;
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
  }, [
    isUser,
    message.role,
    message.thoughtSeconds,
    pendingTurn?.thoughtSeconds,
    toolSteps,
  ]);

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
  const isWritingCode = isPendingStreaming && !!pendingTurn?.isWritingCode;
  const hasImageAttachments = imageAttachments.length > 0;
  const hasPendingShellContent = isPendingAssistant && !hasDisplayText;
  const isImageOnlyEmptyCaption =
    hasImageAttachments &&
    !hasDisplayText &&
    !showCitations &&
    !hasPendingShellContent;
  const shouldRenderBubble = !isImageOnlyEmptyCaption;
  const shouldDoubleNewlines = isUser && !isRichMarkdownUserMessage;
  const displayTextLineCount = useMemo(
    () => getTextLineCount(displayText),
    [displayText],
  );
  const isCollapseEligible =
    displayText.length > COLLAPSE_ELIGIBLE_CHAR_THRESHOLD ||
    displayTextLineCount > COLLAPSE_ELIGIBLE_LINE_THRESHOLD;
  const effectiveCollapseMode: MessageCollapseMode =
    !isPendingAssistant && message.role !== "system" && isCollapseEligible
      ? collapseMode
      : "none";
  const isCollapsed = effectiveCollapseMode === "collapsed";
  const shouldShowCollapseToggle =
    effectiveCollapseMode === "collapsed" ||
    effectiveCollapseMode === "expanded";
  const collapsedPreview = useMemo(
    () => (isCollapsed ? buildCollapsedPreview(displayText) : null),
    [displayText, isCollapsed],
  );
  const renderedText =
    isCollapsed && collapsedPreview?.truncated
      ? collapsedPreview.text
      : displayText;
  const shouldRenderCollapsedRawPreview =
    isCollapsed && !!collapsedPreview?.truncated && collapsedPreview.isRaw;
  const isVeryLongMessage = useMemo(() => {
    if (!displayText) return false;
    if (displayText.length >= LONG_MESSAGE_CHAR_THRESHOLD) {
      return true;
    }

    let lineCount = 1;
    for (let i = 0; i < displayText.length; i += 1) {
      if (displayText[i] === "\n") {
        lineCount += 1;
        if (lineCount >= LONG_MESSAGE_LINE_THRESHOLD) {
          return true;
        }
      }
    }

    return false;
  }, [displayText]);
  const shouldRenderPlainTextMessage =
    !isCollapsed && isUser && isVeryLongMessage;
  const markdownRenderText = useMemo(
    () =>
      preprocessMarkdown(normalizeAttachmentMarkdownLinks(renderedText), {
        doubleNewlines: shouldDoubleNewlines && !shouldRenderPlainTextMessage,
      }),
    [renderedText, shouldDoubleNewlines, shouldRenderPlainTextMessage],
  );
  const streamingMarkdownSegments = useMemo(() => {
    if (!isPendingStreaming || !markdownRenderText) {
      return {
        stable: markdownRenderText,
        tail: "",
      };
    }

    return splitMarkdownAfterLastClosedFence(markdownRenderText);
  }, [isPendingStreaming, markdownRenderText]);
  const shouldSplitStreamingMarkdown =
    streamingMarkdownSegments.stable.length > 0 &&
    streamingMarkdownSegments.tail.length > 0;
  const markdownBoundaryFallback = useMemo(
    () => <pre className={mdStyles.fallbackPlainText}>{renderedText}</pre>,
    [renderedText],
  );
  const markdownBoundaryKey = `${message.id}-${message.role}-${renderedText.length}-${effectiveCollapseMode}`;
  const canCopy = !copyDisabled && hasDisplayText;
  const shouldShowRetryButton =
    !isUser && message.role !== "system" && !!onRetry;
  const shouldShowCopyButton = message.role !== "system";
  const isPendingEmpty = isPendingAssistant && !hasDisplayText;
  const animateCitations = isPendingAssistant;
  const handleToggleCollapse = useCallback(() => {
    if (!onToggleCollapse || !shouldShowCollapseToggle) return;
    onToggleCollapse(message.id, effectiveCollapseMode === "collapsed");
  }, [
    effectiveCollapseMode,
    message.id,
    onToggleCollapse,
    shouldShowCollapseToggle,
  ]);
  const revealCodeBlock = useCallback((blockKey: string) => {
    setRevealedCodeBlockKeys((previous) => {
      if (previous.has(blockKey)) {
        return previous;
      }

      const next = new Set(previous);
      next.add(blockKey);
      return next;
    });
  }, []);
  const getRevealCodeBlockHandler = useCallback(
    (blockKey: string) => {
      const existingHandler = revealCodeBlockHandlersRef.current.get(blockKey);
      if (existingHandler) {
        return existingHandler;
      }

      const nextHandler = () => revealCodeBlock(blockKey);
      revealCodeBlockHandlersRef.current.set(blockKey, nextHandler);
      return nextHandler;
    },
    [revealCodeBlock],
  );

  const handleImageClick = useCallback(
    (attachment: (typeof imageAttachments)[number], index: number) => {
      void openMediaViewer(attachment, {
        isGallery: true,
        chatId: chatId || undefined,
        galleryAttachments: imageAttachments,
        initialIndex: index,
        openedFromChat: true,
      });
    },
    [chatId, imageAttachments, openMediaViewer],
  );

  const handleLocalAttachmentLink = useCallback(
    (href: string, children: React.ReactNode) => {
      const normalizedHref = normalizeAttachmentHref(href);
      const sourcePath =
        getAttachmentSourcePath(normalizedHref) ||
        getAttachmentSourcePath(href) ||
        undefined;
      const attachment = attachmentFromPath(
        normalizedHref,
        undefined,
        undefined,
        sourcePath,
      );
      const label = getNodeText(children).trim();
      const originalName =
        label || (sourcePath ? getBaseName(sourcePath) : attachment.name);

      void openMediaViewer({
        ...attachment,
        name: originalName,
      });
    },
    [getAttachmentSourcePath, openMediaViewer],
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
      pre({ children, ...props }: any) {
        const childNodes = React.Children.toArray(children);

        if (childNodes.length === 1 && React.isValidElement(childNodes[0])) {
          return <>{childNodes[0]}</>;
        }

        return <pre {...props}>{children}</pre>;
      },

      code({ node, className, children, ...props }: any) {
        const codeText = String(children).replace(/\n$/u, "");
        const match = /language-(\w+)/u.exec(className || "");
        const isInline = !match && !String(children).includes("\n");

        if (!isInline) {
          const blockKey =
            typeof node?.position?.start?.offset === "number"
              ? `offset:${node.position.start.offset}`
              : `${className || "plain"}:${codeText.length}:${codeText.slice(0, 32)}`;
          const shouldHideCodeBlock =
            hideCodeBlocksByDefault &&
            !isPendingAssistant &&
            !revealedCodeBlockKeys.has(blockKey);
          const hiddenLineCount = Math.max(1, getTextLineCount(codeText));

          return (
            <CodeBlock
              key={blockKey}
              language={match ? match[1] : ""}
              value={codeText}
              hideCodeContent={shouldHideCodeBlock}
              hiddenCodeLineCount={hiddenLineCount}
              onRevealCodeContent={getRevealCodeBlockHandler(blockKey)}
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
        const isLocalAttachmentLink =
          typeof href === "string" && isAttachmentPath(href);

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

        if (isLocalAttachmentLink && typeof href === "string") {
          const normalizedHref = normalizeAttachmentHref(href);
          const sourcePath =
            getAttachmentSourcePath(normalizedHref) ||
            getAttachmentSourcePath(href) ||
            undefined;
          const attachment = attachmentFromPath(
            normalizedHref,
            undefined,
            undefined,
            sourcePath,
          );
          const fileName = sourcePath
            ? getBaseName(sourcePath)
            : attachment.name;
          const label = getNodeText(children).trim() || fileName;

          return (
            <CitationChip
              variant="file"
              href={normalizedHref}
              label={label}
              visual={{
                kind: "file",
                fileName,
              }}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                handleLocalAttachmentLink(href, children);
              }}
            />
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
    [
      enableInternalLinks,
      getRevealCodeBlockHandler,
      getAttachmentSourcePath,
      handleLocalAttachmentLink,
      hideCodeBlocksByDefault,
      isPendingAssistant,
      onAction,
      revealedCodeBlockKeys,
    ],
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
                  isUser
                    ? styles.imageCollageUserFrame
                    : styles.imageCollageBotFrame
                } ${shouldRenderBubble ? styles.imageCollageWithBubble : ""}`}
              />
            )}

            {shouldRenderBubble && (
              <div
                dir="auto"
                className={`${styles.bubble} ${
                  isUser ? styles.userBubble : styles.botBubble
                } ${
                  isUser && isRichMarkdownUserMessage
                    ? styles.userRichBubble
                    : ""
                } ${isPendingAssistant ? styles.pendingBubbleShell : ""} ${
                  isPendingEmpty ? styles.pendingBubbleEmpty : ""
                } ${
                  pendingTurn?.phase === "streaming" ||
                  pendingTurn?.phase === "finalizing"
                    ? styles.livePendingBubble
                    : ""
                }`}
              >
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
                    {shouldRenderCollapsedRawPreview ? (
                      <pre className={mdStyles.fallbackPlainText}>
                        {renderedText}
                      </pre>
                    ) : shouldRenderPlainTextMessage ? (
                      markdownBoundaryFallback
                    ) : (
                      <MarkdownErrorBoundary
                        resetKey={markdownBoundaryKey}
                        fallback={markdownBoundaryFallback}
                      >
                        {shouldSplitStreamingMarkdown ? (
                          <>
                            <MarkdownRenderer
                              markdown={streamingMarkdownSegments.stable}
                              isUser={isUser}
                              components={markdownComponents}
                            />
                            <MarkdownRenderer
                              markdown={streamingMarkdownSegments.tail}
                              isUser={isUser}
                              components={markdownComponents}
                            />
                          </>
                        ) : (
                          <MarkdownRenderer
                            markdown={markdownRenderText}
                            isUser={isUser}
                            components={markdownComponents}
                          />
                        )}
                      </MarkdownErrorBoundary>
                    )}
                  </div>
                </div>

                {isWritingCode && (
                  <TextShimmer
                    text={API_STATUS_TEXT.WRITING_CODE}
                    compact={true}
                    duration={2}
                    spotWidth={30}
                    angle={90}
                    peakWidth={3}
                    bleedInner={8}
                    bleedOuter={30}
                    className={mdStyles.writingCodeShimmer}
                  />
                )}

                {showCitations && (
                  <div
                    className={`${styles.citationFooter} ${
                      animateCitations ? styles.citationFooterReveal : ""
                    }`}
                  >
                    {citations.map((citation, index) => (
                      <WebsiteCitationChip
                        key={`${citation.url}-${index}`}
                        citation={citation}
                        animate={animateCitations}
                      />
                    ))}
                  </div>
                )}

                {shouldShowCollapseToggle && (
                  <div className={styles.inlineCollapseFooter}>
                    <button
                      type="button"
                      className={styles.inlineCollapseButton}
                      onClick={handleToggleCollapse}
                    >
                      {effectiveCollapseMode === "collapsed"
                        ? "Show more"
                        : "Show less"}
                    </button>
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
      prevProps.enableInternalLinks === nextProps.enableInternalLinks &&
      prevProps.collapseMode === nextProps.collapseMode &&
      prevProps.onToggleCollapse === nextProps.onToggleCollapse &&
      prevProps.hideCodeBlocksByDefault === nextProps.hideCodeBlocksByDefault &&
      prevProps.roleCodeVisibilityKey === nextProps.roleCodeVisibilityKey
    );
  },
);
