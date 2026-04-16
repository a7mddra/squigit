/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

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
