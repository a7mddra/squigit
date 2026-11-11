# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
Patch paddlex/inference/utils/official_models.py to make optional hoster deps lazy.

PaddleX imports `huggingface_hub` and `modelscope` at module import time.
For local/offline OCR inference we do not need those hosters, so this patch
wraps these imports in try/except to avoid hard startup failures in frozen builds.
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
        / "utils"
        / "official_models.py"
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
        / "utils"
        / "official_models.py"
    )

print(f"Patching {target}")

if not target.exists():
    print("[ERROR] Target file not found.")
    raise SystemExit(1)

content = target.read_text(encoding="utf-8")
marker = "# Patched: optionalize huggingface_hub/modelscope imports"

if marker in content:
    print("- Already patched.")
    raise SystemExit(0)

newline = "\r\n" if "\r\n" in content else "\n"
normalized = content.replace("\r\n", "\n")

if "import huggingface_hub as hf_hub" not in normalized:
    print("[ERROR] huggingface_hub import line not found; PaddleX layout changed.")
    raise SystemExit(1)

if "import modelscope" not in normalized:
    print("[ERROR] modelscope import line not found; PaddleX layout changed.")
    raise SystemExit(1)

hf_patch = """# Patched: optionalize huggingface_hub/modelscope imports
try:
    import huggingface_hub as hf_hub
except Exception:
    hf_hub = None
else:
    hf_hub.logging.set_verbosity_error()
"""

normalized = normalized.replace(
    "import huggingface_hub as hf_hub\n\nhf_hub.logging.set_verbosity_error()\n",
    f"{hf_patch}\n",
)

normalized = normalized.replace(
    "import modelscope\n",
    "try:\n    import modelscope\nexcept Exception:\n    modelscope = None\n",
)

target.write_text(normalized.replace("\n", newline), encoding="utf-8")
print("[OK] Patch applied.")
