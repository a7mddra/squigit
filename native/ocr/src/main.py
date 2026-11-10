import argparse
import contextlib
import json
import os
from pathlib import Path
import sys

for key in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[key] = "1"
os.environ["PADDLE_PDX_DISABLE_MODEL_SOURCE_CHECK"] = "True"
os.environ["DISABLE_MODEL_SOURCE_CHECK"] = "True"


def recognize(image, models):
    from paddleocr import PaddleOCR

    engine = PaddleOCR(
        device="cpu",
        enable_mkldnn=False,
        use_doc_orientation_classify=False,
        use_doc_unwarping=False,
        use_textline_orientation=False,
        text_detection_model_name="PP-OCRv5_mobile_det",
        text_detection_model_dir=str(models / "PP-OCRv5_mobile_det"),
        text_recognition_model_name="en_PP-OCRv5_mobile_rec",
        text_recognition_model_dir=str(models / "en_PP-OCRv5_mobile_rec"),
    )
    boxes = []
    for page in engine.predict(str(image)):
        for text, score, polygon in zip(page["rec_texts"], page["rec_scores"], page["rec_polys"]):
            boxes.append({"text": str(text), "confidence": float(score), "box": polygon.tolist()})
    return boxes


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(prog="squigit-ocr")
    parser.add_argument("--version", action="version", version="squigit-ocr 0.1.0")
    parser.add_argument("image", type=Path)
    args = parser.parse_args()
    root = Path(getattr(sys, "_MEIPASS", Path(__file__).resolve().parents[1]))
    try:
        if not args.image.is_file():
            raise FileNotFoundError(f"Image not found: {args.image}")
        with contextlib.redirect_stdout(sys.stderr):
            result = recognize(args.image, root / "models")
        print(json.dumps(result, ensure_ascii=False), flush=True)
        return 0
    except Exception as error:
        print(json.dumps({"error": str(error)}, ensure_ascii=False), flush=True)
        return 1


if __name__ == "__main__":
    sys.exit(main())
