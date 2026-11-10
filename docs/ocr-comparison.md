# OCR comparison — 2026-11-10

The local OCR fork is not identical to the LTS implementation. Its core package versions and output schema match, but its dependency installation, frozen runtime, and inference behavior differ. The reported SciPy failure has a concrete explanation in the local build artifacts; changing runners alone would not reconcile these implementations.

This review reads the LTS source checkout at `/home/a7md/@squigit/squigit` (`39a6aa9`) and the distribution checkout at `/home/a7md/@squigit/distribution` (`8a06782`). Those checkout revisions are not proof of the source revision used for the installed APT release. No OCR executable, test, or interactive validation was run.

## Reported failure

The local environment contains SciPy 1.18.1 and the file `scipy/_external/array_api_compat/numpy/fft.py`. That package's `__init__.py` loads the module dynamically with `__import__(__package__ + ".fft")`.

The installed PyInstaller 6.19.0 hook adds `scipy._lib.array_api_compat.numpy.fft` as a hidden import. SciPy in this environment uses `_external` instead of `_lib`. The local spec collects Paddle, PaddleOCR, and PaddleX modules, but does not explicitly collect this dynamically imported SciPy module.

Reading `native/ocr/build/squigit-ocr/PYZ-00.toc` confirms that the archive contains `scipy._external.array_api_compat.numpy` but omits `scipy._external.array_api_compat.numpy.fft`. This matches the missing-module error reported from the overlay. It is a frozen dependency omission, not evidence of a Qt or Rust JSON protocol problem.

Adding the missing hidden import would address this particular omission. It would not restore the other LTS packaging changes or establish that the remaining frozen runtime works. The application continues to use the installed LTS.

## Implementation differences

| Area                    | LTS checkout                                                                                                                                    | Local fork                                                                                                      |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Core versions           | PaddlePaddle 3.3.0, PaddleOCR 3.4.0, PaddleX 3.4.2, PyInstaller 6.19.0                                                                          | Same pins                                                                                                       |
| Dependency installation | Installs the three OCR packages with `--no-deps`, then an explicit trimmed runtime list; uses headless OpenCV                                   | Installs `paddlex[ocr]` with its full dependency graph, including SciPy and scikit-learn                        |
| Runtime patches         | Applies four patches for frozen Paddle library paths, optional model hosters, trimmed OCR dependency checks, and lazy PDF reader initialization | Applies none of these patches                                                                                   |
| PyInstaller inputs      | Explicit Paddle libraries, package source data, selected metadata, and PaddleOCR submodules; excludes several unused packages                   | Broad `collect_all` for Paddle, PaddleOCR, and PaddleX plus recursive dependency metadata; different exclusions |
| Loader setup            | Re-executes with bundled Paddle library paths and preloads selected native libraries; handles Windows library lookup                            | Relies on PyInstaller's collected libraries and loader environment                                              |
| Models                  | Detection, English recognition, and textline orientation                                                                                        | Detection and English recognition only                                                                          |
| Textline orientation    | Enabled by default; optional CLI toggle                                                                                                         | Disabled                                                                                                        |
| Large images            | Resizes images above a 2048-pixel longest side, then rescales returned coordinates                                                              | Passes the original image to `predict`                                                                          |
| Inference options       | Language and model-directory CLI arguments; model-name aliases                                                                                  | Fixed English models and CPU device                                                                             |
| Result parsing          | Normalizes multiple result forms, including older formats                                                                                       | Reads PaddleOCR 3.x `rec_texts`, `rec_scores`, and `rec_polys` directly                                         |
| JSON contract           | Array of `{text, confidence, box}` or an error object                                                                                           | Same contract; UTF-8 output and a trailing newline                                                              |
| Rust orchestration      | Async runtime, storage integration, model override support, scheduling priorities, and package discovery                                        | Small on-demand process runner with a two-minute limit and cancellation when Qt closes                          |

The smaller Rust runner and absence of storage/model-management code fit the new screenshot proof of concept. The omitted packaging fixes are a separate concern. Older result-format compatibility should not be copied into this refactor.

Sources: [local requirements](../native/ocr/requirements.txt), [local spec](../native/ocr/squigit-ocr.spec), [local inference](../native/ocr/src/main.py), [local builder](../xtask/src/ocr.rs), [LTS builder](/home/a7md/@squigit/squigit/squigit-ocr/build.rs), [LTS spec](/home/a7md/@squigit/squigit/squigit-ocr/native/ocr-engine.spec), [LTS inference](/home/a7md/@squigit/squigit/squigit-ocr/native/src/engine.py).

## Distribution differences

The distribution workflow dispatches the source repository's OCR build matrix and downloads its completed runtime artifacts. The matrix selects Python 3.12, builds the frozen runtime, and validates it before upload. The distribution pipeline packages those artifacts and includes additional installed-package checks. These checks were inspected, not executed here.

The Linux packaging script copies the built runtime into `/usr/lib/squigit-ocr` and installs a `/usr/bin/squigit-ocr` wrapper that sets `LD_LIBRARY_PATH` before executing it. It does not rebuild the Python application inside the Debian package. The current PATH-based fallback invokes that wrapper.

Both pipelines pin the main OCR stack, but their entire dependency graphs are not locked. Environment drift is still possible. Here, however, there are already substantive source and build differences, and the local archive exposes the reported missing module directly.

Sources: [source build matrix](/home/a7md/@squigit/squigit/.github/workflows/ocr-build-matrix.yml), [distribution workflow](/home/a7md/@squigit/distribution/.github/workflows/ocr-release.yml), [Linux package builder](/home/a7md/@squigit/distribution/.github/actions/squigit-ocr-linux-packaging/build.sh).

Before restoring bundled OCR, align the image-only dependencies and frozen-loader patches with the LTS, retain the current PaddleOCR 3.x JSON interface, and hand the rebuilt sidecar to the user for runtime validation. A one-module patch alone is not enough evidence to switch back.
