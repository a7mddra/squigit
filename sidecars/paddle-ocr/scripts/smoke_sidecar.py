#!/usr/bin/env python3
# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def _read_json_output(stdout: str, stderr: str, mode: str) -> list:
    try:
        payload = json.loads(stdout.strip())
    except Exception as exc:
        raise RuntimeError(
            f"{mode}: invalid JSON stdout ({exc})\\nstdout={stdout}\\nstderr={stderr}"
        ) from exc

    if isinstance(payload, dict) and "error" in payload:
        raise RuntimeError(f"{mode}: sidecar returned error: {payload['error']}")
    if not isinstance(payload, list):
        raise RuntimeError(f"{mode}: expected list payload, got {type(payload).__name__}")
    return payload


def _write_ppm_image(path: Path, width: int, height: int, mode: str) -> None:
    header = f"P6\n{width} {height}\n255\n".encode("ascii")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(header)
        if mode == "normal":
            # Light gradient pattern (small file) to avoid trivial parser edge cases.
            row = bytearray()
            for x in range(width):
                row.extend((x % 256, 255 - (x % 256), 180))
            for _ in range(height):
                f.write(row)
        else:
            # Dense alternating pattern to make processing non-trivial for cancel smoke.
            row_a = bytes([20, 20, 20]) * width
            row_b = bytes([240, 240, 240]) * width
            for y in range(height):
                f.write(row_a if y % 2 == 0 else row_b)


def _prepend_env_path(env: dict[str, str], key: str, value: str, sep: str) -> None:
    current = env.get(key, "")
    parts = [p for p in current.split(sep) if p]
    if value in parts:
        return
    env[key] = f"{value}{sep}{current}" if current else value


def _sidecar_env(sidecar: Path) -> dict[str, str]:
    env = os.environ.copy()
    runtime_dir = sidecar.parent
    candidates = [
        runtime_dir / "_internal" / "paddle" / "libs",
        runtime_dir / "paddle" / "libs",
    ]
    paddle_lib_dir = next((p for p in candidates if p.is_dir()), None)
    if not paddle_lib_dir:
        return env

    if sys.platform == "darwin":
        _prepend_env_path(env, "DYLD_LIBRARY_PATH", str(paddle_lib_dir), ":")
        _prepend_env_path(env, "PATH", str(paddle_lib_dir), ":")
    elif os.name == "nt":
        _prepend_env_path(env, "PATH", str(paddle_lib_dir), ";")
    else:
        _prepend_env_path(env, "LD_LIBRARY_PATH", str(paddle_lib_dir), ":")
        _prepend_env_path(env, "PATH", str(paddle_lib_dir), ":")

    return env


def smoke_cli(sidecar: Path, image_path: Path, env: dict[str, str]) -> None:
    proc = subprocess.run(
        [str(sidecar), str(image_path)],
        capture_output=True,
        text=True,
        timeout=240,
        check=False,
        env=env,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"cli: non-zero exit {proc.returncode}\\nstdout={proc.stdout}\\nstderr={proc.stderr}"
        )
    _read_json_output(proc.stdout, proc.stderr, "cli")


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke test packaged OCR sidecar.")
    parser.add_argument("--sidecar", required=True, help="Path to ocr-engine executable")
    args = parser.parse_args()

    sidecar = Path(args.sidecar).resolve()
    if not sidecar.exists():
        raise SystemExit(f"Sidecar executable not found: {sidecar}")

    with tempfile.TemporaryDirectory(prefix="ocr-smoke-") as tmpdir:
        tmp = Path(tmpdir)
        normal_image = tmp / "normal.ppm"
        env = _sidecar_env(sidecar)
        _write_ppm_image(normal_image, width=960, height=240, mode="normal")
        smoke_cli(sidecar, normal_image, env)

    print("OCR sidecar smoke passed.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"OCR sidecar smoke failed: {exc}", file=sys.stderr)
        raise
