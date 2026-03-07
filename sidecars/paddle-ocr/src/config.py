# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
Configuration for OCR Engine.

This module defines the configuration options for the OCR engine
including model paths and processing settings.

@author a7mddra
@version 0.1.0
"""

import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


def _get_base_dir() -> Path:
    """
    Determine the base directory for model files.

    When running as a frozen PyInstaller executable, uses the
    temporary extraction directory. Otherwise uses the script
    directory.

    @return Path to the base directory.
    """
    if getattr(sys, "frozen", False):
        if hasattr(sys, "_MEIPASS"):
            return Path(sys._MEIPASS)
        # OneDir fallback: keep resources relative to the executable directory.
        return Path(sys.executable).resolve().parent

    return Path(__file__).parent.parent.absolute()


@dataclass
class EngineConfig:
    """
    Configuration settings for the OCR engine.

    Controls model paths, language settings, and processing options.

    @example
        config = EngineConfig(lang='en', use_angle_cls=True)
        engine = OCREngine(config)
    """

    lang: str = "en"

    use_angle_cls: bool = True

    base_dir: Optional[Path] = None

    # Custom model paths (optional overrides)
    det_model_path: Optional[str] = None
    rec_model_path: Optional[str] = None
    cls_model_path: Optional[str] = None

    def __post_init__(self):
        """Initialize computed paths after dataclass init."""
        if self.base_dir is None:
            self.base_dir = _get_base_dir()

    @property
    def model_dir(self) -> Path:
        """
        Get the models directory path.

        @return Path to the models directory.
        """
        return self.base_dir / "models"

    @property
    def det_model_dir(self) -> str:
        """
        Get the detection model directory path.
        Returns custom path if set, otherwise default bundled path.
        """
        if self.det_model_path:
            return self.det_model_path
        return str(self.model_dir / "PP-OCRv5_mobile_det")

    @property
    def rec_model_dir(self) -> str:
        """
        Get the recognition model directory path.
        Returns custom path if set, otherwise default bundled path.
        """
        if self.rec_model_path:
            return self.rec_model_path
        return str(self.model_dir / "en_PP-OCRv5_mobile_rec")

    @property
    def cls_model_dir(self) -> str:
        """
        Get the classification model directory path.
        Returns custom path if set, otherwise default bundled path.
        """
        if self.cls_model_path:
            return self.cls_model_path
        return str(self.model_dir / "PP-LCNet_x1_0_textline_ori")
