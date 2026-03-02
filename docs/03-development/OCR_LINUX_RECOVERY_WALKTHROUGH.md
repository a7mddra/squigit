# OCR Linux Recovery Walkthrough (From `libmklml` Crash to Stable Build)

## 1. Starting point (failure state)

The Linux packaged sidecar failed at runtime with:

- `libmklml_intel.so: cannot open shared object file: No such file or directory`

At the same time, Linux payload size was too large compared to Windows:

- Windows compressed sidecar: ~`226 MB` (working)
- Linux compressed sidecar (before fixes): ~`485.76 MB` (failing runtime)

Constraints for this recovery:

1. Do not break Windows (`226 MB` working baseline must stay intact).
2. Keep OCR request/response shape unchanged.
3. Keep macOS support conservative and safe.
4. Reduce Linux payload without relying on fragile Paddle source rewrites.

## 2. Root causes found

### Root cause A: frozen loader order/path issue on Linux

In frozen runtime, Paddle CPU libs were not always discoverable early enough, and load order mattered:

- `libiomp5` must be available before `libmklml_intel`.
- Some layouts resolved libs from `_internal/paddle/libs`, not only `paddle/libs`.

### Root cause B: symlink flattening during OCR runtime packaging

PyInstaller `onedir` output already deduplicates many shared objects via symlinks, but xtask packaging copied runtime trees in a way that recreated duplicates, inflating Linux/macOS payload.

### Root cause C: stale model directories were being carried into bundle

Model bootstrap ensured required models existed, but did not prune unknown leftovers in `sidecars/paddle-ocr/models`.

### Root cause D: heavy default detector model

Default detector was still `PP-OCRv5_server_det` (larger) while the sidecar target is on-device CPU use. For your app behavior, mobile det is the correct default.

## 3. What changed (file-by-file)

## Runtime stability changes

- `sidecars/paddle-ocr/patches/paddle_core.py`
  - Upgraded patch logic to a v4 frozen loader patch.
  - Added candidate lookup for both:
    - `<runtime>/paddle/libs`
    - `<runtime>/_internal/paddle/libs`
    - same patterns under executable directory.
  - Fixed preload order for Unix:
    - `libiomp5` before `libmklml_intel`.
  - Keeps Windows path untouched.

- `sidecars/paddle-ocr/src/main.py`
  - Added `_get_frozen_paddle_lib_dir()` to search both classic and `_internal` layouts.
  - Added `_bootstrap_frozen_loader_env()` to re-exec with loader path prepared before Paddle init.
  - Added `_preload_frozen_paddle_libs()` (`libiomp5` first).
  - Important regression fix: re-exec argument list now preserves only user args:
    - from `[sys.executable, *sys.argv]`
    - to `[sys.executable, *sys.argv[1:]]`
  - This fixed the bug where CLI treated `./ocr-engine` as image input.

- `sidecars/paddle-ocr/scripts/smoke_sidecar.py`
  - Added sidecar environment preparation for bundled Paddle libs in subprocess runs.
  - Fixed stdin-close handling (`proc.stdin = None`) to avoid communicate/flush edge cases.

## Packaging + size changes

- `xtask/src/lib.rs`
  - Added `copy_dir_all_preserve_symlinks()` for Unix runtime copy.
  - Added helper to safely replace pre-existing paths when recreating symlinks.

- `xtask/src/commands/pkg.rs`
  - OCR runtime copy now uses symlink-preserving copy on non-Windows.
  - Added symlink integrity assertion after copy (`src` symlink count must equal `dst` count).
  - Windows copy path remains the old behavior.

- `sidecars/paddle-ocr/scripts/measure_runtime_size.py`
  - Added `--preserve-symlinks` mode.
  - Measurement can now count/archive symlinks as symlinks instead of dereferencing to full duplicate size.

- `xtask/src/commands/build.rs`
  - Uses `--preserve-symlinks` for Linux/macOS size reports.
  - Added post-build smoke gates on Linux/macOS:
    - dist sidecar smoke (`dist/ocr-engine/ocr-engine`)
    - packaged sidecar smoke (`app/binaries/paddle-ocr-<triple>/ocr-engine`)

## Model selection + hygiene changes

- `sidecars/paddle-ocr/src/config.py`
  - Default detector switched:
    - `PP-OCRv5_server_det` -> `PP-OCRv5_mobile_det`

- `sidecars/paddle-ocr/download_models.py`
  - Bundled model list switched to mobile det archive:
    - `PP-OCRv5_mobile_det_infer.tar`
  - Added `--clean-stale` option:
    - removes model directories not in allowlist before ensuring required models.
  - Added resilient download fallback chain for CI/network variance:
    - tries archive URL(s) first (BOS)
    - if archive fetch fails (example: HTTP `403` on GitHub mac runner), falls back to direct Hugging Face model file download (`inference.json`, `inference.pdiparams`, `inference.yml`)
    - includes `hf-mirror.com` as secondary HF endpoint
  - Added explicit request user-agent to reduce host-side blocking of default `python-requests` identity.

- `xtask/src/commands/build.rs`
  - Linux/macOS invocation now calls:
    - `download_models.py --clean-stale`
  - Windows still calls:
    - `download_models.py`

- Effective bundled core models now:
  - `PP-OCRv5_mobile_det`
  - `en_PP-OCRv5_mobile_rec`
  - `PP-LCNet_x1_0_textline_ori`

## Documentation update

- `docs/02-architecture/OCR.md`
  - Updated default bundled detector to mobile det.
  - Documented stale-model pruning behavior.
  - Documented symlink-preserving packaging and symlink-aware size measurement.
  - Documented Linux/macOS preload order fix.

## 4. New macOS CI workflow added

To validate build + ML behavior on hosted Apple Silicon runner:

- `.github/workflows/ocr-macos-sidecar-smoke.yml`
  - Runs on `macos-latest`.
  - Builds OCR sidecar via existing composite action (`cargo xtask build ocr` path).
  - Generates PNG via:
    - `sidecars/paddle-ocr/scripts/gen_test_image.py`
  - Executes packaged sidecar CLI on generated image.
  - Prints raw JSON OCR output in runner logs.
  - Validates JSON payload shape and non-empty detections.
  - Uploads generated image + JSON output as artifacts.

- `sidecars/paddle-ocr/scripts/gen_test_image.py`
  - Generates white canvas PNG with high-contrast black text for deterministic smoke input.

## 5. Result snapshot after fixes

Linux packaged sidecar (symlink-aware measurement):

- `file_count`: `2648`
- `raw_mb`: `953.34`
- `compressed_mb`: `310.46`

Compared to earlier Linux compressed ~`485.76 MB`, this is a large reduction while restoring runtime success.

CLI runtime now works again (same command style that previously failed):

- `./oc* /path/to/image.png` returns OCR JSON list instead of `libmklml` init error.

## 6. Windows guardrails (for Windows agent)

Windows behavior was intentionally not changed in critical code paths:

1. `build.rs` keeps Windows model bootstrap invocation unchanged.
2. Symlink-preserving copy logic is Unix-gated in `pkg.rs`.
3. Loader-env and preload logic in `main.py` is non-Windows.
4. Patch behavior for Paddle loader adjustments targets frozen runtime compatibility without modifying Windows-only branch behavior.

If Windows size regresses above `226 MB` or OCR fails, inspect only recent Windows-specific changes first. The Linux/macOS fixes were gated by OS to avoid that.

## 7. Repro / validation commands

Local Linux:

```bash
cargo xtask build ocr
app/binaries/paddle-ocr-x86_64-unknown-linux-gnu/ocr-engine /path/to/image.png
python sidecars/paddle-ocr/scripts/measure_runtime_size.py \
  --input app/binaries/paddle-ocr-x86_64-unknown-linux-gnu \
  --preserve-symlinks
```

macOS CI:

1. Run workflow `OCR macOS Sidecar Smoke`.
2. Check step `Run sidecar CLI and print JSON` for OCR output.
3. Download artifact `ocr-macos-smoke-artifacts` for generated input/output.
