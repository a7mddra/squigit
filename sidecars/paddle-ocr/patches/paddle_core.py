# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

"""
Patch paddle/base/core.py for PyInstaller frozen executables.

This inserts an early _MEIPASS-aware lib lookup in set_paddle_lib_path().
Compatible with PaddlePaddle 3.x.
"""

from __future__ import annotations

import pathlib
import sys

SCRIPT_DIR = pathlib.Path(__file__).parent.parent.absolute()
PY_VERSION = f"python{sys.version_info.major}.{sys.version_info.minor}"

if sys.platform == "win32":
    path = SCRIPT_DIR / "venv" / "Lib" / "site-packages" / "paddle" / "base" / "core.py"
else:
    path = (
        SCRIPT_DIR
        / "venv"
        / "lib"
        / PY_VERSION
        / "site-packages"
        / "paddle"
        / "base"
        / "core.py"
    )

print(f"Patching {path}")

with open(path, "r", encoding="utf-8", newline="") as file:
    content = file.read()

marker_v2 = "# Patched: handle PyInstaller frozen executable (v2)"
marker_v3 = "# Patched: handle PyInstaller frozen executable (v3)"
marker_v4 = "# Patched: handle PyInstaller frozen executable (v4)"
if marker_v4 in content:
    print("- Already patched (v4).")
    raise SystemExit(0)

signature = "def set_paddle_lib_path():"
idx = content.find(signature)
if idx == -1:
    print("[ERROR] Could not find set_paddle_lib_path(). Manual fix may be needed.")
    raise SystemExit(1)

line_end = content.find("\n", idx)
if line_end == -1:
    line_end = len(content)

site_dirs_idx = content.find("    site_dirs = ", line_end)
if site_dirs_idx == -1:
    print("[ERROR] Could not find site_dirs assignment; Paddle layout changed.")
    raise SystemExit(1)

new_injection = """
    # Patched: handle PyInstaller frozen executable (v4)
    if getattr(sys, "frozen", False):
        candidate_lib_dirs = []
        candidate_custom_dirs = []

        if hasattr(sys, "_MEIPASS"):
            candidate_lib_dirs.append(os.path.join(sys._MEIPASS, "paddle", "libs"))
            candidate_lib_dirs.append(
                os.path.join(sys._MEIPASS, "_internal", "paddle", "libs")
            )
            candidate_custom_dirs.append(
                os.path.join(sys._MEIPASS, "paddle_custom_device")
            )
            candidate_custom_dirs.append(
                os.path.join(sys._MEIPASS, "_internal", "paddle_custom_device")
            )

        exe_dir = os.path.dirname(os.path.abspath(sys.executable))
        candidate_lib_dirs.append(os.path.join(exe_dir, "paddle", "libs"))
        candidate_lib_dirs.append(os.path.join(exe_dir, "_internal", "paddle", "libs"))
        candidate_custom_dirs.append(os.path.join(exe_dir, "paddle_custom_device"))
        candidate_custom_dirs.append(
            os.path.join(exe_dir, "_internal", "paddle_custom_device")
        )

        lib_dir = next((d for d in candidate_lib_dirs if os.path.exists(d)), None)
        if lib_dir:
            _set_paddle_lib_path(lib_dir)
            custom_dir = next(
                (d for d in candidate_custom_dirs if os.path.exists(d)),
                None,
            )
            if custom_dir:
                set_paddle_custom_device_lib_path(custom_dir)
            if os.name != "nt":
                try:
                    from ctypes import CDLL

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

                    for so_name in lib_candidates:
                        so_path = os.path.join(lib_dir, so_name)
                        if os.path.exists(so_path):
                            try:
                                CDLL(so_path)
                            except OSError:
                                pass
                except Exception:
                    pass
            return
"""

patched = content[: line_end + 1] + new_injection + content[site_dirs_idx:]

with open(path, "w", encoding="utf-8", newline="") as file:
    file.write(patched)

if marker_v3 in content or marker_v2 in content:
    print("[OK] Upgraded patch to v4.")
else:
    print("[OK] Patch applied successfully.")
