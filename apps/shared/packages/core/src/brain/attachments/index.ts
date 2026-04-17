/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type { Attachment } from "./types.ts";
export {
  IMAGE_EXTENSION_VALUES,
  ACCEPTED_EXTENSION_VALUES,
  IMAGE_EXTENSIONS,
  ACCEPTED_EXTENSIONS,
  isImageExtension,
  isAcceptedExtension,
  getExtension,
} from "./extensions.ts";

export {
  ABSOLUTE_CAS_PATH_RE,
  getBaseName,
  unwrapMarkdownLinkDestination,
  isAttachmentPath,
  isAbsoluteCasPath,
} from "./paths.ts";

export {
  LEGACY_ATTACHMENT_MENTION_RE,
  LINK_ATTACHMENT_MENTION_RE,
  formatAttachmentLinkDestination,
  isAbsoluteCasAttachmentMarkdownLink,
  normalizeAttachmentMarkdownLinks,
  parseAttachmentPaths,
  stripAttachmentMentions,
  stripImageAttachmentMentions,
  buildAttachmentMention,
} from "./markdown.ts";

export { attachmentFromPath } from "./factory.ts";
export { normalizeMessageForHistory } from "./memory.ts";
