/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

// =============================================================================
// GitHub
// =============================================================================

const GITHUB_BASE = (sub: string = "github") =>
  `https://${sub}.com/a7mddra/snapllm`;

export const github = {
  repo: GITHUB_BASE(),
  latestRelease: `${GITHUB_BASE()}/releases/latest`,
  rawChangelog: `${GITHUB_BASE("raw.githubusercontent")}/main/CHANGELOG.md`,
  issues: {
    base: `${GITHUB_BASE()}/issues`,
    new: (template: string = "bug_report.md") =>
      `${GITHUB_BASE()}/issues/new?template=${template}`,
  },
  license: `${GITHUB_BASE()}/blob/main/LICENSE`,
  docs: (path: string = "") => `${GITHUB_BASE()}/blob/main/docs/${path}`,
  blob: (branch: string, path: string) =>
    `${GITHUB_BASE()}/blob/${branch}/${path}`,
};

// =============================================================================
// Google
// =============================================================================

const GOOGLE_BASE = (sub: string) => `https://${sub}.google.com`;

export const google = {
  search: GOOGLE_BASE("www"),
  lens: GOOGLE_BASE("lens"),
  translate: GOOGLE_BASE("translate"),
  aiStudio: {
    dashboard: GOOGLE_BASE("aistudio"),
    key: `${GOOGLE_BASE("aistudio")}/app/apikey`,
  },
  support: GOOGLE_BASE("support"),
};

// =============================================================================
// ImgBB
// =============================================================================

const IMGBB_BASE = "https://api.imgbb.com";

export const imgbb = {
  upload: `${IMGBB_BASE}/1/upload`,
};
