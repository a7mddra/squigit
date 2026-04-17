/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { getSystemPort } from "../../ports/system";

export async function openExternalUrl(url: string): Promise<void> {
  await getSystemPort().openExternalUrl(url);
}
