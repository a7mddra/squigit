# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import argparse
import shutil
import tarfile
from pathlib import Path
from typing import Iterable

import requests

ARCHIVE_BASE_URLS = (
    "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0",
)
HF_BASE_URLS = (
    "https://huggingface.co",
    "https://hf-mirror.com",
)
SCRIPT_DIR = Path(__file__).parent.resolve()
MODELS_DIR = SCRIPT_DIR / "models"

MODELS = [
    "PP-OCRv5_mobile_det_infer.tar",
    "en_PP-OCRv5_mobile_rec_infer.tar",
    "PP-LCNet_x1_0_textline_ori_infer.tar",
]

HF_REPO_BY_MODEL = {
    "PP-OCRv5_mobile_det": "PaddlePaddle/PP-OCRv5_mobile_det",
    "en_PP-OCRv5_mobile_rec": "PaddlePaddle/en_PP-OCRv5_mobile_rec",
    "PP-LCNet_x1_0_textline_ori": "PaddlePaddle/PP-LCNet_x1_0_textline_ori",
}

HF_REQUIRED_FILES = (
    "inference.json",
    "inference.pdiparams",
    "inference.yml",
)

REQUEST_HEADERS = {
    "User-Agent": "snapllm-ocr-model-bootstrap/1.0",
}


def _model_dir_name(archive_name: str) -> str:
    return archive_name.removesuffix("_infer.tar")


def _allowlisted_model_dirs() -> set[str]:
    allowed = set()
    for archive_name in MODELS:
        model_name = _model_dir_name(archive_name)
        allowed.add(model_name)
        allowed.add(f"{model_name}_infer")
    return allowed


def _has_model_graph(model_dir: Path) -> bool:
    return (model_dir / "inference.pdmodel").exists() or (
        model_dir / "inference.json"
    ).exists()


def _is_model_ready(model_dir: Path) -> bool:
    return _has_model_graph(model_dir) and (
        model_dir / "inference.pdiparams"
    ).exists()


def _archive_urls(archive_name: str) -> Iterable[str]:
    for base in ARCHIVE_BASE_URLS:
        yield f"{base.rstrip('/')}/{archive_name}"


def _normalize_model_dir(model_name: str) -> Path:
    canonical = MODELS_DIR / model_name
    inferred = MODELS_DIR / f"{model_name}_infer"

    if _is_model_ready(canonical):
        return canonical

    if _is_model_ready(inferred):
        if canonical.exists():
            shutil.rmtree(canonical)
        inferred.rename(canonical)
        return canonical

    return canonical


def download_file(url: str, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with requests.get(
        url,
        stream=True,
        timeout=(20, 180),
        headers=REQUEST_HEADERS,
        allow_redirects=True,
    ) as response:
        response.raise_for_status()
        with output_path.open("wb") as file:
            for chunk in response.iter_content(chunk_size=1024 * 1024):
                if chunk:
                    file.write(chunk)


def _download_from_hf(model_name: str, model_dir: Path) -> None:
    repo = HF_REPO_BY_MODEL.get(model_name)
    if not repo:
        raise RuntimeError(f"No Hugging Face repository mapping for {model_name}")

    if model_dir.exists():
        shutil.rmtree(model_dir)
    model_dir.mkdir(parents=True, exist_ok=True)

    errors = []
    for base in HF_BASE_URLS:
        failed = False
        for file_name in HF_REQUIRED_FILES:
            url = f"{base.rstrip('/')}/{repo}/resolve/main/{file_name}"
            dst = model_dir / file_name
            try:
                print(f"Downloading {model_name}/{file_name} from {base}...")
                download_file(url, dst)
            except Exception as exc:
                errors.append(f"{url}: {exc}")
                failed = True
                break

        if not failed and _is_model_ready(model_dir):
            return

        for file_name in HF_REQUIRED_FILES:
            try:
                (model_dir / file_name).unlink()
            except FileNotFoundError:
                pass

    raise RuntimeError(
        "Hugging Face fallback download failed:\n  - " + "\n  - ".join(errors)
    )


def extract_archive(archive_path: Path, destination: Path) -> None:
    with tarfile.open(archive_path, "r") as archive:
        try:
            archive.extractall(path=destination, filter="data")
        except TypeError:
            archive.extractall(path=destination)


def ensure_model(archive_name: str) -> None:
    model_name = _model_dir_name(archive_name)
    model_dir = _normalize_model_dir(model_name)

    if _is_model_ready(model_dir):
        print(f"Model already present: {model_name}")
        return

    for stale_dir in [model_dir, MODELS_DIR / f"{model_name}_infer"]:
        if stale_dir.exists():
            shutil.rmtree(stale_dir)

    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    archive_path = MODELS_DIR / archive_name
    archive_errors = []

    for url in _archive_urls(archive_name):
        try:
            print(f"Downloading {archive_name} from {url}...")
            download_file(url, archive_path)

            print(f"Extracting {archive_name}...")
            extract_archive(archive_path, MODELS_DIR)

            if archive_path.exists():
                archive_path.unlink()

            model_dir = _normalize_model_dir(model_name)
            if _is_model_ready(model_dir):
                return
            archive_errors.append(
                f"{url}: extracted archive but model is incomplete ({model_name})"
            )
        except Exception as exc:
            archive_errors.append(f"{url}: {exc}")
        finally:
            if archive_path.exists():
                archive_path.unlink()

    print(
        f"Archive mirrors failed for {archive_name}; trying direct Hugging Face files..."
    )
    _download_from_hf(model_name, model_dir)
    model_dir = _normalize_model_dir(model_name)
    if _is_model_ready(model_dir):
        return

    archive_error_text = "\n  - ".join(archive_errors) if archive_errors else "unknown"
    raise RuntimeError(
        f"Model bootstrap failed for {model_name}.\n"
        f"Archive attempts:\n  - {archive_error_text}"
    )


def prune_stale_model_dirs() -> None:
    if not MODELS_DIR.exists():
        return

    allowed = _allowlisted_model_dirs()
    for path in sorted(MODELS_DIR.iterdir()):
        if not path.is_dir():
            continue
        if path.name in allowed:
            continue
        print(f"Removing stale model directory: {path.name}")
        shutil.rmtree(path)


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare bundled OCR model directories.")
    parser.add_argument(
        "--clean-stale",
        action="store_true",
        help="Remove non-allowlisted model directories before ensuring required models.",
    )
    args = parser.parse_args()

    print("Preparing bundled PP-OCRv5 models...")
    if args.clean_stale:
        prune_stale_model_dirs()
    for archive_name in MODELS:
        ensure_model(archive_name)
    print("All bundled models are ready.")


if __name__ == "__main__":
    main()
