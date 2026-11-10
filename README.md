# Squigit

Squigit is a screenshot-first desktop proof of concept built with Rust, Rune, Qt 6, and PaddleX. It starts in the system tray with no home page.

- **Take screenshot** starts the Qt overlay. Draw a squiggle or select a rectangle; the selection stays open with **Save** and **OCR** actions.
- **Save** writes a PNG to `Pictures/squigit` and closes the overlay. Tray captures then show a native **Image saved** notification with the filename, an image preview, and an **Open in folder** action. Clicking the notification also reveals the image.
- **OCR** asks Rust to run the installed `squigit-ocr` LTS from PATH for that selection. Qt receives JSON and places transparent, selectable text over the frozen screen. Drag within a text line and press Ctrl+C to copy it; Ctrl+Shift+C copies all recognized text. Escape cancels the overlay.
- **Open savepath** opens `Pictures/squigit` in the OS file manager.
- **Configuration** opens a framed Rune window inspired by `rune-settings`. Its controls only edit a temporary demo config at `<temp>/squigit-settings-demo-<pid>/config.toml`; they do not configure capture or OCR.
- **About** opens a separate Rune dialog. **Quit** stops the resident application and its active capture.

Only Rust stays resident. Qt and OCR run on demand. OCR temporarily uses the published LTS while the local Python fork's frozen SciPy dependency is repaired. Install the LTS so `squigit-ocr` is on PATH; normal builds skip the local OCR fork. There is no chat, history, model manager, or persistent OCR worker.

## Layout

```text
squigit/                 Rust application, tray, Rune windows, product icons
crates/squigit-qt/        Rust Qt process orchestration and capture sandbox
crates/squigit-ocr/       Rust OCR process runner and JSON result types
native/qt/               C++/QML screenshot overlay and OS grabbers
native/ocr/              Python OCR CLI, PyInstaller spec, model preparation
xtask/                   Build, deploy, package, run, format, doctor
```

The executable identities are `squigit`, `squigit-qt`, and `squigit-ocr`. Only `squigit` has a desktop entry.

## Build and run

Install Rust stable, a C++17 toolchain, CMake, Qt 6 with Quick/QML and Qt5Compat, the `squigit-ocr` LTS, and the platform's Qt deployment tool (`linuxdeployqt`, `macdeployqt`, or `windeployqt`). Linux also needs fontconfig, Wayland, and xkbcommon development libraries for Rune. Set `Qt6_DIR` or `CMAKE_PREFIX_PATH` when Qt is outside the system prefix. Building the local OCR fork separately requires Python 3.10–3.12 with venv/pip; `SQUIGIT_PYTHON` selects that build interpreter.

```sh
cargo xtask build          # Build the application and bundle Qt; use installed OCR LTS
cargo xtask run            # Start the tray application
cargo xtask build --qt     # Build only native/qt/build
cargo xtask build --ocr    # Build the local OCR fork separately (currently unused)
cargo xtask doctor         # Check the Rust workspace and configure Qt
cargo xtask fmt            # Format changed files
```

The explicit `--ocr` build installs pinned Paddle/PaddleX/PaddleOCR and PyInstaller versions into `native/ocr/.venv`, prepares two models, and freezes the local CLI. Its output is currently excluded from the application package and is not used for OCR requests.

```text
binaries/<host-triple>/
├── squigit
├── squigit.desktop        Linux desktop entry
├── squigit.png
└── _internal/
    └── qt/                Qt deployment output, including squigit-qt
```

Keep the entire directory together. Qt is resolved relative to the application executable. OCR temporarily resolves the installed `squigit-ocr` command through PATH, including its launcher and library environment.

For direct capture and window development:

```sh
cargo xtask run -- --traditional --output-dir /tmp/shots
cargo xtask run -- --squiggle
cargo xtask run -- --configuration
cargo xtask run -- --about
```

The pinned Rune tray backend supports Linux and Windows. Linux needs a StatusNotifier host (GNOME AppIndicator extension, KDE, or a compatible panel). Rune's pinned revision does not yet provide a macOS tray backend; the native capture backend remains available there.

## Save notifications

The Rust application owns notifications through `notify-rust` 4.19. The action uses `opener::reveal` to show the selected image in Nautilus, Finder, Explorer, or the system file manager. Notification errors are reported to stderr without changing the successful save result. Action listeners run off the capture and UI threads, so another capture can start immediately. They remain available while the tray application is running. Direct one-shot CLI captures continue to print the saved path and exit without a notification.

Platform limits:

- Linux uses the desktop's notification service over D-Bus. Action buttons, image previews, and banner duration depend on that service. If file selection is unavailable, `opener` opens the containing folder.
- Windows uses native toasts with the Squigit AppUserModelID, registered under the current user with the product name and icon. Notification permissions and Focus settings can suppress banners. Actions require the application to remain running; activation after quitting is not registered.
- macOS uses the crate's default `NSUserNotificationCenter` backend, which Apple has deprecated. It requires the main run loop for action callbacks, supplied by Rune. The modern `UNUserNotificationCenter` backend needs a macOS app bundle and is not enabled in this proof of concept. The pinned Rune tray limitation above still applies.

## Process protocol

Qt writes capture markers and newline-delimited OCR requests to stdout; diagnostics go to stderr. Rust owns both sidecar processes. Each OCR click creates one OCR process and Rust replies to Qt on stdin:

```text
Qt → Rust: OCR_REQUEST:{"id":"0-1","image":"/tmp/squigit-ocr-xxxxxx.png"}
Rust → Qt: {"id":"0-1","boxes":[{"text":"Hello","confidence":0.99,"box":[[1,2],[50,2],[50,20],[1,20]]}]}
Rust → Qt: {"id":"0-1","error":"An actionable OCR failure"}
```

Boxes use physical pixels relative to the selected image. Qt applies the selection offset, display pixel ratio, text width, and rotation. Temporary selection files belong to Qt and are removed after a response or when the overlay exits. OCR is cancelled when the overlay closes and has a two-minute limit.

`SQUIGIT_SAVED:<absolute path>` reports a successful save; direct capture prints just that path and exits 0. Cancellation exits 2; failure exits 1. The existing permission and audio suppression markers remain part of the capture protocol. `SQUIGIT_OUTPUT_DIR` is passed by Rust from `--output-dir` when provided.
