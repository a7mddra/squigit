# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import argparse
import shutil
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
MODELS_DIR = SCRIPT_DIR / "models"

MODELS = {
    "ggml-tiny.en.bin": [
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
    ],
    "ggml-base.en.bin": [
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
    ],
}

CHUNK_SIZE = 1024 * 1024
REQUEST_HEADERS = {
    "User-Agent": "squigit-whisper-model-bootstrap/1.0",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Download and prepare bundled whisper models."
    )
    parser.add_argument(
        "--clean-stale",
        action="store_true",
        help="Remove unknown files/directories in sidecars/whisper-stt/models before download.",
    )
    parser.add_argument(
        "--models",
        nargs="+",
        choices=sorted(MODELS.keys()),
        default=sorted(MODELS.keys()),
        help="Subset of models to download (default: all known models).",
    )
    return parser.parse_args()


def clean_stale(selected_models: set[str]) -> None:
    if not MODELS_DIR.exists():
        return

    for path in MODELS_DIR.iterdir():
        if path.name in selected_models:
            continue
        print(f"Removing stale entry: {path.name}")
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink(missing_ok=True)


def validate_model(path: Path) -> None:
    if not path.exists():
        raise RuntimeError(f"Model not found after download: {path}")
    if path.stat().st_size <= 0:
        raise RuntimeError(f"Model file is empty: {path}")


def download_to_file(url: str, destination: Path) -> None:
    temp_path: Path | None = None
    request = urllib.request.Request(url, headers=REQUEST_HEADERS)
    try:
        with urllib.request.urlopen(request, timeout=180) as response:
            with tempfile.NamedTemporaryFile(
                dir=destination.parent,
                delete=False,
                prefix=f".{destination.name}.",
                suffix=".tmp",
            ) as temp_file:
                temp_path = Path(temp_file.name)
                while True:
                    chunk = response.read(CHUNK_SIZE)
                    if not chunk:
                        break
                    temp_file.write(chunk)

        if temp_path is None:
            raise RuntimeError(f"Temporary file was not created for {destination.name}")
        temp_path.replace(destination)
    finally:
        if temp_path is not None and temp_path.exists():
            temp_path.unlink(missing_ok=True)


def ensure_model(model_name: str) -> None:
    destination = MODELS_DIR / model_name
    urls = MODELS[model_name]
    errors: list[str] = []

    for url in urls:
        try:
            print(f"Downloading {model_name} from {url}")
            download_to_file(url, destination)
            validate_model(destination)
            return
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError) as exc:
            errors.append(f"{url}: {exc}")
        except Exception as exc:  # Keep final error actionable for bootstrap scripts.
            errors.append(f"{url}: {exc}")

    error_text = "\n  - ".join(errors) if errors else "unknown error"
    raise RuntimeError(f"Failed downloading {model_name}:\n  - {error_text}")


def main() -> None:
    args = parse_args()
    selected = set(args.models)

    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    if args.clean_stale:
        clean_stale(selected)

    for model_name in sorted(selected):
        ensure_model(model_name)

    for model_name in sorted(selected):
        validate_model(MODELS_DIR / model_name)

    print("Whisper models are ready.")


if __name__ == "__main__":
    main()
