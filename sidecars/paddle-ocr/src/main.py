#!/usr/bin/env python3
# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

import argparse
import contextlib
import json
import os
import sys
import traceback
import warnings
from pathlib import Path
from typing import Any

os.environ["OMP_NUM_THREADS"] = "1"
os.environ["OPENBLAS_NUM_THREADS"] = "1"
os.environ["MKL_NUM_THREADS"] = "1"
os.environ["NUMEXPR_NUM_THREADS"] = "1"
os.environ["OMP_WAIT_POLICY"] = "PASSIVE"

# Keep sidecar stderr focused on actionable OCR failures.
warnings.filterwarnings(
    "ignore",
    message=r".*urllib3 .* doesn't match a supported version!.*",
)


def _prepend_env_path(key: str, value: str, sep: str = os.pathsep) -> None:
    current = os.environ.get(key, "")
    parts = [p for p in current.split(sep) if p]
    if value in parts:
        return
    if current:
        os.environ[key] = f"{value}{sep}{current}"
    else:
        os.environ[key] = value


def _get_frozen_runtime_root() -> str | None:
    if not getattr(sys, "frozen", False):
        return None
    if hasattr(sys, "_MEIPASS"):
        return str(sys._MEIPASS)
    return str(Path(sys.executable).resolve().parent)


def _get_frozen_paddle_lib_dir() -> str | None:
    candidates: list[str] = []

    runtime_root = _get_frozen_runtime_root()
    if runtime_root:
        candidates.append(os.path.join(runtime_root, "paddle", "libs"))
        candidates.append(os.path.join(runtime_root, "_internal", "paddle", "libs"))

    exe_dir = str(Path(sys.executable).resolve().parent)
    candidates.append(os.path.join(exe_dir, "paddle", "libs"))
    candidates.append(os.path.join(exe_dir, "_internal", "paddle", "libs"))

    for candidate in candidates:
        if os.path.isdir(candidate):
            return candidate
    return None


def _bootstrap_frozen_loader_env() -> None:
    """
    Re-exec frozen sidecar with loader env pointing to bundled Paddle libs.
    """
    if os.name == "nt":
        return

    paddle_lib_dir = _get_frozen_paddle_lib_dir()
    if not paddle_lib_dir:
        return

    if os.environ.get("SQUIGIT_PADDLE_LIBPATH_BOOTSTRAPPED") == "1":
        return

    if sys.platform == "darwin":
        key = "DYLD_LIBRARY_PATH"
        sep = ":"
    else:
        key = "LD_LIBRARY_PATH"
        sep = ":"

    current = os.environ.get(key, "")
    parts = [p for p in current.split(sep) if p]
    if paddle_lib_dir in parts:
        return

    env = os.environ.copy()
    env[key] = f"{paddle_lib_dir}{sep}{current}" if current else paddle_lib_dir
    env["SQUIGIT_PADDLE_LIBPATH_BOOTSTRAPPED"] = "1"
    os.execvpe(sys.executable, [sys.executable, *sys.argv[1:]], env)


def _configure_frozen_lib_paths() -> None:
    paddle_lib_dir = _get_frozen_paddle_lib_dir()
    if not paddle_lib_dir:
        return

    if os.name == "nt":
        _prepend_env_path("PATH", paddle_lib_dir, ";")
    elif sys.platform == "darwin":
        _prepend_env_path("DYLD_LIBRARY_PATH", paddle_lib_dir, ":")
        _prepend_env_path("PATH", paddle_lib_dir, ":")
    else:
        _prepend_env_path("LD_LIBRARY_PATH", paddle_lib_dir, ":")
        _prepend_env_path("PATH", paddle_lib_dir, ":")


def _preload_frozen_paddle_libs() -> None:
    if os.name == "nt":
        return

    paddle_lib_dir = _get_frozen_paddle_lib_dir()
    if not paddle_lib_dir:
        return

    try:
        import ctypes
    except Exception:
        return

    if sys.platform == "darwin":
        lib_candidates = (
            "libiomp5.dylib",
            "libmklml_intel.dylib",
        )
    else:
        lib_candidates = (
            "libiomp5.so",
            "libmklml_intel.so",
            "libgfortran.so.3",
            "libquadmath.so.0",
        )

    rtld_global = getattr(ctypes, "RTLD_GLOBAL", None)
    for lib_name in lib_candidates:
        lib_path = os.path.join(paddle_lib_dir, lib_name)
        if not os.path.exists(lib_path):
            continue
        try:
            if rtld_global is None:
                ctypes.CDLL(lib_path)
            else:
                ctypes.CDLL(lib_path, mode=rtld_global)
        except OSError:
            pass


_bootstrap_frozen_loader_env()
_configure_frozen_lib_paths()
_preload_frozen_paddle_libs()

if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).parent.parent))

from src import EngineConfig, NumpyEncoder, OCREngine, __version__


def _format_exception(exc: Exception) -> str:
    if os.environ.get("OCR_DEBUG_TRACEBACK", "").strip().lower() in {
        "1",
        "true",
        "yes",
        "on",
    }:
        return f"{exc}\n{traceback.format_exc()}"
    return str(exc)


def _emit_json(payload: Any) -> None:
    stream = sys.__stdout__
    stream.write(json.dumps(payload, cls=NumpyEncoder))
    stream.flush()


def _emit_error(message: str) -> int:
    _emit_json({"error": message})
    return 1


def _create_config(args: argparse.Namespace) -> EngineConfig:
    return EngineConfig(
        lang=args.lang,
        use_angle_cls=args.use_angle_cls,
        det_model_path=args.det_model_dir,
        rec_model_path=args.rec_model_dir,
        cls_model_path=args.cls_model_dir,
    )


def process_path(image_path: str, args: argparse.Namespace) -> int:
    if not Path(image_path).exists():
        return _emit_error(f"Image not found: {image_path}")

    try:
        config = _create_config(args)
        # Route noisy Python-level prints from third-party code away from stdout.
        with contextlib.redirect_stdout(sys.stderr):
            engine = OCREngine(config)
            results = engine.process(image_path)
        output = [result.to_dict() for result in results]
        _emit_json(output)
        return 0
    except Exception as exc:
        return _emit_error(_format_exception(exc))


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Squigit PaddleOCR sidecar (CLI mode)."
    )
    parser.add_argument(
        "--version", action="version", version=__version__
    )
    parser.add_argument("image_path", help="Path to image file.")
    parser.add_argument("--lang", default="en", help="Language hint (default: en).")
    parser.add_argument(
        "--det-model-dir", default=None, help="Detection model directory."
    )
    parser.add_argument(
        "--rec-model-dir", default=None, help="Recognition model directory."
    )
    parser.add_argument(
        "--cls-model-dir", default=None, help="Textline orientation model directory."
    )
    parser.add_argument(
        "--use-angle-cls",
        dest="use_angle_cls",
        action="store_true",
        default=True,
        help="Enable textline orientation model.",
    )
    parser.add_argument(
        "--no-angle-cls",
        dest="use_angle_cls",
        action="store_false",
        help="Disable textline orientation model.",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()
    return process_path(args.image_path, args)


if __name__ == "__main__":
    sys.exit(main())
