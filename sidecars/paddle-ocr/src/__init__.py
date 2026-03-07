# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
OCR Engine Package.

A standalone OCR engine built on PaddleOCR for extracting
text and bounding boxes from images.

@author a7mddra
@version 0.1.0

@example
    from src import OCREngine

    engine = OCREngine()
    results = engine.process("image.png")

    for result in results:
        print(f"{result.text} at {result.box.center}")
"""

from .engine import OCREngine
from .config import EngineConfig
from .models import OCRResult, BoundingBox, NumpyEncoder

__all__ = [
    "OCREngine",
    "EngineConfig",
    "OCRResult",
    "BoundingBox",
    "NumpyEncoder",
]

__version__ = "0.1.0"
