# Squigit

Squigit is a screenshot-first desktop proof of concept built with Rust, Rune, Qt 6, and PaddleX. It starts in the system tray with no home page.

- **Take screenshot** waits 300 ms for the tray menu to close, then starts the Qt overlay using the configured default capture type. Draw a squiggle or select a rectangle; it becomes a crop with resize handles and **Save** and **OCR** actions. Drag blank space inside the crop to move it.
- **Save** writes the current crop as a PNG to `Pictures/squigit` and closes the overlay. Tray captures then show a native **Image saved** notification with the filename, an image preview, and an **Open in folder** action. Clicking the notification also reveals the image.
- **OCR** asks Rust to run the installed `squigit-ocr` LTS from PATH on the whole chosen display's frozen image. Text inside the crop is highlighted blue and selectable; text outside is highlighted grey. Moving or resizing the crop updates which characters are selectable without rescanning. The first OCR click scans, or retries a failed or cancelled scan. During recognition, **Cancel OCR** stops the entire OCR process tree while keeping the overlay open. After a successful scan, the button toggles all OCR highlights and text selection. Drag within a blue text line and press Ctrl+C to copy it; Ctrl+Shift+C copies text inside the crop while OCR is visible. Escape first clears selected OCR text; another press closes the overlay. Clicking blank space or moving/resizing the crop also clears text selection. OCR text selection keeps keyboard focus in the overlay so its shortcuts stay available. Keyboard input follows the display where selection begins; the other displays return to their live contents.
- **Open savepath** opens `Pictures/squigit` in the OS file manager.
- **Configuration** opens a framed Rune window inspired by `rune-settings`. **General → Default capture type** chooses Squiggle or Traditional. **Auto OCR**, off by default, starts recognition silently on the first press that begins drawing a selection. OCR controls and status appear only after the selection is finished. Cancelling that scan leaves Auto OCR enabled for future captures. Both settings apply to the next tray capture and persist across restarts. The other controls remain demonstration options. Settings are saved to the OS config directory at `squigit/config.toml` (`~/.config/squigit/config.toml` on Linux by default). Direct CLI captures use their explicit flags, including `--auto-ocr` for automatic recognition.
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

Install Rust stable, a C++17 toolchain, CMake, Qt 6.4 or newer with Quick/QML and Qt5Compat, the `squigit-ocr` LTS, and the platform's Qt deployment tool (`linuxdeployqt`, `macdeployqt`, or `windeployqt`). The QML components use [bound component contexts](https://doc.qt.io/qt-6/qtqml-documents-structure.html#componentbehavior), available since Qt 6.4. Linux also needs fontconfig, Wayland, and xkbcommon development libraries for Rune. Set `Qt6_DIR` or `CMAKE_PREFIX_PATH` when Qt is outside the system prefix. Building the local OCR fork separately requires Python 3.10–3.12 with venv/pip; `SQUIGIT_PYTHON` selects that build interpreter.

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
cargo xtask run -- --traditional --auto-ocr
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

Qt writes capture markers and newline-delimited OCR requests to stdout; diagnostics go to stderr. Rust owns both sidecar processes. Automatic recognition or the first OCR click creates one OCR process. Failed scans can be retried; successful results are cached for the lifetime of the frozen overlay, so visibility toggles and crop changes create no further processes. Rust replies to Qt on stdin:

```text
Qt → Rust: OCR_REQUEST:{"id":"0-1","image":"/tmp/squigit-ocr-xxxxxx.png"}
Rust → Qt: {"id":"0-1","boxes":[{"text":"Hello","confidence":0.99,"box":[[1,2],[50,2],[50,20],[1,20]]}]}
Rust → Qt: {"id":"0-1","error":"An actionable OCR failure"}
Qt → Rust: OCR_CANCEL:{"id":"0-1"}
Rust → Qt: {"id":"0-1","cancelled":true}
```

Cancellation replies arrive only after the OCR worker and its process tree have stopped. Qt ignores a racing scan result while waiting for that acknowledgement. A termination failure returns `{"id":"0-1","cancelled":false,"error":"..."}`. The runner uses [process-wrap](https://github.com/watchexec/process-wrap) with Unix process groups and Windows Job Objects to terminate descendants, including frozen Python subprocesses.

Boxes use physical pixels relative to the entire frozen image of the chosen display. Qt applies the display pixel ratio, text width, and rotation without adding a crop offset. Character centers determine crop membership, using measured text advances and grapheme boundaries so partial lines can become selectable as the crop expands. Temporary full-display files belong to Qt and are removed after a response or when the overlay exits. Save crops the original frozen image, without OCR decorations. OCR is cancelled when the overlay closes and has a two-minute limit. The chosen overlay alone enables application shortcuts for Escape, copy, and copy-all. Text editors keep mouse selection visible without taking keyboard focus; the overlay invokes the selected editor's copy method. Native focus and keyboard grabs change only after the drawing release has finished; resizing does not repeat that handoff. The selected display acquires its keyboard grab before the other overlays are hidden, without releasing the existing grab first. A failed transfer is reported to stderr and keeps the other overlays visible.

`SQUIGIT_SAVED:<absolute path>` reports a successful save; direct capture prints just that path and exits 0. Cancellation exits 2; failure exits 1. The existing permission and audio suppression markers remain part of the capture protocol. `SQUIGIT_OUTPUT_DIR` is passed by Rust from `--output-dir` when provided.
