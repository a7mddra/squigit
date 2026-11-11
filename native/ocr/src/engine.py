# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
OCR Engine - PaddleOCR based text extraction engine.

This module provides the main OCREngine class for extracting text
and bounding boxes from images using PaddleOCR.

@author a7mddra
@version 3.0.0
"""

import logging
import os
import tempfile
from typing import Any, Iterable, List, Optional, Tuple

# Must be set before importing paddleocr/paddlex to avoid online source probing in offline mode.
os.environ.setdefault("DISABLE_MODEL_SOURCE_CHECK", "True")
os.environ.setdefault("PADDLE_PDX_DISABLE_MODEL_SOURCE_CHECK", "True")

import cv2
from paddleocr import PaddleOCR

from .config import EngineConfig
from .models import BoundingBox, OCRResult

MAX_DET_SIDE = 2048

MODEL_NAME_ALIASES = {
    # App-level IDs -> official PaddleOCR model names
    "pp-ocr-v5-en": "en_PP-OCRv5_mobile_rec",
    "pp-ocr-v5-latin": "latin_PP-OCRv5_mobile_rec",
    "pp-ocr-v5-cyrillic": "cyrillic_PP-OCRv5_mobile_rec",
    "pp-ocr-v5-korean": "korean_PP-OCRv5_mobile_rec",
    "pp-ocr-v5-cjk": "PP-OCRv5_server_rec",
    "pp-ocr-v5-devanagari": "devanagari_PP-OCRv5_mobile_rec",
}


class OCREngine:
    """
    Main OCR engine class for text extraction from images.

    This class wraps PaddleOCR and provides a clean interface for
    extracting text with bounding box coordinates from images.
    """

    def __init__(self, config: Optional[EngineConfig] = None):
        self.config = config or EngineConfig()
        self._setup_environment()
        self._ocr: Optional[PaddleOCR] = None

    def _setup_environment(self) -> None:
        os.environ["PADDLEOCR_BASE_PATH"] = str(self.config.model_dir)
        cv2.setNumThreads(1)
        logging.getLogger("ppocr").setLevel(logging.ERROR)

    @staticmethod
    def _model_name_from_dir(model_dir: str) -> str:
        folder_name = os.path.basename(os.path.normpath(model_dir))
        return MODEL_NAME_ALIASES.get(folder_name, folder_name)

    def _get_ocr(self) -> PaddleOCR:
        if self._ocr is None:
            try:
                # PP3-native parameter names; 2.x still works via fallback in process().
                self._ocr = PaddleOCR(
                    use_doc_orientation_classify=False,
                    use_doc_unwarping=False,
                    use_textline_orientation=self.config.use_angle_cls,
                    text_detection_model_name=self._model_name_from_dir(
                        self.config.det_model_dir
                    ),
                    text_detection_model_dir=str(self.config.det_model_dir),
                    text_recognition_model_name=self._model_name_from_dir(
                        self.config.rec_model_dir
                    ),
                    text_recognition_model_dir=str(self.config.rec_model_dir),
                    textline_orientation_model_name=self._model_name_from_dir(
                        self.config.cls_model_dir
                    ),
                    textline_orientation_model_dir=str(self.config.cls_model_dir),
                    enable_mkldnn=False,
                )
            except Exception as exc:
                raise RuntimeError(f"Failed to initialize PaddleOCR: {exc}") from exc
        return self._ocr

    def _preprocess_image(self, image_path: str) -> Tuple[str, float, Optional[str]]:
        img = cv2.imread(image_path)
        if img is None:
            return image_path, 1.0, None

        h, w = img.shape[:2]
        max_side = max(h, w)
        if max_side <= MAX_DET_SIDE:
            return image_path, 1.0, None

        scale = MAX_DET_SIDE / max_side
        new_w = int(w * scale)
        new_h = int(h * scale)
        resized = cv2.resize(img, (new_w, new_h), interpolation=cv2.INTER_AREA)

        tmp = tempfile.NamedTemporaryFile(suffix=".png", delete=False)
        cv2.imwrite(tmp.name, resized)
        tmp.close()
        return tmp.name, scale, tmp.name

    def process(self, image_path: str) -> List[OCRResult]:
        if not os.path.exists(image_path):
            raise FileNotFoundError(f"Image not found: {image_path}")

        det_path, scale, tmp_path = self._preprocess_image(image_path)
        ocr = self._get_ocr()

        try:
            try:
                # PP3 call shape.
                result = ocr.ocr(det_path)
            except TypeError:
                # Legacy compatibility with 2.x style signature.
                result = ocr.ocr(det_path, cls=self.config.use_angle_cls)
        except Exception as exc:
            raise RuntimeError(f"OCR processing failed: {exc}") from exc
        finally:
            if tmp_path and os.path.exists(tmp_path):
                os.unlink(tmp_path)

        return self._parse_results(result, scale)

    @staticmethod
    def _as_sequence(value: Any) -> Optional[List[Any]]:
        if isinstance(value, (list, tuple)):
            return list(value)
        if hasattr(value, "tolist"):
            candidate = value.tolist()
            if isinstance(candidate, (list, tuple)):
                return list(candidate)
        return None

    @staticmethod
    def _as_float(value: Any, default: float = 0.0) -> float:
        try:
            return float(value)
        except (TypeError, ValueError):
            return default

    @staticmethod
    def _normalize_quad(points: Any) -> Optional[List[List[float]]]:
        seq = OCREngine._as_sequence(points)
        if seq is None or len(seq) < 4:
            return None

        quad: List[List[float]] = []
        for point in seq[:4]:
            point_seq = OCREngine._as_sequence(point)
            if point_seq is None or len(point_seq) < 2:
                return None
            quad.append(
                [OCREngine._as_float(point_seq[0]), OCREngine._as_float(point_seq[1])]
            )
        return quad

    @staticmethod
    def _parse_text_and_conf(
        rec_entry: Any, score_entry: Any = None
    ) -> Tuple[str, float]:
        text = ""
        confidence = OCREngine._as_float(score_entry, 1.0)

        if isinstance(rec_entry, dict):
            text = str(rec_entry.get("text", ""))
            confidence = OCREngine._as_float(
                rec_entry.get("confidence", rec_entry.get("score", confidence)),
                confidence,
            )
        elif isinstance(rec_entry, (list, tuple)):
            if len(rec_entry) > 0:
                text = str(rec_entry[0])
            if len(rec_entry) > 1:
                confidence = OCREngine._as_float(rec_entry[1], confidence)
        elif rec_entry is not None:
            text = str(rec_entry)

        return text, confidence

    @staticmethod
    def _looks_like_old_style_line(item: Any) -> bool:
        return (
            isinstance(item, (list, tuple))
            and len(item) >= 2
            and OCREngine._normalize_quad(item[0]) is not None
        )

    def _normalize_old_style_lines(
        self, lines: Iterable[Any]
    ) -> Iterable[Tuple[List[List[float]], str, float]]:
        for item in lines:
            if isinstance(item, dict):
                quad = self._normalize_quad(item.get("box") or item.get("points"))
                text, confidence = self._parse_text_and_conf(
                    item,
                    item.get("confidence", item.get("score", 1.0)),
                )
            elif self._looks_like_old_style_line(item):
                quad = self._normalize_quad(item[0])
                text, confidence = self._parse_text_and_conf(item[1])
            else:
                continue

            if quad is None or text == "":
                continue
            yield quad, text, confidence

    def _normalize_page_dict(
        self, page: dict
    ) -> Iterable[Tuple[List[List[float]], str, float]]:
        polys = page.get("rec_polys") or page.get("dt_polys") or []
        texts = page.get("rec_texts") or []
        scores = page.get("rec_scores") or []

        for idx, poly in enumerate(polys):
            quad = self._normalize_quad(poly)
            if quad is None:
                continue

            rec_entry = texts[idx] if idx < len(texts) else ""
            score_entry = scores[idx] if idx < len(scores) else None
            text, confidence = self._parse_text_and_conf(rec_entry, score_entry)

            if text == "":
                continue
            yield quad, text, confidence

    def _normalize_lines(
        self, raw_result: Any
    ) -> List[Tuple[List[List[float]], str, float]]:
        if raw_result is None:
            return []

        if isinstance(raw_result, dict):
            return list(self._normalize_page_dict(raw_result))

        if not isinstance(raw_result, (list, tuple)) or len(raw_result) == 0:
            return []

        # Older shape can be either `[line, ...]` or `[[line, ...], ...]`.
        if self._looks_like_old_style_line(raw_result[0]):
            return list(self._normalize_old_style_lines(raw_result))

        normalized: List[Tuple[List[List[float]], str, float]] = []
        for page in raw_result:
            if isinstance(page, dict):
                normalized.extend(self._normalize_page_dict(page))
            elif isinstance(page, (list, tuple)):
                normalized.extend(self._normalize_old_style_lines(page))
        return normalized

    def _parse_results(self, raw_result: Any, scale: float = 1.0) -> List[OCRResult]:
        normalized_lines = self._normalize_lines(raw_result)
        if not normalized_lines:
            return []

        inv_scale = 1.0 / scale if scale != 1.0 else 1.0
        results: List[OCRResult] = []

        for box_coords, text, confidence in normalized_lines:
            if inv_scale != 1.0:
                box_coords = [[c[0] * inv_scale, c[1] * inv_scale] for c in box_coords]

            box = BoundingBox(
                top_left=box_coords[0],
                top_right=box_coords[1],
                bottom_right=box_coords[2],
                bottom_left=box_coords[3],
            )
            results.append(OCRResult(text=text, box=box, confidence=confidence))

        return results
