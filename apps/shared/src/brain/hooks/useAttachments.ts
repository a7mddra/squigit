/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState, useCallback } from "react";
import type { Attachment } from "../session/attachments";
import { attachmentFromPath } from "../session/attachments";
import { getSystemPort } from "../../ports/system";

function filterImageAttachments(attachments: Attachment[]): Attachment[] {
  return attachments.filter((attachment) => attachment.type === "image");
}

export function useAttachments() {
  const [attachments, setAttachmentsState] = useState<Attachment[]>([]);

  const setAttachments = useCallback((nextAttachments: Attachment[]) => {
    setAttachmentsState(filterImageAttachments(nextAttachments));
  }, []);

  const addAttachments = useCallback((newOnes: Attachment[]) => {
    setAttachmentsState((prev) => [
      ...prev,
      ...filterImageAttachments(newOnes),
    ]);
  }, []);

  const removeAttachment = useCallback((id: string) => {
    setAttachmentsState((prev) => {
      const target = prev.find((a) => a.id === id);
      if (target?.isTemp) {
        getSystemPort().deleteTempFile(target.path).catch(() => {});
      }
      return prev.filter((a) => a.id !== id);
    });
  }, []);

  const clearAttachments = useCallback(() => {
    setAttachmentsState((prev) => {
      prev
        .filter((a) => a.isTemp)
        .forEach((a) => {
          getSystemPort().deleteTempFile(a.path).catch(() => {});
        });
      return [];
    });
  }, []);

  const addFromPath = useCallback(
    (path: string, id?: string) => {
      addAttachments([attachmentFromPath(path, id)]);
    },
    [addAttachments],
  );

  return {
    attachments,
    setAttachments,
    addAttachments,
    addFromPath,
    removeAttachment,
    clearAttachments,
  };
}
