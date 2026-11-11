#!/usr/bin/env python3
# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

import cv2
import numpy as np

if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from src import EngineConfig, OCREngine  # noqa: E402


def main() -> int:
    cfg = EngineConfig()

    required_model_dirs = [cfg.det_model_dir, cfg.rec_model_dir, cfg.cls_model_dir]
    missing = [p for p in required_model_dirs if not Path(p).exists()]
    if missing:
        print("OCR runtime smoke failed: missing model directories:")
        for p in missing:
            print(f"  - {p}")
        return 1

    try:
        engine = OCREngine(cfg)
        # Force full PaddleOCR initialization to catch missing runtime deps early.
        engine._get_ocr()
    except Exception as exc:
        print(f"OCR runtime smoke failed during engine init: {exc}")
        return 1

    image = np.full((220, 900, 3), 255, dtype=np.uint8)
    cv2.putText(
        image,
        "SQUIGIT OCR SMOKE",
        (20, 130),
        cv2.FONT_HERSHEY_SIMPLEX,
        1.6,
        (0, 0, 0),
        3,
        cv2.LINE_AA,
    )

    fd, tmp_path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        cv2.imwrite(tmp_path, image)
        results = engine.process(tmp_path)
    except Exception as exc:
        print(f"OCR runtime smoke failed during inference: {exc}")
        return 1
    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass

    if not isinstance(results, list):
        print("OCR runtime smoke failed: result is not a list")
        return 1

    print(f"OCR runtime smoke passed (detections={len(results)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
