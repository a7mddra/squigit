# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
Data models for OCR Engine.

This module defines the data structures used to represent
OCR results including bounding boxes and text detections.

@author a7mddra
@version 0.1.0
"""

import json
from dataclasses import dataclass, field
from typing import List, Tuple
import numpy as np


class NumpyEncoder(json.JSONEncoder):
    """
    JSON encoder that handles NumPy types.

    Converts NumPy integers, floats, and arrays to their
    Python equivalents for JSON serialization.
    """

    def default(self, obj):
        """
        Convert NumPy types to JSON-serializable Python types.

        @param obj Object to convert.
        @return JSON-serializable Python object.
        """
        if isinstance(obj, np.integer):
            return int(obj)
        elif isinstance(obj, np.floating):
            return float(obj)
        elif isinstance(obj, np.ndarray):
            return obj.tolist()
        return super().default(obj)


@dataclass
class BoundingBox:
    """
    Represents a quadrilateral bounding box around detected text.

    The box is defined by four corner points in clockwise order
    starting from the top-left corner.

    @example
        box = BoundingBox(
            top_left=[10, 20],
            top_right=[100, 20],
            bottom_right=[100, 50],
            bottom_left=[10, 50]
        )
    """

    top_left: List[float] = field(default_factory=list)
    top_right: List[float] = field(default_factory=list)
    bottom_right: List[float] = field(default_factory=list)
    bottom_left: List[float] = field(default_factory=list)

    def to_list(self) -> List[List[float]]:
        """
        Convert bounding box to list of coordinates.

        @return List of [x, y] coordinate pairs.
        """
        return [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]

    @property
    def center(self) -> Tuple[float, float]:
        """
        Calculate the center point of the bounding box.

        @return Tuple of (x, y) center coordinates.
        """
        x = (self.top_left[0] + self.bottom_right[0]) / 2
        y = (self.top_left[1] + self.bottom_right[1]) / 2
        return (x, y)

    @property
    def width(self) -> float:
        """
        Calculate the width of the bounding box.

        @return Width in pixels.
        """
        return abs(self.top_right[0] - self.top_left[0])

    @property
    def height(self) -> float:
        """
        Calculate the height of the bounding box.

        @return Height in pixels.
        """
        return abs(self.bottom_left[1] - self.top_left[1])


@dataclass
class OCRResult:
    """
    Represents a single OCR detection result.

    Contains the detected text, its bounding box coordinates,
    and the confidence score of the detection.

    @example
        result = OCRResult(
            text="Hello World",
            box=BoundingBox(...),
            confidence=0.98
        )
    """

    text: str
    box: BoundingBox
    confidence: float = 1.0

    def to_dict(self) -> dict:
        """
        Convert result to dictionary for JSON serialization.

        @return Dictionary with text, box, and confidence fields.
        """
        return {
            "text": self.text,
            "box": self.box.to_list(),
            "confidence": self.confidence,
        }

    def to_json(self) -> str:
        """
        Convert result to JSON string.

        @return JSON string representation.
        """
        return json.dumps(self.to_dict(), cls=NumpyEncoder)
