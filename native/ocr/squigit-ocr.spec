import os
from importlib.metadata import requires
from packaging.requirements import Requirement

os.environ["PADDLE_PDX_DISABLE_MODEL_SOURCE_CHECK"] = "True"

from PyInstaller.utils.hooks import collect_all, copy_metadata

packages = ("paddle", "paddlex", "paddleocr")
datas, binaries, hiddenimports = [], [], []
for package in packages:
    data, libraries, modules = collect_all(package)
    datas.extend(data)
    binaries.extend(libraries)
    hiddenimports.extend(modules)
for distribution in ("paddlepaddle", "paddleocr", "paddlex"):
    datas.extend(copy_metadata(distribution, recursive=True))
for dependency in requires("paddlex"):
    requirement = Requirement(dependency)
    if requirement.marker is not None and requirement.marker.evaluate({"extra": "ocr"}):
        datas.extend(copy_metadata(requirement.name, recursive=True))
datas.append(("models", "models"))

a = Analysis(["src/main.py"], pathex=[], binaries=binaries, datas=datas,
    hiddenimports=hiddenimports, excludes=["torch", "tensorflow", "matplotlib", "IPython", "pytest"],
    noarchive=False)
pyz = PYZ(a.pure)
exe = EXE(pyz, a.scripts, [], exclude_binaries=True, name="squigit-ocr", console=True, upx=False)
coll = COLLECT(exe, a.binaries, a.datas, name="squigit-ocr", upx=False)
