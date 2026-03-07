# Development Setup

Use `xtask` as the contributor control panel.

## Quick Start

```bash
cargo xtask setup
cargo xtask report
cargo xtask build
```

## Setup Modes

`cargo xtask setup` runs safe checks and user-level setup guidance.

`cargo xtask setup --all` attempts broader OS-level installs when possible, then prints exact remediation commands for anything still missing.

Component-focused setup:

```bash
cargo xtask setup --qt
cargo xtask setup --py
cargo xtask setup --cargo
cargo xtask setup --npm
cargo xtask setup --qt --py --cargo --npm
```

## Build Selectors

Default build includes current buildable targets (OCR, Whisper, Capture, Desktop):

```bash
cargo xtask build
cargo xtask build all
cargo xtask build --all
```

Exclusions and explicit target combinations:

```bash
cargo xtask build --all -ocr
cargo xtask build all -whisper
cargo xtask build ocr whisper
cargo xtask build capture-qt
```

Desktop aliases:

```bash
cargo xtask build desktop
cargo xtask build tauri
cargo xtask build app
```

OCR size report is opt-in:

```bash
cargo xtask build ocr --measure-ocr-size
# or
SQUIGIT_OCR_MEASURE_SIZE=1 cargo xtask build ocr
```

## Health Report

```bash
cargo xtask report
cargo xtask report --strict
```

## Version Sync

Explicit version:

```bash
cargo xtask version 0.1.1
```

Semantic bump:

```bash
cargo xtask version --bump patch
cargo xtask version --bump minor
cargo xtask version --bump major
```

Canonical project version lives in the root `VERSION` file.
