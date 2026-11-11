# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

import os
import subprocess
import sys
from pathlib import Path
from PyInstaller.utils.hooks import collect_data_files, collect_submodules, copy_metadata

BUILD_MODE = os.environ.get("SQUIGIT_OCR_PYI_MODE", "onedir").strip().lower()
if BUILD_MODE not in {"onedir", "onefile"}:
    raise ValueError(
        f"Unsupported SQUIGIT_OCR_PYI_MODE={BUILD_MODE!r}; expected 'onedir' or 'onefile'"
    )

if sys.platform == "win32":
    site_packages = "venv/Lib/site-packages"
else:
    site_packages = f"venv/lib/python{sys.version_info.major}.{sys.version_info.minor}/site-packages"

def safe_copy_metadata(dist_name):
    try:
        return copy_metadata(dist_name)
    except Exception as exc:
        print(f"[warn] metadata not found for {dist_name}: {exc}")
        return []

def macos_openmp_binaries():
    if sys.platform != "darwin":
        return []

    prefixes = []
    for opt_dir in (Path("/opt/homebrew/opt"), Path("/usr/local/opt")):
        prefixes.append(opt_dir / "gcc")
        prefixes.extend(sorted(opt_dir.glob("gcc@*"), reverse=True))

    for formula in ("gcc", "gcc@15", "gcc@14", "gcc@13"):
        try:
            brew_prefix = subprocess.check_output(
                ["brew", "--prefix", formula],
                stderr=subprocess.DEVNULL,
                text=True,
            ).strip()
        except (FileNotFoundError, subprocess.CalledProcessError):
            continue
        if brew_prefix:
            prefixes.insert(0, Path(brew_prefix))

    for prefix in prefixes:
        candidates = [prefix / "lib/gcc/current/libgomp.1.dylib"]
        candidates.extend(prefix.glob("lib/gcc/*/libgomp.1.dylib"))
        for candidate in candidates:
            if candidate.is_file():
                print(f"Bundling macOS OpenMP runtime: {candidate}")
                return [(str(candidate), ".")]

    raise FileNotFoundError(
        "Paddle requires libgomp.1.dylib, but no installed Homebrew GCC runtime was found"
    )

metadata_datas = []
for dist_name in [
    "paddlepaddle",
    "paddleocr",
    "paddlex",
    "imagesize",
    "opencv-python-headless",
    "pyclipper",
    "python-bidi",
    "shapely",
    "requests",
    "PyYAML",
    "pydantic",
    "ujson",
]:
    metadata_datas += safe_copy_metadata(dist_name)

cython_datas = collect_data_files("Cython")
native_binaries = macos_openmp_binaries()

a = Analysis(
    ['src/main.py'],
    pathex=[],
    binaries=native_binaries,
    datas=[
        (f'{site_packages}/paddle/libs', 'paddle/libs'),
        (f'{site_packages}/paddleocr', 'paddleocr'),
        (f'{site_packages}/paddlex', 'paddlex'),
        ('models', 'models'),
        ('src', 'src'),
    ] + metadata_datas + cython_datas,
    hiddenimports=collect_submodules('paddleocr')
    + [
        'requests',
        'PIL.ImageDraw',
        'PIL.ImageFont',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'torch',
        'tensorflow',
        'cv2.gapi',
        'matplotlib',
        'sklearn',
        'modelscope',
        'huggingface_hub',
        'hf_xet',
        'pypdfium2',
        'pypdfium2_raw',
        'rich',
        'typer',
        'markdown_it',
        'mdurl',
    ],
    noarchive=False,
    optimize=0,
)

pyz = PYZ(a.pure)

if BUILD_MODE == "onefile":
    exe = EXE(
        pyz,
        a.scripts,
        a.binaries,
        a.datas,
        [],
        name='squigit-ocr',
        debug=False,
        bootloader_ignore_signals=False,
        strip=False,
        upx=True,
        upx_exclude=[],
        runtime_tmpdir=None,
        console=True,
        disable_windowed_traceback=False,
        argv_emulation=False,
        target_arch=None,
        codesign_identity=None,
        entitlements_file=None,
    )
else:
    exe = EXE(
        pyz,
        a.scripts,
        [],
        exclude_binaries=True,
        name='squigit-ocr',
        debug=False,
        bootloader_ignore_signals=False,
        strip=False,
        upx=True,
        upx_exclude=[],
        console=True,
        disable_windowed_traceback=False,
        argv_emulation=False,
        target_arch=None,
        codesign_identity=None,
        entitlements_file=None,
    )

    coll = COLLECT(
        exe,
        a.binaries,
        a.zipfiles,
        a.datas,
        strip=False,
        upx=True,
        upx_exclude=[],
        name='squigit-ocr',
    )
