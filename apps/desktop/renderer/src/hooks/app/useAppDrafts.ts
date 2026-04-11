/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from "react";
import { ModelType } from "@/core";

export const useAppDrafts = () => {
  const [input, setInput] = useState("");
  const [imageInput, setImageInput] = useState("");
  const [inputModel, setInputModel] = useState<string>(
    ModelType.GEMINI_3_1_FLASH,
  );

  return {
    input,
    setInput,
    imageInput,
    setImageInput,
    inputModel,
    setInputModel,
  };
};
