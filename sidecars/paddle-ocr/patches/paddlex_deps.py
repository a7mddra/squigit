# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
Patch paddlex/utils/deps.py to relax OCR extra checks for trimmed runtime builds.

PaddleX `ocr` pipelines allow `ocr-core` as an alternative, but `ocr-core` requires
`opencv-contrib-python` and `pypdfium2` even when running image-only OCR. In our
shipping sidecar we provide `opencv-python-headless` and do not support PDF OCR,
so this patch adjusts only the extra-check map to avoid false dependency failures.
"""

from __future__ import annotations

import pathlib
import sys

SCRIPT_DIR = pathlib.Path(__file__).parent.parent.absolute()
PY_VERSION = f"python{sys.version_info.major}.{sys.version_info.minor}"

if sys.platform == "win32":
    target = (
        SCRIPT_DIR / "venv" / "Lib" / "site-packages" / "paddlex" / "utils" / "deps.py"
    )
else:
    target = (
        SCRIPT_DIR
        / "venv"
        / "lib"
        / PY_VERSION
        / "site-packages"
        / "paddlex"
        / "utils"
        / "deps.py"
    )

print(f"Patching {target}")

if not target.exists():
    print("[ERROR] Target file not found.")
    raise SystemExit(1)

content = target.read_text(encoding="utf-8")
relax_marker = "# Patched: relax OCR extra dependency checks for trimmed runtime"
opencv_marker = "# Patched: treat OpenCV package variants as cv2-compatible deps"
changed = False

if relax_marker not in content:
    anchor = "EXTRAS = _get_extras()"
    if anchor not in content:
        print("[ERROR] Could not find EXTRAS assignment; PaddleX layout changed.")
        raise SystemExit(1)

    relax_injection = """
# Patched: relax OCR extra dependency checks for trimmed runtime
def _relax_ocr_extra_dependency_checks():
    for extra_name in ("ocr-core", "ocr"):
        dep_specs = EXTRAS.get(extra_name)
        if not dep_specs:
            continue
        # PDF parser dependency is not required for image OCR flow.
        dep_specs.pop("pypdfium2", None)
        # Accept OpenCV headless/runtime variants when cv2 is importable.
        if (
            "opencv-contrib-python" in dep_specs
            and importlib.util.find_spec("cv2") is not None
        ):
            dep_specs.pop("opencv-contrib-python", None)


_relax_ocr_extra_dependency_checks()
"""
    content = content.replace(anchor, f"{anchor}{relax_injection}")
    changed = True

if opencv_marker not in content:
    old_block = """    elif dep == "onnxruntime":
        return importlib.util.find_spec("onnxruntime") is not None
    version = get_dep_version(dep)
"""
    new_block = """    elif dep == "onnxruntime":
        return importlib.util.find_spec("onnxruntime") is not None
    # Patched: treat OpenCV package variants as cv2-compatible deps
    elif dep in ("opencv-contrib-python", "opencv-python", "opencv-python-headless"):
        return importlib.util.find_spec("cv2") is not None
    version = get_dep_version(dep)
"""
    if old_block not in content:
        print(
            "[ERROR] Could not find OpenCV dependency branch; PaddleX layout changed."
        )
        raise SystemExit(1)
    content = content.replace(old_block, new_block)
    changed = True

if not changed:
    print("- Already patched.")
    raise SystemExit(0)

target.write_text(content, encoding="utf-8")
print("[OK] Patch applied.")
