/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import "prosemirror-view/style/prosemirror.css";

import React, {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
} from "react";
import {
  EditorContent,
  NodeViewWrapper,
  ReactNodeViewRenderer,
  useEditor,
  type Editor as TiptapEditor,
  type NodeViewProps,
} from "@tiptap/react";
import {
  Extension,
  Node,
  mergeAttributes,
  nodeInputRule,
  type InputRuleMatch,
  type JSONContent,
} from "@tiptap/core";
import type { Fragment, Node as ProseMirrorNode } from "prosemirror-model";
import { baseKeymap, splitBlock } from "prosemirror-commands";
import {
  history,
  redo as redoHistory,
  undo as undoHistory,
} from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { Plugin, PluginKey, type Command } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import { invoke } from "@tauri-apps/api/core";
import { CollapseTextareaIcon, ExpandTextareaIcon } from "@/components/icons";
import { CitationChip } from "@/components/ui";
import {
  attachmentFromPath,
  buildAttachmentMention,
  isAttachmentPath,
  unwrapMarkdownLinkDestination,
} from "@squigit/core/brain/session/attachments";
import { useMediaContext } from "@/app/context/AppMedia";
import styles from "./ChatInput.module.css";

const ATTACHMENT_LINK_RE = /\[([^\]\n]+)\]\((<[^>\n]+>|[^)\n]+)\)/g;
const ATTACHMENT_INPUT_RE = /\[([^\]\n]+)\]\((<[^>\n]+>|[^)\n]+)\)$/u;
const LINE_HEIGHT = 24;
const INPUT_VERTICAL_PADDING = 8;
const LOCAL_ECHO_BUFFER_LIMIT = 64;
const EMPTY_DOC_JSON: JSONContent = {
  type: "doc",
  content: [{ type: "paragraph" }],
};

interface AttachmentMentionAttrs {
  label: string;
  path: string;
}

interface InputTextareaProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  disabled: boolean;
  placeholder: string;
  onSelectionChange: (hasSelection: boolean) => void;
  onContextMenu: (e: React.MouseEvent<HTMLElement>) => void;
  onImagePasted: (path: string) => void;
  isExpanded: boolean;
  setIsExpanded: React.Dispatch<React.SetStateAction<boolean>>;
}

export interface ChatInputEditorHandle {
  focus: () => void;
  blur: () => void;
  resetScroll: () => void;
  insertRawText: (value: string) => void;
  appendRawText: (value: string) => void;
  copySelection: () => Promise<void>;
  cutSelection: () => Promise<void>;
  pasteFromClipboard: () => Promise<void>;
  selectAll: () => void;
  undo: () => void;
  redo: () => void;
  hasSelection: () => boolean;
}

function getBaseName(path: string): string {
  const lastSlash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return lastSlash >= 0 ? path.slice(lastSlash + 1) : path;
}

function toAttachmentMentionAttrs(
  label: string,
  rawDestination: string,
): AttachmentMentionAttrs | null {
  const nextLabel = label.trim();
  const nextPath = unwrapMarkdownLinkDestination(rawDestination);

  if (!nextLabel || !isAttachmentPath(nextPath)) {
    return null;
  }

  return {
    label: nextLabel,
    path: nextPath,
  };
}

function findAttachmentInputMatch(text: string): InputRuleMatch | null {
  const match = text.match(ATTACHMENT_INPUT_RE);

  if (!match || typeof match.index !== "number") {
    return null;
  }

  const attrs = toAttachmentMentionAttrs(match[1] || "", match[2] || "");
  if (!attrs) {
    return null;
  }

  return {
    index: match.index,
    text: match[0],
    replaceWith: match[0],
    data: attrs,
  };
}

function appendText(content: JSONContent[], text: string) {
  if (text.length === 0) {
    return;
  }

  content.push({
    type: "text",
    text,
  });
}

function rawTextToInlineContent(text: string): JSONContent[] {
  const content: JSONContent[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(ATTACHMENT_LINK_RE)) {
    const start = match.index ?? 0;

    if (start > lastIndex) {
      appendText(content, text.slice(lastIndex, start));
    }

    const attrs = toAttachmentMentionAttrs(match[1] || "", match[2] || "");

    if (attrs) {
      content.push({
        type: "attachmentMention",
        attrs,
      });
    } else {
      appendText(content, match[0]);
    }

    lastIndex = start + match[0].length;
  }

  if (lastIndex < text.length) {
    appendText(content, text.slice(lastIndex));
  }

  return content;
}

function rawTextToParagraphContent(text: string): JSONContent[] {
  return text.split("\n").map((line) => {
    const content = rawTextToInlineContent(line);
    if (content.length === 0) {
      return {
        type: "paragraph",
      };
    }

    return {
      type: "paragraph",
      content,
    };
  });
}

function rawTextToDocument(text: string): JSONContent {
  if (!text) {
    return EMPTY_DOC_JSON;
  }

  return {
    type: "doc",
    content: rawTextToParagraphContent(text),
  };
}

function serializeLeaf(node: ProseMirrorNode): string {
  if (node.type.name === "attachmentMention") {
    return buildAttachmentMention(
      String(node.attrs.path || ""),
      String(node.attrs.label || ""),
    );
  }

  if (node.type.name === "hardBreak") {
    return "\n";
  }

  return "";
}

function serializeComposerFragment(fragment: Fragment): string {
  return fragment.textBetween(0, fragment.size, "\n", serializeLeaf);
}

function serializeComposerDocument(doc: ProseMirrorNode): string {
  return serializeComposerFragment(doc.content);
}

function pushPendingLocalEcho(
  queueRef: React.MutableRefObject<string[]>,
  value: string,
) {
  if (queueRef.current[queueRef.current.length - 1] === value) {
    return;
  }

  queueRef.current.push(value);

  if (queueRef.current.length > LOCAL_ECHO_BUFFER_LIMIT) {
    queueRef.current.splice(
      0,
      queueRef.current.length - LOCAL_ECHO_BUFFER_LIMIT,
    );
  }
}

function acknowledgePendingLocalEcho(
  queueRef: React.MutableRefObject<string[]>,
  value: string,
): boolean {
  const index = queueRef.current.lastIndexOf(value);

  if (index === -1) {
    return false;
  }

  queueRef.current.splice(0, index + 1);
  return true;
}

function deleteAdjacentAttachmentMention(
  editor: TiptapEditor,
  direction: "backward" | "forward",
): boolean {
  const { selection } = editor.state;

  if (!selection.empty) {
    return false;
  }

  const { $from } = selection;
  const targetNode =
    direction === "backward" ? $from.nodeBefore : $from.nodeAfter;

  if (!targetNode || targetNode.type.name !== "attachmentMention") {
    return false;
  }

  const from =
    direction === "backward" ? $from.pos - targetNode.nodeSize : $from.pos;
  const to = from + targetNode.nodeSize;

  editor.view.dispatch(editor.state.tr.delete(from, to));
  return true;
}

function createParagraphBreakCommand(): Command {
  return (state, dispatch, view) => {
    if (view?.composing) {
      return false;
    }

    return splitBlock(state, dispatch);
  };
}

function createSubmitCommand(
  editorRef: React.MutableRefObject<TiptapEditor | null>,
  onSubmitRef: React.MutableRefObject<(() => void) | null>,
): Command {
  return (_state, _dispatch, view) => {
    if (view?.composing) {
      return false;
    }

    if (!editorRef.current) {
      return false;
    }

    onSubmitRef.current?.();
    return true;
  };
}

const ComposerDocument = Node.create({
  name: "doc",
  topNode: true,
  content: "paragraph+",
});

const ComposerParagraph = Node.create({
  name: "paragraph",
  group: "block",
  content: "inline*",

  parseHTML() {
    return [{ tag: "p" }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["p", mergeAttributes(HTMLAttributes), 0];
  },
});

const ComposerText = Node.create({
  name: "text",
  group: "inline",
});

const ComposerHardBreak = Node.create({
  name: "hardBreak",
  group: "inline",
  inline: true,
  selectable: false,

  parseHTML() {
    return [{ tag: "br" }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["br", mergeAttributes(HTMLAttributes)];
  },
});

const ComposerCore = Extension.create<{
  editorRef: React.MutableRefObject<TiptapEditor | null>;
  onSubmitRef: React.MutableRefObject<(() => void) | null>;
}>({
  name: "composerCore",

  addProseMirrorPlugins() {
    return [
      history(),
      keymap({
        Enter: createSubmitCommand(
          this.options.editorRef,
          this.options.onSubmitRef,
        ),
        "Shift-Enter": createParagraphBreakCommand(),
        "Mod-z": (_state, dispatch) =>
          undoHistory(this.editor.state, dispatch ?? this.editor.view.dispatch),
        "Mod-Shift-z": (_state, dispatch) =>
          redoHistory(this.editor.state, dispatch ?? this.editor.view.dispatch),
        "Mod-y": (_state, dispatch) =>
          redoHistory(this.editor.state, dispatch ?? this.editor.view.dispatch),
        "Mod-a": () => this.editor.commands.selectAll(),
      }),
      keymap(baseKeymap),
    ];
  },
});

const Placeholder = Extension.create<{
  placeholder: string;
}>({
  name: "composerPlaceholder",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("chatInputPlaceholder"),
        props: {
          decorations: (state) => {
            const { doc } = state;
            const firstChild = doc.firstChild;

            if (
              !firstChild ||
              doc.childCount !== 1 ||
              firstChild.type.name !== "paragraph" ||
              firstChild.content.size > 0
            ) {
              return null;
            }

            return DecorationSet.create(doc, [
              Decoration.node(0, firstChild.nodeSize, {
                class: styles.emptyParagraph,
                "data-placeholder": this.options.placeholder,
              }),
            ]);
          },
        },
      }),
    ];
  },
});

const AttachmentMentionChip: React.FC<NodeViewProps> = ({ node }) => {
  const { getAttachmentSourcePath, openMediaViewer } = useMediaContext();
  const path = String(node.attrs.path || "");
  const label = String(node.attrs.label || "").trim();
  const sourcePath = getAttachmentSourcePath(path) || undefined;
  const attachment = attachmentFromPath(path, undefined, undefined, sourcePath);
  const fileName = sourcePath ? getBaseName(sourcePath) : attachment.name;
  const chipLabel = label || fileName;

  const openAttachment = useCallback(() => {
    void openMediaViewer({
      ...attachment,
      name: chipLabel,
    });
  }, [attachment, chipLabel, openMediaViewer]);

  return (
    <NodeViewWrapper
      as="span"
      className={styles.attachmentMentionNode}
      contentEditable={false}
      data-path={path}
    >
      <CitationChip
        variant="file"
        href={path}
        visual={{
          kind: "file",
          fileName,
        }}
        label={chipLabel}
        tabIndex={-1}
        draggable={false}
        title={sourcePath || path}
        onMouseDown={(event) => {
          if (event.button !== 0) {
            return;
          }

          event.preventDefault();
        }}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          openAttachment();
        }}
      />
    </NodeViewWrapper>
  );
};

const AttachmentMention = Node.create({
  name: "attachmentMention",
  group: "inline",
  inline: true,
  atom: true,
  selectable: false,
  draggable: false,

  addAttributes() {
    return {
      label: {
        default: "",
      },
      path: {
        default: "",
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: "span[data-attachment-mention]",
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-attachment-mention": "true",
      }),
    ];
  },

  addNodeView() {
    return ReactNodeViewRenderer(AttachmentMentionChip);
  },

  addInputRules() {
    return [
      nodeInputRule({
        find: findAttachmentInputMatch,
        type: this.type,
        getAttributes: (match) =>
          (match.data as AttachmentMentionAttrs | undefined) || {},
      }),
    ];
  },

  addKeyboardShortcuts() {
    return {
      Backspace: () => deleteAdjacentAttachmentMention(this.editor, "backward"),
      Delete: () => deleteAdjacentAttachmentMention(this.editor, "forward"),
    };
  },
});

async function readClipboardImagePath(): Promise<string | null> {
  try {
    const result = await invoke<{ hash: string; path: string }>(
      "read_clipboard_image",
    );
    return result?.path || null;
  } catch {
    return null;
  }
}

async function readClipboardTextValue(): Promise<string> {
  try {
    return await invoke<string>("read_clipboard_text");
  } catch {
    try {
      return await navigator.clipboard.readText();
    } catch (err) {
      console.error("Failed to read clipboard text:", err);
      return "";
    }
  }
}

export const InputTextarea = forwardRef<
  ChatInputEditorHandle,
  InputTextareaProps
>(
  (
    {
      value,
      onChange,
      onSubmit,
      disabled,
      placeholder,
      onSelectionChange,
      onContextMenu,
      onImagePasted,
      isExpanded,
      setIsExpanded,
    },
    ref,
  ) => {
    const editorRef = useRef<TiptapEditor | null>(null);
    const onChangeRef = useRef(onChange);
    const onSubmitRef = useRef(onSubmit);
    const onSelectionChangeRef = useRef(onSelectionChange);
    const onImagePastedRef = useRef(onImagePasted);
    const pendingLocalValuesRef = useRef<string[]>([]);
    const maxHeight = `${LINE_HEIGHT * (isExpanded ? 15 : 10) + INPUT_VERTICAL_PADDING}px`;
    const showExpandButton =
      isExpanded || value.includes("\n") || value.length > 220;

    useEffect(() => {
      onChangeRef.current = onChange;
    }, [onChange]);

    useEffect(() => {
      onSubmitRef.current = onSubmit;
    }, [onSubmit]);

    useEffect(() => {
      onSelectionChangeRef.current = onSelectionChange;
    }, [onSelectionChange]);

    useEffect(() => {
      onImagePastedRef.current = onImagePasted;
    }, [onImagePasted]);

    const insertRawText = useCallback(
      (nextValue: string, mode: "selection" | "end" = "selection") => {
        const editor = editorRef.current;
        if (!editor || !nextValue) {
          return;
        }

        const content = nextValue.includes("\n")
          ? rawTextToParagraphContent(nextValue)
          : rawTextToInlineContent(nextValue);
        const chain = editor.chain();

        if (mode === "end") {
          chain.focus("end", { scrollIntoView: false });
        } else {
          chain.focus(undefined, { scrollIntoView: false });
        }

        chain.insertContent(content).run();
      },
      [],
    );

    const getSelectedMarkdown = useCallback(() => {
      const editor = editorRef.current;
      if (!editor || editor.state.selection.empty) {
        return "";
      }

      return serializeComposerFragment(
        editor.state.selection.content().content,
      );
    }, []);

    const copySelection = useCallback(async () => {
      const selectedText = getSelectedMarkdown();
      if (!selectedText) {
        return;
      }

      await navigator.clipboard.writeText(selectedText);
    }, [getSelectedMarkdown]);

    const cutSelection = useCallback(async () => {
      const editor = editorRef.current;
      const selectedText = getSelectedMarkdown();

      if (!editor || !selectedText) {
        return;
      }

      await navigator.clipboard.writeText(selectedText);
      editor
        .chain()
        .focus(undefined, { scrollIntoView: false })
        .deleteSelection()
        .run();
    }, [getSelectedMarkdown]);

    const pasteFromClipboard = useCallback(async () => {
      const imagePath = await readClipboardImagePath();
      if (imagePath) {
        onImagePastedRef.current(imagePath);
        return;
      }

      const text = await readClipboardTextValue();
      if (!text) {
        return;
      }

      insertRawText(text);
    }, [insertRawText]);

    const editor = useEditor({
      immediatelyRender: false,
      editable: !disabled,
      content: rawTextToDocument(value),
      extensions: [
        ComposerDocument,
        ComposerParagraph,
        ComposerText,
        ComposerHardBreak,
        ComposerCore.configure({
          editorRef,
          onSubmitRef,
        }),
        Placeholder.configure({
          placeholder,
        }),
        AttachmentMention,
      ],
      editorProps: {
        attributes: {
          class: styles.richTextEditor,
          spellcheck: "false",
          autocorrect: "off",
          autocapitalize: "off",
          autocomplete: "off",
          "data-gramm": "false",
          "data-gramm_editor": "false",
          "data-enable-grammarly": "false",
        },
        clipboardTextSerializer: (slice) =>
          serializeComposerFragment(slice.content),
        handlePaste: (_view, pasteEvent) => {
          pasteEvent.preventDefault();
          const text = pasteEvent.clipboardData?.getData("text/plain") || "";

          void (async () => {
            const imagePath = await readClipboardImagePath();
            if (imagePath) {
              onImagePastedRef.current(imagePath);
              return;
            }

            if (text) {
              insertRawText(text);
            }
          })();

          return true;
        },
        handleScrollToSelection: (view) => {
          const root = view.dom as HTMLElement | null;
          if (!root) {
            return false;
          }

          const coords = view.coordsAtPos(view.state.selection.head, 1);
          const bounds = root.getBoundingClientRect();
          const margin = 8;

          if (coords.bottom > bounds.bottom - margin) {
            root.scrollTop += coords.bottom - bounds.bottom + margin;
          } else if (coords.top < bounds.top + margin) {
            root.scrollTop -= bounds.top + margin - coords.top;
          }

          // Prevent ProseMirror from scrolling outer ancestors/window.
          return true;
        },
      },
      onCreate: ({ editor: nextEditor }) => {
        editorRef.current = nextEditor;
        onSelectionChangeRef.current(!nextEditor.state.selection.empty);
      },
      onUpdate: ({ editor: nextEditor }) => {
        editorRef.current = nextEditor;
        const serializedValue = serializeComposerDocument(nextEditor.state.doc);

        pushPendingLocalEcho(pendingLocalValuesRef, serializedValue);
        onChangeRef.current(serializedValue);
      },
      onSelectionUpdate: ({ editor: nextEditor }) => {
        onSelectionChangeRef.current(!nextEditor.state.selection.empty);
      },
      onBlur: () => {
        onSelectionChangeRef.current(false);
      },
    });

    useEffect(() => {
      editorRef.current = editor;
    }, [editor]);

    useEffect(() => {
      if (!editor) {
        return;
      }

      editor.setEditable(!disabled);
    }, [disabled, editor]);

    useEffect(() => {
      if (!editor) {
        return;
      }

      const currentValue = serializeComposerDocument(editor.state.doc);
      if (value === currentValue) {
        acknowledgePendingLocalEcho(pendingLocalValuesRef, value);
        return;
      }

      if (acknowledgePendingLocalEcho(pendingLocalValuesRef, value)) {
        return;
      }

      editor.commands.setContent(rawTextToDocument(value), {
        emitUpdate: false,
      });

      if (!value) {
        const root = editor.view.dom as HTMLElement | null;
        if (root) {
          root.scrollTop = 0;
        }
      }

      onSelectionChangeRef.current(!editor.state.selection.empty);
    }, [editor, value]);

    useImperativeHandle(
      ref,
      () => ({
        focus: () => {
          editorRef.current?.commands.focus(undefined, {
            scrollIntoView: false,
          });
        },
        blur: () => {
          const root = editorRef.current?.view.dom as HTMLElement | null;
          root?.blur();
        },
        resetScroll: () => {
          const root = editorRef.current?.view.dom as HTMLElement | null;
          if (root) {
            root.scrollTop = 0;
          }
        },
        insertRawText: (nextValue: string) => {
          insertRawText(nextValue);
        },
        appendRawText: (nextValue: string) => {
          insertRawText(nextValue, "end");
        },
        copySelection,
        cutSelection,
        pasteFromClipboard,
        selectAll: () => {
          editorRef.current?.commands.selectAll();
        },
        undo: () => {
          const nextEditor = editorRef.current;
          if (!nextEditor) {
            return;
          }

          undoHistory(nextEditor.state, nextEditor.view.dispatch);
        },
        redo: () => {
          const nextEditor = editorRef.current;
          if (!nextEditor) {
            return;
          }

          redoHistory(nextEditor.state, nextEditor.view.dispatch);
        },
        hasSelection: () =>
          Boolean(
            editorRef.current && !editorRef.current.state.selection.empty,
          ),
      }),
      [copySelection, cutSelection, insertRawText, pasteFromClipboard],
    );

    return (
      <>
        <div className={styles.topRow}>
          {showExpandButton && (
            <button
              className={styles.expandButton}
              onClick={() => setIsExpanded(!isExpanded)}
              type="button"
            >
              {isExpanded ? (
                <CollapseTextareaIcon size={14} />
              ) : (
                <ExpandTextareaIcon size={14} />
              )}
            </button>
          )}
        </div>

        <div className={styles.inputArea}>
          <div
            className={styles.richTextSurface}
            style={
              {
                "--chat-input-max-height": maxHeight,
              } as React.CSSProperties
            }
          >
            <EditorContent editor={editor} onContextMenu={onContextMenu} />
          </div>
        </div>
      </>
    );
  },
);

InputTextarea.displayName = "InputTextarea";
