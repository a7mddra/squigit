/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from "react";
import { DEFAULT_MODEL_ID } from "@squigit/core/config";

export const useAppDrafts = () => {
  const [input, setInput] = useState("");
  const [imageInput, setImageInput] = useState("");
  const [inputModel, setInputModel] = useState<string>(DEFAULT_MODEL_ID);

  return {
    input,
    setInput,
    imageInput,
    setImageInput,
    inputModel,
    setInputModel,
  };
};
