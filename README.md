# Squigit

Squigit is a screenshot-first desktop proof of concept built with Rust, Rune, and Qt 6. It starts in the system tray with no home page.

- **Show menu** reveals the resident Qt radial menu; **Hide menu** hides it immediately. Arrow keys and Tab cycle petals, Shift+Tab reverses, and Enter or Space activates the selected action. A bundled tick plays when the highlighted petal changes.
- **Snapshot** opens a traditional selection. After release, only the chosen display stays frozen, with a movable, resizable crop and the shared annotation dock. Export writes the annotated crop to `Pictures/squigit`, closes the overlay, and shows the saved-image notification. Its **Export as a copy** submenu action keeps the overlay open and confirms with a check icon, without a notification or sound.
- **Ask Google** uses the same selection, then shows a floating prompt composer centered at the bottom of the chosen display, layered above the crop. Sending saves the annotated final crop to app data `blob_storage` and closes the overlay. Rust uploads the image through the selected hosting provider in the background, then opens Google AI Mode with the image and prompt in the default browser.
- **Circle to Search** uses squiggle selection and saves automatically to app data `blob_storage`, without buttons. The overlay closes before the background image upload, then Google Lens opens in the default browser.
- **OCR** asks Rust to run the embedded `squigit-ocr` sidecar on the whole chosen display's frozen image. Text inside the crop is highlighted blue and selectable; text outside is highlighted grey. Moving or resizing the crop updates which characters are selectable without rescanning. The first OCR click scans, or retries a failed or cancelled scan. During recognition, **Cancel OCR** stops the entire OCR process tree while keeping the overlay open. After a successful scan, the button toggles all OCR highlights and text selection. Drag within a blue text line and press Ctrl+C to copy it; Ctrl+Shift+C copies text inside the crop while OCR is visible. Tapping Escape clears selected OCR text; holding Escape for 650 ms closes the overlay. Clicking blank space or moving/resizing the crop also clears text selection. OCR text selection keeps keyboard focus in the overlay so its shortcuts stay available. Keyboard input follows the display where selection begins; the other displays return to their live contents.
- **Scan QR** automatically scans the selected crop locally in Rust using [rqrr](https://github.com/WanzenBug/rqrr), without buttons. The first decoded HTTP or HTTPS URL closes the overlay and opens in the default browser. A missing QR code or a code containing something other than a web URL shows an error in the overlay; resize the crop to retry. Its **Ask Google?** fallback closes the overlay and uses the shared image upload and Google AI Mode flow with the prompt `scan this qr code if found`. Local QR scanning does not start OCR or use a network service.
- **Open savepath** opens `Pictures/squigit` in the OS file manager.
- **Configuration** opens the Rune **General** page, with Play sounds, the installed OCR model selector, Open on startup, and Clear storage with its reclaimable size. **OCR Models** manages downloads with Download, Cancel, hold-to-Delete, and actual transfer progress. English is bundled; Latin, Cyrillic, Korean, CJK, and Devanagari can be downloaded. The selected recognition model applies to the next scan. **Image hosting** selects the provider and manages API keys for image search. Captures default to Traditional with Auto OCR enabled; `--squiggle` and `--no-auto-ocr` provide capture overrides.
- **About** opens a separate Rune dialog. **Quit** stops the resident application and its active capture.

Rust and Qt stay resident; the radial menu starts hidden, and actions switch UI inside the existing Qt process. Each capture briefly fades in a hint below the top edge to press and hold Esc to exit; hovering the top-center edge drops down an exit button. A short Esc tap replays the animated hint; holding Esc for 650 ms exits directly without replaying it, even if OCR text is selected. OCR runs on demand. Release builds compress the complete OCR runtime, including its default models and libraries, into the Rust executable and extract it atomically into a content-addressed cache when needed. Debug builds use the fully built runtime staged under `binaries` directly. OCR never searches PATH or uses an installed package. There is no chat, history, or persistent OCR worker.

Snapshot and Ask Google share one annotation editor after region selection. Draw on the full frozen display; moving or resizing the crop changes only the final exported area. Drag the crop border to move it and its handles to resize it. The dark dock provides Select (V), Brush (B), Eraser (E), Shapes (R), Counter (C), Text (T), Blur (X), and Highlighter (H). Right-click a dock tool for its variants or formatting. Shapes include rectangles, arrows, lines, circles, and solid shapes; the eraser offers brush erasing, whole-object erasing, and Clear all. Highlighter also offers a translucent solid rectangle. Tool variant menus mark the current choice with a check and highlighted row. The counter icon shows the next number; deleting or cutting counters renumbers those remaining, and pasted counters receive new numbers at the end. Brush smoothing is fixed at 12% while drawing and 50% on release; blur and highlighter use raw input.

Use Select (V) to select annotations, Ctrl-click to add or remove objects, and drag selected objects to move the group. Drawing tools draw over existing objects without moving them. Selection handles resize and rotate objects or groups. Newly drawn shapes and text boxes show handles immediately; their tools can resize or rotate the current object while clicks elsewhere create a new one. Rotation uses the standard four-arrow cursor, and resize handles use the same cursors as the crop box. Brush-style tools hide the system pointer on hover and show only the size outline. Lines and arrows have two endpoint handles; rectangles have inner corner-radius handles. Double-click text to edit it. Backspace deletes selected objects; Ctrl+C, Ctrl+X, and Ctrl+V copy, cut, and paste annotation objects. Right-click selected objects for Delete, Copy, Cut, and Format; right-click blank space with Select active for Paste and Select all. Ctrl+A selects all annotation objects and switches to Select when a text editor is not focused. Format opens a floating color spectrum, hue slider, hex field, and tool-size slider. Solid shapes and groups omit the size slider; choosing a color recolors the entire selection. Text boxes and the Ask Google composer share a text context menu, with Format added for annotation text.

Ctrl+Shift+C copies the annotated crop. Ctrl+S exports it to `Pictures/squigit`, closes the editor, and shows the saved-image notification. Their dock submenus also copy or export the original crop. **Export as a copy** saves the annotated crop to the same folder, keeps the editor open, and briefly changes the export icon to a check without a notification or sound. Ctrl+Z undoes an annotation edit; Ctrl+Shift+Z redoes it. Send writes the annotated crop to `blob_storage`. Selection borders, handles, the dock, and other controls never appear in output.

Rune initializes the shared app data directory on launch. Settings use `config.toml`, and additional downloaded models use `models/` inside that directory. Missing directories or configuration are recreated on launch, when reopening settings, and when accessing models. `SQUIGIT_CONFIG_DIR` can override the app data directory.

| Platform | App data directory                                 |
| -------- | -------------------------------------------------- |
| Linux    | `$XDG_CONFIG_HOME/squigit`, or `~/.config/squigit` |
| Windows  | `%APPDATA%\Squigit`                                |
| macOS    | `~/Library/Application Support/Squigit`            |

`config.toml` contains the sound preference, recognition model, hosting provider, and custom instance URL. Writes are atomic and synchronized across processes.

```toml
[general]
play_sounds = true

[ocr]
selected_model = "pp-ocr-v5-en"

[hosting]
provider = "imgbb"
chevereto_url = ""
```

**Play sounds** controls both menu ticks and notification alerts without restarting. Changes in the tray's configuration window update the in-memory preference immediately; a separate configuration process is picked up by a background monitor within 250 ms. Playback does not read configuration files. Both embedded sounds are decoded once and share one bounded audio worker and output stream. The worker detects backend errors, stalled callbacks, and default-device changes, then recreates output with a bounded retry delay. Stale feedback is discarded, and failures use rate-limited stderr diagnostics. Notification alerts are queued only after the OS accepts the notification.

**Open on startup** registers the current executable for the current user's OS login: an XDG autostart entry on Linux, a LaunchAgent on macOS, or a Run registry value on Windows. It starts Squigit in the tray with its windows and radial menu hidden. The switch reads the OS registration directly; it is off until enabled.

**Clear storage** measures and removes the app's Qt disk cache, extracted Rust OCR runtime cache, and local `blob_storage` contents on a background thread. The button shows the reclaimable size and refreshes it after cleanup. Saved screenshots, downloaded OCR models, configuration, and encrypted API keys are preserved. Active OCR and image uploads hold shared storage leases; cleanup reports an active operation instead of deleting its files. Deleted runtime caches are recreated on demand. Debug builds keep the staged OCR runtime under `binaries` intact.

Image hosting stores each provider's key in `auth.json` under the same app data root, encrypted with AES-256-GCM and a separately salted, provider-specific HKDF-SHA256 key. The random encryption master stays in the OS credential vault: Secret Service on Linux, Credential Manager on Windows, and Keychain on macOS. There are no accounts, profiles, SQLite databases, hosted-URL caches, or image deduplication. Each submission uploads its current capture.

**Configuration → Image hosting** has exactly three provider rows, in this order: **ImgBB**, **ImageKit**, **Custom / Chevereto**. ImgBB is selected by default. Each row has a two-line info tooltip, a setup link, and its own masked API key field. Chevereto also asks for the instance's HTTPS base URL. Its encrypted key is bound to that endpoint; changing the instance clears its saved key. Select a row to use its saved credentials; saving a new key selects that provider automatically. Uploads never silently fall back to a different provider.

ImgBB uses its 32-character API key and requests automatic deletion after ten minutes. ImageKit uses a private API key, uploads with `isPrivateFile=true`, and gives Google a signed URL valid for ten minutes; the original remains in the user's ImageKit account until deleted. Chevereto uses `X-API-Key` at the configured instance's `/api/1/upload` endpoint and requests `expiration=PT10M`; the instance controls deletion. See the [ImgBB API](https://api.imgbb.com/), [ImageKit signed URL documentation](https://imagekit.io/docs/media-delivery-basic-security), and [Chevereto upload API](https://v4-docs.chevereto.com/api/1/file-upload.html).

Press Enter or leave a key field to save it. The configured key stays masked; the eye button requires Polkit authentication on Linux, Windows Hello on Windows, or Touch ID/system authentication on macOS before revealing it. Successful authentication grants ten minutes of repeat reveals across the providers without another prompt. This fixed grace period starts at authentication, does not extend on repeat reveals, and resets when the app restarts. Leaving the field, changing pages, or hiding the window clears revealed text while retaining the grace period. Each trash button removes only that provider's saved key.

All three online image actions share one background upload and URL-construction path. Upload or browser-launch failures produce an OS notification. Each app launch checks the selected provider's configuration in the background. Missing credentials or an invalid custom endpoint produce a persistent **Set API** notification that opens configuration directly to **Image hosting**, including on a new installation. Every image action attempted without valid setup sends a fresh notification, even while an earlier notice remains uncleared. Each warning uses a random hidden action identifier, plus event metadata on Linux; the visible text has no counter or identifier. The startup check does not decrypt keys or prompt for vault access. Google Lens receives the hosted image URL; Ask Google and the QR fallback additionally request AI Mode (`udm=50`) and include the encoded prompt.

Linux key reveal needs a running desktop Secret Service and Polkit agent, plus Squigit's policy installed once:

```sh
sudo sh squigit/resources/polkit/install-policy.sh
```

Packaged Linux releases include the same installer at `polkit/install-policy.sh` beside the executable. Installation copies only Squigit's reveal policy to `/usr/share/polkit-1/actions/org.squigit.policy`.

## Layout

```text
squigit/                 Rust application, tray, Rune windows, product icons
crates/squigit-qt/        Rust Qt orchestration, local QR scanner, capture sandbox
crates/squigit-ocr/       Rust OCR runner, embedded runtime, model downloads and selection
squigit/src/storage.rs   App data paths and persistent TOML settings
squigit/src/auth.rs      Encrypted provider keys and native reveal authentication
squigit/src/image_hosting.rs  ImgBB, ImageKit, and Chevereto upload adapters
squigit/src/image_search.rs  Background image search and Google URL construction
native/qt/               C++/QML screenshot overlay and OS grabbers
native/ocr/              Forked Python OCR runtime, patches, models and build scripts
xtask/                   Build, deploy, package, run, format, doctor
```

The application builds `squigit`, its private `squigit-qt` helper, and the forked `squigit-ocr` runtime. Release builds embed OCR; debug builds reuse the complete staged runtime. Only `squigit` has a desktop entry.

## Build and run

Install Rust stable, a C++17 toolchain, CMake, and Qt 6.4 or newer with Quick/QML and Qt5Compat. Release packaging additionally needs the platform's Qt deployment tool (`linuxdeployqt`, `macdeployqt`, or `windeployqt`); local debug does not deploy Qt. Linux Rust builds need ALSA development headers for the radial-menu tick. OCR builds need Python 3 with venv/pip; CI uses Python 3.12. The QML components use [bound component contexts](https://doc.qt.io/qt-6/qtqml-documents-structure.html#componentbehavior), available since Qt 6.4. Linux also needs fontconfig, Wayland, and xkbcommon development libraries for Rune. Set `Qt6_DIR` or `CMAKE_PREFIX_PATH` when Qt is outside the system prefix.

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
├── polkit/                 Linux reveal policy and installer
└── _internal/
    └── qt/                Qt deployment output, including squigit-qt
```

Keep the entire directory together. Qt is resolved relative to the application executable. Release executables contain OCR; no installed OCR package or `_internal/ocr` directory is used. The release extraction cache is `squigit/ocr/<runtime-hash>` under the OS cache directory.

Ubuntu and Debian system Qt installs also need the QML controls and templates modules for the annotation editor:

```sh
sudo apt install qml6-module-qtquick-controls qml6-module-qtquick-templates
```

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

Boxes use physical pixels relative to the entire frozen image of the chosen display. Qt applies the display pixel ratio, text width, and rotation without adding a crop offset. Character centers determine crop membership, using measured text advances and grapheme boundaries so partial lines can become selectable as the crop expands. Temporary full-display files belong to Qt and are removed after a response or when the overlay exits. OCR and QR use the original frozen image without annotation decorations. Snapshot and Ask Google flatten annotations before cropping. OCR is cancelled when the overlay closes and has a two-minute limit. The chosen overlay alone enables application shortcuts for Escape, copy, and copy-all. Text editors keep mouse selection visible without taking keyboard focus; the overlay invokes the selected editor's copy method. Native focus and keyboard grabs change only after the drawing release has finished; resizing does not repeat that handoff. The selected display acquires its keyboard grab before the other overlays are hidden, without releasing the existing grab first. A failed transfer is reported to stderr and keeps the other overlays visible.

The overlay uses normal window-manager click activation. After a crop exists, clicking anywhere in the Qt window also refreshes its native activation and keyboard grab before the click reaches QML. Clicking back from another display therefore restores input across text, controls, handles, and blank areas without requiring an OCR visibility toggle.

`SQUIGIT_SAVED:<absolute path>` reports a successful save; direct capture prints just that path and exits 0. Cancellation exits 2; failure exits 1. The existing permission and audio suppression markers remain part of the capture protocol. `SQUIGIT_OUTPUT_DIR` is passed by Rust from `--output-dir` when provided.
