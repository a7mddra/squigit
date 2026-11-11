# Squigit

Squigit is a screenshot-first desktop proof of concept built with Rust, Rune, and Qt 6. It starts in the system tray with no home page.

- **Show menu** reveals the resident Qt radial menu; **Hide menu** hides it immediately. Arrow keys and Tab cycle petals, Shift+Tab reverses, and Enter or Space activates the selected action. A bundled tick plays when the highlighted petal changes.
- **Snapshot** opens a traditional selection. After release, only the chosen display stays frozen, with a movable, resizable crop and a **Save** button. Save writes the final crop to `Pictures/Squigit` and closes the overlay. A native **Image saved** notification shows the filename, an image preview, and an **Open in folder** action. Clicking the notification also reveals the image.
- **Ask Google** uses the same selection, then shows a floating prompt composer centered at the bottom of the chosen display, layered above the crop. Sending saves the final crop to app data `blob_storage`, prints the prompt, and closes the overlay.
- **Circle to Search** uses squiggle selection and saves automatically to app data `blob_storage`, without buttons.
- **OCR** asks Rust to run the embedded `squigit-ocr` sidecar on the whole chosen display's frozen image. Text inside the crop is highlighted blue and selectable; text outside is highlighted grey. Moving or resizing the crop updates which characters are selectable without rescanning. The first OCR click scans, or retries a failed or cancelled scan. During recognition, **Cancel OCR** stops the entire OCR process tree while keeping the overlay open. After a successful scan, the button toggles all OCR highlights and text selection. Drag within a blue text line and press Ctrl+C to copy it; Ctrl+Shift+C copies text inside the crop while OCR is visible. Escape first clears selected OCR text; another press closes the overlay. Clicking blank space or moving/resizing the crop also clears text selection. OCR text selection keeps keyboard focus in the overlay so its shortcuts stay available. Keyboard input follows the display where selection begins; the other displays return to their live contents.
- **Scan QR** automatically scans the selected crop locally in Rust using [rqrr](https://github.com/WanzenBug/rqrr), without buttons. The first decoded HTTP or HTTPS URL closes the overlay and opens in the default browser. A missing QR code or a code containing something other than a web URL shows an error in the overlay; resize the crop to retry. QR scanning does not start OCR or use a network service.
- **Open savepath** opens `Pictures/Squigit` in the OS file manager.
- **Configuration** opens the Rune **OCR Models** page. Its model rows match the desktop model downloader, with Download, Cancel, hold-to-Delete, actual transfer progress, and an installed-model selector. English is bundled; Latin, Cyrillic, Korean, CJK, and Devanagari can be downloaded. The selected recognition model applies to the next scan. Captures default to Traditional with Auto OCR enabled. The former settings pages and capture preferences are removed; `--squiggle` and `--no-auto-ocr` provide capture overrides.
- **About** opens a separate Rune dialog. **Quit** stops the resident application and its active capture.

Rust and Qt stay resident; the radial menu starts hidden, and actions switch UI inside the existing Qt process. Each capture briefly fades in a hint below the top edge to press and hold Esc to exit; hovering the top-center edge reveals an exit button. Holding Esc cancels the active action even if OCR text is selected. OCR runs on demand. Release builds compress the complete OCR runtime, including its default models and libraries, into the Rust executable and extract it atomically into a content-addressed cache when needed. Debug builds use the fully built runtime staged under `binaries` directly. OCR never searches PATH or uses an installed package. There is no chat, history, or persistent OCR worker.

Rune initializes the shared app data directory on launch. Settings use `config.toml`, and additional downloaded models use `models/` inside that directory. Missing directories or configuration are recreated on launch, when reopening settings, and when accessing models. `SQUIGIT_CONFIG_DIR` can override the app data directory.

| Platform | App data directory                                 |
| -------- | -------------------------------------------------- |
| Linux    | `$XDG_CONFIG_HOME/squigit`, or `~/.config/squigit` |
| Windows  | `%APPDATA%\Squigit`                                |
| macOS    | `~/Library/Application Support/Squigit`            |

The current configuration contains only the recognition model selection. Writes are atomic and synchronized across processes; later storage features share the same app data root.

```toml
[ocr]
selected_model = "pp-ocr-v5-en"
```

## Layout

```text
squigit/                 Rust application, tray, Rune windows, product icons
crates/squigit-qt/        Rust Qt orchestration, local QR scanner, capture sandbox
crates/squigit-ocr/       Rust OCR runner, embedded runtime, model downloads and selection
squigit/src/storage.rs   App data paths and persistent TOML settings
native/qt/               C++/QML screenshot overlay and OS grabbers
native/ocr/              Forked Python OCR runtime, patches, models and build scripts
xtask/                   Build, deploy, package, run, format, doctor
```

The application builds `squigit`, its private `squigit-qt` helper, and the forked `squigit-ocr` runtime. Release builds embed OCR; debug builds reuse the complete staged runtime. Only `squigit` has a desktop entry.

## Build and run

Install Rust stable, a C++17 toolchain, CMake, and Qt 6.4 or newer with Quick/QML, Qt Multimedia, and Qt5Compat. Release packaging additionally needs the platform's Qt deployment tool (`linuxdeployqt`, `macdeployqt`, or `windeployqt`); local debug does not deploy Qt. On Ubuntu, the menu tick requires `qt6-multimedia-dev`. OCR builds need Python 3 with venv/pip; CI uses Python 3.12. The QML components use [bound component contexts](https://doc.qt.io/qt-6/qtqml-documents-structure.html#componentbehavior), available since Qt 6.4. Linux also needs fontconfig, Wayland, and xkbcommon development libraries for Rune. Set `Qt6_DIR` or `CMAKE_PREFIX_PATH` when Qt is outside the system prefix.

```sh
cargo xtask build          # Build OCR, Qt and Rust; package the application
cargo xtask run            # Run target/debug/squigit directly
cargo xtask run --release  # Run the packaged release application
cargo xtask build --debug  # Incremental Qt + Rust, reuse fully built OCR
cargo xtask stage-qt       # Stage only release Qt for CI/packaging
cargo xtask build --app    # Build Rust and package already-staged OCR and Qt
cargo xtask build --ocr    # Build and stage the native OCR runtime
cargo xtask doctor         # Check the Rust workspace and configure Qt
cargo xtask fmt            # Format changed files
```

```text
binaries/<host-triple>/
├── squigit
├── squigit.desktop        Linux desktop entry
├── squigit.png
└── _internal/
    └── qt/                Qt deployment output, including squigit-qt
```

Keep the entire directory together. Qt is resolved relative to the application executable. Release executables contain OCR; no installed OCR package or `_internal/ocr` directory is used. The release extraction cache is `squigit/ocr/<runtime-hash>` under the OS cache directory.

For iteration, run `cargo xtask build --debug`, then `cargo xtask run`. This builds Qt and Rust incrementally, then runs `target/debug/squigit` with the `development` feature. Rust reads CMake's `native/qt/build-debug/runtime-Debug.txt` to locate the Qt executable and the matching installed Qt kit's libraries, plugins, and QML modules. It runs the helper directly from the CMake build folder, including its app bundle on macOS and Debug configuration on Windows. Debug mode performs no deployment, packaging, executable copying, or library copying.

The complete prebuilt OCR tree is still required at `binaries/paddle-ocr-<host-triple>`; build it once with `cargo xtask build --ocr` if missing. Debug mode uses that tree directly, avoiding Python rebuilding, compression, embedding, and extraction. It never uses an installed OCR package. Release builds still deploy Qt and embed the full OCR runtime; use `cargo xtask run --release` to launch the packaged application.

Rust uses unoptimized incremental compilation with 256 codegen units and reduced debug information. Unix Qt Debug builds use Ninja when installed, with optimization disabled and line information retained. Separate debug and release caches use more disk space to avoid rebuilding when switching modes. Quit the running tray application before rebuilding, then restart it with `cargo xtask run`.

The OCR fork comes from `squigit/squigit-ocr` at `39a6aa9efa6d81e5e06493fee991c67db5550733`. Its native inputs are preserved: split requirements installed in the original order, all four patches, optional-package removal, pinned-version verification, the same three bundled models, the original PyInstaller spec, symlink-preserving staging, and the upstream runtime/executable and size-check scripts. The native build driver changes only the source directory to `native/ocr`; embedding follows native packaging. Qt building and deployment live in `crates/squigit-qt/build.rs`, loaded by xtask with Cargo auto-build disabled. Application orchestration and packaging live in `xtask/src/build.rs`. The `build-ocr` and `build-qt` composite GitHub Actions build these runtimes separately before the main action runs `cargo xtask build --app`. `SQUIGIT_OCR_PYI_MODE`, `SQUIGIT_OCR_RECREATE_VENV`, and `SQUIGIT_OCR_MEASURE_SIZE` retain their upstream behavior.

After `cargo xtask build --ocr`, a normal `cargo build -p squigit` embeds that staged runtime for development. Release builds reject missing OCR payloads; debug checks can compile before the native build, but OCR reports an explicit missing-runtime error until the application is rebuilt with its payload.

For direct capture and window development:

```sh
cargo xtask run -- --traditional --output-dir /tmp/shots
cargo xtask run -- --squiggle
cargo xtask run -- --traditional --no-auto-ocr
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

Qt writes capture markers and newline-delimited OCR and QR requests to stdout; diagnostics go to stderr. Rust owns both sidecar processes. Automatic recognition or the first OCR click creates one OCR process. Failed scans can be retried; successful results are cached for the lifetime of the frozen overlay, so visibility toggles and crop changes create no further processes. Rust replies to Qt on stdin:

```text
Qt → Rust: OCR_REQUEST:{"id":"0-1","image":"/tmp/squigit-ocr-xxxxxx.png"}
Rust → Qt: {"id":"0-1","boxes":[{"text":"Hello","confidence":0.99,"box":[[1,2],[50,2],[50,20],[1,20]]}]}
Rust → Qt: {"id":"0-1","error":"An actionable OCR failure"}
Qt → Rust: OCR_CANCEL:{"id":"0-1"}
Rust → Qt: {"id":"0-1","cancelled":true}
Qt → Rust: QR_REQUEST:{"id":"qr-0-1","image":"/tmp/squigit-qr-xxxxxx.png"}
Rust → Qt: {"id":"qr-0-1","url":"https://example.com/"}
Rust → Qt: {"id":"qr-0-1","error":"No QR code found in the selected region"}
Qt → Rust: QR_OPEN:{"url":"https://example.com/"}
```

QR uses the original frozen crop in physical pixels, without OCR or selection decorations. Its temporary PNG belongs to Qt and is removed after the response or when the overlay exits. Rust scans on a background thread using the local decoder, validates HTTP/HTTPS URLs, and returns the first valid URL. Qt emits `QR_OPEN` and exits; Rust stops capture workers and waits for Qt to close before opening the browser through `opener::open_browser`. Closing the overlay during a scan discards its result without opening a browser.

Cancellation replies arrive only after the OCR worker and its process tree have stopped. Qt ignores a racing scan result while waiting for that acknowledgement. A termination failure returns `{"id":"0-1","cancelled":false,"error":"..."}`. The runner uses [process-wrap](https://github.com/watchexec/process-wrap) with Unix process groups and Windows Job Objects to terminate descendants, including frozen Python subprocesses.

Boxes use physical pixels relative to the entire frozen image of the chosen display. Qt applies the display pixel ratio, text width, and rotation without adding a crop offset. Character centers determine crop membership, using measured text advances and grapheme boundaries so partial lines can become selectable as the crop expands. Temporary full-display files belong to Qt and are removed after a response or when the overlay exits. Save crops the original frozen image, without OCR decorations. OCR is cancelled when the overlay closes and has a two-minute limit. The chosen overlay alone enables application shortcuts for Escape, copy, and copy-all. Text editors keep mouse selection visible without taking keyboard focus; the overlay invokes the selected editor's copy method. Native focus and keyboard grabs change only after the drawing release has finished; resizing does not repeat that handoff. The selected display acquires its keyboard grab before the other overlays are hidden, without releasing the existing grab first. A failed transfer is reported to stderr and keeps the other overlays visible.

The overlay uses normal window-manager click activation. After a crop exists, clicking anywhere in the Qt window also refreshes its native activation and keyboard grab before the click reaches QML. Clicking back from another display therefore restores input across text, controls, handles, and blank areas without requiring an OCR visibility toggle.

`SQUIGIT_SAVED:<absolute path>` reports a successful save; direct capture prints just that path and exits 0. Cancellation exits 2; failure exits 1. The existing permission and audio suppression markers remain part of the capture protocol. `SQUIGIT_OUTPUT_DIR` is passed by Rust from `--output-dir` when provided.
