/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

export type { DialogVariant, DialogContent } from "./dialogs.ts";
export {
  getDialogs,
  getDeleteMultipleChatsDialog,
  getAppBusyDialog,
  getErrorDialog,
  getMissingPackageDialog,
  getOutdatedPackageDialog,
  getUpdateAvailableDialog,
} from "./dialogs.ts";

export type { AttachmentAnalysisCounts } from "./api-status.ts";
export {
  ATTACHMENT_ANALYSIS_STATUS_DELAY_MS,
  HIGH_DEMAND_RETRY_DELAYS_MS,
  HIGH_DEMAND_RETRY_ATTEMPTS,
  API_STATUS_TEXT,
  getAttachmentAnalysisStatusText,
  getHighDemandRetryStatusText,
  isQuickAnswerSuppressedProgressText,
  mapToolStatusText,
  getProgressStatusText,
} from "./api-status.ts";

export type { ParsedError } from "./error-parser.ts";
export { parseAppError } from "./error-parser.ts";

export type { ReportAction } from "./reporting.ts";
export {
  buildMailto,
  buildGithubNewIssueUrl,
  prepareMailReport,
  prepareGitHubIssueReport,
} from "./reporting.ts";
