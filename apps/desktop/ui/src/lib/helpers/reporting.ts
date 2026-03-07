/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { github } from "@/lib";

const MAX_URL_LENGTH = 1900;

const enc = encodeURIComponent;

export function buildMailto(email: string, subject = "", body = ""): string {
  const params: string[] = [];
  if (subject) params.push(`subject=${enc(subject)}`);
  if (body) params.push(`body=${enc(body)}`);
  return `mailto:${email}${params.length ? "?" + params.join("&") : ""}`;
}

export function buildGithubNewIssueUrl({
  base = github.issues.new("bug_report.md"),
  title = "",
  body = "",
  template,
}: {
  base?: string;
  title?: string;
  body?: string;
  template?: string;
}) {
  const params = new URLSearchParams();
  if (title) params.set("title", title);
  if (body) params.set("body", body);
  if (template) params.set("template", template);
  return `${base}?${params.toString()}`;
}

export type ReportAction = {
  openUrl: string;
  didCopy: boolean;
  copyText?: string;
  message?: string;
};

export function prepareMailReport(
  appName: string,
  {
    email = "a7mddra@gmail.com",
    subject = `This is a bug report from ${appName}`,
    diagnostics = "",
  }: {
    email?: string;
    subject?: string;
    diagnostics?: string;
  },
): ReportAction {
  const body = [
    "Please describe the bug below and provide the system diagnostics. Thank you!",
    "",
    "---- SYSTEM DIAGNOSTICS ----",
    diagnostics,
  ].join("\n");

  const url = buildMailto(email, subject, body);

  if (url.length < MAX_URL_LENGTH) {
    return { openUrl: url, didCopy: false };
  } else {
    const shortBody = [
      "Please describe the bug below. I have copied the system diagnostics to my clipboard — paste them where requested.",
      "",
      "---- paste diagnostics here ----",
    ].join("\n");

    const fallbackUrl = buildMailto(email, subject, shortBody);
    return {
      openUrl: fallbackUrl,
      didCopy: true,
      copyText: diagnostics,
      message:
        "Diagnostics are long — they were copied to your clipboard. Paste them into the email body before sending.",
    };
  }
}

export function prepareGitHubIssueReport(
  appName: string,
  {
    title = "[BUG] ",
    diagnostics = "",
    template = "bug_report.md",
  }: {
    title?: string;
    diagnostics?: string;
    template?: string;
  },
): ReportAction {
  const body = [
    "**Describe the bug**",
    "A clear and concise description of what happened in " + appName + ".",
    "",
    "**System diagnostics (auto-paste below)**",
    "```json\n" + diagnostics + "\n```",
    "",
    "**Steps to reproduce**",
    "1. ...",
    "",
    "**Expected behavior**",
    "",
    "**Additional context**",
    "",
  ].join("\n");

  if (buildGithubNewIssueUrl({ body }).length < MAX_URL_LENGTH) {
    return {
      openUrl: buildGithubNewIssueUrl({ body, title, template }),
      didCopy: false,
    };
  } else {
    const smallBody = [
      "**Describe the bug**",
      "A clear and concise description of what the bug is.",
      "",
      "**System diagnostics**",
      "Diagnostics are long — I copied them to the clipboard. Please paste them below.",
    ].join("\n");

    return {
      openUrl: buildGithubNewIssueUrl({ body: smallBody, title, template }),
      didCopy: true,
      copyText: diagnostics,
      message:
        "Diagnostics copied to clipboard. Paste them into the issue body.",
    };
  }
}
