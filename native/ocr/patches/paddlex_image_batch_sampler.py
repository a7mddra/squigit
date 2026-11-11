# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
Patch paddlex image batch sampler to lazily initialize PDF reader.

PaddleX currently constructs `PDFReader` unconditionally in `ImageBatchSampler`,
which forces PDF-only dependencies during startup even for image-only OCR.
This patch defers PDF reader creation until a PDF input is actually processed.
"""

from __future__ import annotations

import pathlib
import sys

SCRIPT_DIR = pathlib.Path(__file__).parent.parent.absolute()
PY_VERSION = f"python{sys.version_info.major}.{sys.version_info.minor}"

if sys.platform == "win32":
    target = (
        SCRIPT_DIR
        / "venv"
        / "Lib"
        / "site-packages"
        / "paddlex"
        / "inference"
        / "common"
        / "batch_sampler"
        / "image_batch_sampler.py"
    )
else:
    target = (
        SCRIPT_DIR
        / "venv"
        / "lib"
        / PY_VERSION
        / "site-packages"
        / "paddlex"
        / "inference"
        / "common"
        / "batch_sampler"
        / "image_batch_sampler.py"
    )

print(f"Patching {target}")

if not target.exists():
    print("[ERROR] Target file not found.")
    raise SystemExit(1)

content = target.read_text(encoding="utf-8")
marker = "# Patched: lazily initialize PDFReader for image-only OCR startup"
if marker in content:
    print("- Already patched.")
    raise SystemExit(0)

init_old = """    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.pdf_reader = PDFReader(zoom=PDF_RENDER_SCALE)
"""

init_new = """    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Patched: lazily initialize PDFReader for image-only OCR startup
        self.pdf_reader = None
"""

pdf_old = """                    file_path = (
                        self._download_from_url(input)
                        if input.startswith("http")
                        else input
                    )
                    doc = self.pdf_reader.load(file_path)
"""

pdf_new = """                    file_path = (
                        self._download_from_url(input)
                        if input.startswith("http")
                        else input
                    )
                    if self.pdf_reader is None:
                        self.pdf_reader = PDFReader(zoom=PDF_RENDER_SCALE)
                    doc = self.pdf_reader.load(file_path)
"""

if init_old not in content:
    print("[ERROR] __init__ block not found; PaddleX layout changed.")
    raise SystemExit(1)

if pdf_old not in content:
    print("[ERROR] PDF sampling block not found; PaddleX layout changed.")
    raise SystemExit(1)

content = content.replace(init_old, init_new)
content = content.replace(pdf_old, pdf_new)
target.write_text(content, encoding="utf-8")
print("[OK] Patch applied.")
