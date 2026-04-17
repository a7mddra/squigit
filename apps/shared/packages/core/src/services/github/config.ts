/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

const GITHUB_BASE = (sub: string = "github") =>
  `https://${sub}.com/a7mddra/squigit`;

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
