/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { Processor } from "unified";

export function remarkDisableIndentedCode(this: Processor) {
  const data = this.data();

  const micromarkExtensions =
    (data as any).micromarkExtensions ||
    ((data as any).micromarkExtensions = []);

  micromarkExtensions.push({
    disable: { null: ["codeIndented"] },
  });
}
