# Squigit — Agent & Contributor Guide

Welcome to **Squigit**, the native Qt 6 + Rust screenshot tool. This
document outlines the architecture boundaries, naming rules, build workflows,
and the runtime protocol that all AI agents and contributors must strictly
adhere to.

---

## 1. Repository Structure

```text
squigit/
├── squigit/              # Rust application, tray, Rune windows, icons
├── crates/
│   ├── squigit-qt/       # Rust Qt process wrapper and capture sandbox
│   └── squigit-ocr/      # Rust OCR runner, embedded runtime and model manager
├── native/
│   ├── qt/              # C++/QML overlay and platform grabbers
│   └── ocr/             # Forked Python OCR runtime, patches and build scripts
├── xtask/               # Build, deployment, packaging, and development tasks
├── Cargo.toml           # Workspace manifest
└── README.md
```

The only public application is `squigit`. It owns the tray, settings, about,
and distribution. `squigit-qt` is a private sidecar. The complete `squigit-ocr`
runtime is embedded in release executables and extracted to its cache on demand.
Debug builds use the complete prebuilt runtime at `binaries/paddle-ocr-<host>`
directly, without embedding, compression, or a Python rebuild.
Never resolve OCR through PATH, system package managers, or an installed package.

Do not add or run tests unless asked. Do not automate runtime or UI
validation; ask the user to validate interactive flows. Never use sub-agents.
Refactors replace old formats; do not introduce legacy compatibility paths.
Commit each completed milestone as work progresses. Keep commits focused and
avoid accumulating unrelated changes in the working tree.

---

## 2. Naming Rules

1. **Binary naming.** The delivered binary is `squigit` (Rust entrypoint). The
   Qt helper inside the deployed runtime is `squigit-qt` (`squigit-qt.app` bundle
   on macOS), so every user-visible name stays anchored on `squigit`.
2. **Rust naming.** Functions, modules, and variables are `snake_case`; types
   are `CamelCase`. Keep crate modules shallow.
3. **C++/CMake naming.** The CMake project is `SQUIGIT`, the target is
   `squigit-qt`, and the QML module URI is `SquigitCapture`.
4. **App identity.** Linux desktop file name: `squigit`; macOS bundle
   identifier: `io.github.a7mddra.squigit`; display/org name: `Squigit`.

---

## 3. Essential Commands & Development Workflows

Project tasks are orchestrated using `cargo xtask`:

Use `cargo xtask build --debug` for feature iteration. It incrementally builds
Qt Debug and Rust, then runs directly from their build outputs. Cargo reuses unchanged
Rust artifacts when only Qt changes. The complete OCR runtime must already be
built under `binaries`; this mode never invokes Python. Release builds are
reserved for an explicit packaging request.

Keep separate CMake caches in `native/qt/build-debug` and `native/qt/build`.
Debug mode must never invoke deployment tools or copy Qt libraries, plugins,
or executables. `target/debug/squigit` reads CMake-generated
`native/qt/build-debug/runtime-Debug.txt` and launches the helper directly from
the build tree, using that Qt kit's installed dependencies. Use Ninja when
available for a fresh Unix debug cache. Keep Rust incremental compilation
enabled. The `development` Cargo feature enables these build-tree paths and
the staged OCR runtime; release builds retain private deployed Qt and embedded
OCR.

```bash
# Full pipeline: OCR build -> CMake build -> Qt deploy -> cargo release build -> package
cargo xtask build

# Build Qt and Rust incrementally; use their build outputs and prebuilt OCR
cargo xtask build --debug

# Stage the release Qt runtime for CI and packaging
cargo xtask stage-qt

# Build Rust and package already-staged OCR and Qt
cargo xtask build --app

# Only build and stage the forked OCR runtime
cargo xtask build --ocr

# Execute target/debug/squigit; arguments after -- are forwarded to it
cargo xtask run
cargo xtask run -- --configuration

# Execute the packaged release binary
cargo xtask run --release

# Workspace checks: cargo check + cmake configure smoke test
cargo xtask doctor

# Format git-changed files only (rustfmt + Prettier + clang-format)
cargo xtask fmt

# Format everything
cargo xtask fmt --all

# Remove Cargo artifacts, Qt build/deployment caches, OCR build/dist, and binaries/
cargo xtask clean
```

> [!TIP]
> Always run `cargo xtask fmt` before committing and `cargo xtask doctor`
> before completing tasks, so Rust, Prettier, and native diagnostics stay
> clean.

Packaging output layout:

```text
binaries/<host-triple>/
├── squigit            # Rust binary with embedded OCR (squigit.exe on Windows)
└── _internal/         # private Qt deployment tree
```

Only release packaging updates this executable. `cargo xtask run --release`
executes `binaries/<host-triple>/squigit` and forwards the
remaining arguments (a single leading `--` is stripped).

---

## 4. Rust ↔ Qt Runtime Protocol

The Rust capture wrapper and the Qt runtime (`squigit-qt`) exchange single-line
markers on stdout. Keep both sides (`crates/squigit-qt/src/instance.rs`, `native/qt/src/`) in sync
whenever a marker changes. The wrapper forwards its verdict to the user.

| Marker                       | Direction | Meaning                                                                  |
| ---------------------------- | --------- | ------------------------------------------------------------------------ |
| `AUDIO_MUTE` / `REQ_MUTE`    | Qt → Rust | open a shutter-sound suppression scope (Linux Wayland capture path only) |
| `AUDIO_UNMUTE`               | Qt → Rust | close the suppression scope (Linux Wayland capture path only)            |
| `CAPTURE_AUTHORIZED`         | Qt → Rust | screenshot access request accepted                                       |
| `CAPTURE_PERMISSION_GRANTED` | Qt → Rust | headless permission probe passed                                         |
| `CAPTURE_PERMISSION_DENIED`  | Qt → Rust | headless permission probe failed                                         |
| `CAPTURE_CANCELLED`          | Qt → Rust | the user dismissed the overlay (exit code 2)                             |
| `CAPTURE_FAIL`               | Qt → Rust | grab/crop/save failed (exit code 1)                                      |
| `SQUIGIT_SAVED:<abs path>`   | Qt → Rust | the PNG was written; the wrapper prints `path`                           |
| `menu_tick`                  | Qt → Rust | play one radial-menu navigation tick                                     |

OCR uses `OCR_REQUEST:{"id":"...","image":"<path>"}` from Qt and one
newline-delimited JSON response on Qt stdin, with the same `id` and either
`boxes` or `error`. Rust extracts and starts the embedded `squigit-ocr` sidecar
for each request. The selected recognition model is read for each new scan and
passed as `--rec-model-dir`; English uses the bundled default. Never fall back to
an installed OCR package. The OCR CLI prints one JSON array of
`{text, confidence, box}` on success,
or an `{error}` object on failure. Coordinates are physical pixels relative
to the entire frozen image of the chosen display. Qt caches successful results
and partitions characters by the current crop. `--auto-ocr` starts recognition
on the first press that begins drawing a selection; subsequent OCR button clicks
toggle visibility. Failed and cancelled scans remain retryable.

Qt cancels a pending scan with `OCR_CANCEL:{"id":"..."}`. Rust terminates and
reaps the OCR process tree before replying `{"id":"...","cancelled":true}`.
Termination failures return `cancelled:false` and `error`. Qt ignores racing
scan results while cancellation is pending. The OCR canvas context menu can
cancel a pending scan without closing the overlay; the status capsule then
offers Retry. There is no separate Cancel OCR button. Captures default to Traditional with Auto OCR enabled; the former capture preferences
are not read. Configuration contains OCR models, their selector, and Image hosting. Unix process groups and
Windows Job Objects contain each OCR job. Closing the overlay stops its jobs.
Keep this contract in sync across both Rust crates and Qt.

QR uses `QR_REQUEST:{"id":"qr-...","image":"<path>"}` from Qt and one
JSON response on stdin with the same `id` and either `url` or `error`. Qt
provides a temporary PNG of only the selected crop in physical pixels, without
overlay decorations. Rust decodes it locally using `rqrr` on a background
thread and accepts only HTTP/HTTPS URLs. Missing codes and non-URL payloads
remain errors in the overlay. On success Qt emits `QR_OPEN:{"url":"..."}`
and exits 0. Rust stops capture workers and waits for the Qt process to close
before opening the URL in the default browser. Closing the overlay while a
scan is pending discards the result without opening a browser. QR creates no
additional sidecar and does not depend on OCR.
After a failed local scan, its status capsule offers "Ask Google?" with an
external-link icon. The action submits the current crop through the shared
Ask Google path with the exact prompt "scan this qr code if found". Resizing
the crop or starting another scan clears this fallback; image-preparation
errors do not offer it.

The first selection press chooses the sole owner of application shortcuts and
starts Auto OCR silently. OCR controls and status stay hidden until selection
finishes. Do not change native activation, keyboard grabs, or window visibility
inside the drawing press. After release, activate the chosen display and acquire
its keyboard grab before hiding any other overlay. Do not release the previous
grab first; replacing it must keep keyboard input inside Qt. Check the native
grab result, and keep the other overlays mapped if transfer fails. Crop changes
do not repeat that transfer. Drawing canvases must stop on a cancelled press and
must not draw on hover. Escape is handled by the chosen overlay's native event
filter; copy and copy-all use application shortcuts enabled only for that
overlay. OCR text editors never take keyboard focus
on a press: use `activeFocusOnPress: false` and `persistentSelection: true`.
OCR text selection uses one anchor and cursor range across recognized lines,
including reverse dragging, word selection, and Shift-click extension. The
overlay copies the combined plain text with line breaks. Ctrl+A and the OCR
dock's Select all select recognized text inside the current crop. Editors
remain unfocused while the overlay owns keyboard shortcuts.
Tapping Escape clears text or annotation selection and closes open menus without
dismissing the capture and replays the exit hint. Holding Escape for 650 ms cancels the active capture directly,
including pending OCR or QR work. Each capture briefly fades in a hint below the
top edge with an Esc keycap; hovering the top-center edge drops down an X exit
button that cancels the capture. Clicking blank space or starting a crop
move/resize clears text selection.
Keyboard focus stays in the overlay throughout text selection and deselection.

Keep the overlay managed by the window manager; do not use
`BypassWindowManagerHint`, which suppresses normal X11 click activation. Once a
crop exists, every mouse press, double click, or touch press anywhere in its
window must request native activation and refresh that window's keyboard grab
before QML dispatch. This includes text, resize handles, controls, and blank
space. The window event filter must not consume the input event. Clicking back
from another display must restore keyboard input without toggling OCR.

Other capture contracts:

- Full screen lives in the annotation canvas context menus and the OCR/QR
  canvas context menu; there is no visible crop button. It expands the shared
  crop to the current display through `setSelection`. Resize handles stay
  visible within the display bounds for manual resizing afterward.

- Crop gestures keep only the latest pointer position and apply it at most
  once per 16 ms animation frame, with an immediate final update on release.
  The dim area uses one native `CropOverlay` scene-graph mesh with fixed vertex
  and index buffers rather than resizing separate QML rectangles.
  Annotation brush-hover painting pauses while a crop gesture is active.
  During a held crop gesture, canvas cursor bindings retain the pressed
  handle's native cursor shape even if the pointer outruns the crop geometry.
  Releasing or cancelling the gesture restores normal hover cursor shapes.
  OCR crop partitioning runs once when the gesture ends rather than rebuilding
  text delegates on every pointer event. Hidden OCR layers do not instantiate
  text delegates, and clearing an empty text selection does not walk the model.
  QR rescanning waits until the gesture ends. Its empty crop interior uses
  native open/closed hand cursors and supports dragging the box; other scenes
  retain their existing interior interaction.

- While OCR scans, its canvas context menu offers Full screen and Cancel OCR.
  Otherwise it offers Select all, Copy, Search, Translate, Hide/Show OCR, and
  Full screen. Text actions use the floating dock's shared enablement rules.
  Opening the menu preserves text selection, and its entries stay current as
  recognition or selection state changes. It uses the shared annotation menu
  style and closes on a tap of Escape.

- OCR has no toast or separate scan/cancel toolbar. After crop selection, a
  bottom-centered capsule uses a 180 ms fade-in shared with QR, without blur
  or scale. Ask Google alone retains its 700 ms blur-and-scale opening.
  The OCR capsule shows a circular rotator in the text color with
  "Recognizing display text".
  Successful recognition selects all text inside the crop once its visible
  delegates are ready. If recognition finishes before crop selection or during
  a crop gesture, this initial text selection waits until the crop is ready.
  Success animates its width to fit Select all, Copy, Search, Open in Translate,
  and Hide OCR. Errors appear
  inside the capsule with Retry. Copy, Search, and Translate require text
  selection. Clicking the dock's Copy button temporarily changes Copy to
  Copied; repeated dock clicks restart the feedback. Context-menu and keyboard
  copies are silent. Hide OCR changes to Show OCR while
  results are hidden. Action buttons stay mounted across state changes to
  preserve hover. OCR, Snapshot, and Ask Google use the same active crop-border
  and crop-handle MouseAreas, with fixed native cursor shapes. OCR text hit
  areas and a default-arrow blank-canvas MouseArea stay below those crop
  controls. The blank-canvas surface remains active while OCR is hidden.
  Text lines share one selection range while owning fixed I-beam hover regions.
  Do not add native cursor overrides or a covering pointer layer above the crop.
  On Linux, native cursor changes request a capture-window frame so cursor
  presentation stays current after recognition and opening animations stop.
  Hidden drawing canvases and OCR line editors do not handle pointer input.
  Holding Escape dismisses the capture and terminates its pending jobs.
  Search and Translate percent-encode the selected text into Google URLs and
  finish with `capture_finished` status
  `ocr_open` and a `url` field. Rust releases the capture before opening the URL
  in the default browser.

- Snapshot keeps only the chosen display's overlay after selection, with a
  movable, resizable crop and a shared annotation dock. Its Export action saves
  to `Pictures/squigit` and keeps the overlay open. Ask Google shares the dock
  and adds a floating composer centered at the
  bottom of the chosen display and layered above the crop; sending writes the
  crop to app data `blob_storage` and closes the overlay before uploading.
  Circle to Search saves automatically, and QR scans automatically with an
  Ask Google fallback after local recognition fails. The composer and QR
  fallback use `CaptureEngine::askGoogle` to save their current crop in
  `SQUIGIT_BLOB_DIR` and finish with `capture_finished` status `ask_google`,
  including `prompt` and `path`. Rust forwards both to the shared background
  submission in `squigit/src/image_search.rs`. Circle to Search also uses this
  submission after saving and closing the overlay. Upload each capture to
  ImgBB, then open Google Lens for Circle to Search, or AI Mode (`udm=50`)
  with the encoded prompt for Ask Google and the QR fallback. Do not cache
  hosted URLs or deduplicate images. Upload and browser errors use OS
  notifications. On every app launch, check for a configured ImgBB key in the
  background without decrypting it or prompting for vault access. A missing
  key produces a persistent Set API action that opens Rune configuration
  directly to Image hosting. Every missing-key image action failure must send
  a fresh notification even when an earlier notice remains uncleared. Use a
  random hidden action identifier and Linux event metadata to distinguish
  occurrences; keep the visible text unchanged without counters. Each Set API
  action must still open Image hosting.

- Each successful dock export emits
  `{"type":"image_exported","session":"...","path":"..."}` on Qt stdout.
  Rust forwards it to the existing saved-image OS notification without releasing
  the capture or cancelling its workers. Copy and Export confirm each successful
  action with a temporary check icon, including repeated clicks and shortcuts.
  Annotation operations do not show toasts.

- Hide the radial window and its center label immediately, with zero hide
  animation. Disable native transitions where supported and allow 350 ms for
  compositor transitions to finish before grabbing a new frame.
- Keep the center label window within the ring's hole; its native bounds must
  not overlap the petals. Arrow keys and Tab cycle petals while the menu is
  focused, with Shift+Tab reversing and Enter or Space activating. Play the
  bundled tick only after a stable change to a different highlighted petal.

- Exit codes of direct capture: `0` success, `2` cancelled, `1` failure or
  unexpected runtime exit.
- On a successful capture the wrapper prints exactly one stdout line: the
  absolute path of the saved PNG. Diagnostics go to stderr (`[squigit]` and
  `CAPTURE_NATIVE_ERROR:` prefixes).
- Save location: `SQUIGIT_OUTPUT_DIR` (set by the wrapper from
  `--output-dir`, or `Pictures/squigit` when unset). The Qt runtime also
  falls back to the standard Pictures location when run bare.
- File naming: `squigit-<yyyyMMdd-HHmmss-zzz>.png`, with a numeric suffix on
  collision.

---

## 5. Qt Native Layer (`native/`)

- C++17, CMake ≥ 3.16, Qt ≥ 6.4 modules: `Core`, `Gui`, `Quick`, `Qml`, and
  `DBus` on Linux. The QML canvases
  require `Qt5Compat.GraphicalEffects`. The annotation editor also requires the
  `QtQuick.Controls` and `QtQuick.Templates` QML modules.
- `native/qt/backends/` holds the per-OS grabbers behind the `Grabber`
  interface; each platform module exposes `createUnixEngine` /
  `createWindowsEngine`.
- The Linux overlay forces `QT_QPA_PLATFORM=xcb`; Wayland frames come from
  the desktop portal and are re-cropped per screen.
- Deploy flows: `linuxdeployqt` (Linux), `macdeployqt` + optional codesign
  (macOS), `windeployqt` with VC runtime bundling (Windows). These live in
  `crates/squigit-qt/build.rs`, included by xtask with Cargo auto-build disabled.
- Keep the runtime's stdout markers identical to §4; do not add prose output
  on stdout.

---

OCR native inputs are forked from `squigit/squigit-ocr` at
`39a6aa9efa6d81e5e06493fee991c67db5550733`. Preserve its split dependency
installation, all patches, model bootstrap, PyInstaller spec, loader behavior,
validation scripts, and symlink-preserving packaging. The build driver lives in
`crates/squigit-ocr/build.rs`; `crates/squigit-ocr/embed.rs` embeds its staged output
for release and normal Cargo builds. The `staged-runtime` feature is restricted
to debug builds and resolves only this repository's prebuilt
`binaries/paddle-ocr-<host>` tree.
Model downloads stream real byte progress to Rune through the Rust model manager.
The application's `squigit/src/storage.rs` owns app data paths and atomic, process-locked
`config.toml` updates. Rune ensures this directory and its `models/` subdirectory
exist on launch and settings focus; model access also recreates missing data.
Use `~/.config/squigit` (or `$XDG_CONFIG_HOME/squigit`) on Linux,
`%APPDATA%\Squigit` on Windows, and `~/Library/Application Support/Squigit` on macOS.
`SQUIGIT_CONFIG_DIR` overrides that root. `config.toml` contains
`[ocr].selected_model`; do not restore the removed capture or demo preferences.
The Image hosting page manages only ImgBB. `squigit/src/auth.rs` stores its key
as an AES-256-GCM record in atomic, process-locked `auth.json`, with HKDF-SHA256
and a random encryption master in the OS credential vault. There are no
accounts, profiles, OAuth, or SQLite stores. Never fall back to plaintext or
replace a missing encryption master while encrypted keys exist. Native reveal
authentication uses Polkit on Linux, Windows Hello on Windows, and Local
Authentication on macOS. A successful authenticated reveal grants 600 seconds
of repeat reveals for the same app data store without another prompt. Keep
this grant only in memory, start its monotonic timer after native authentication,
and do not extend it on repeat reveals or reset it when hiding settings.
Failed or cancelled authentication does not grant a grace period.
Clear plaintext UI buffers on field/window blur,
page changes, and close. Linux releases include `squigit/resources/polkit`;
the installer copies the reveal policy to the system policy directory.

## 6. Writing Rules (Comments & Emojis)

- Do not add explanation comments. Code must be self-explanatory. Never write
  `//` walkthroughs, `/* */` section dividers, or narration describing what
  code does or why it exists. The only comments ever permitted are machine
  directives that change tool behavior (`clang-format` markers,
  `rustfmt::skip`, `allow(dead_code)`, cfg annotations) and the concise domain
  notes already present on window flags, protocol markers, and portal quirks —
  preserve those, but do not grow them.
- Do not use emojis anywhere in code, comments, commit messages, or
  user-facing strings unless the user explicitly requests them.
- No decorative artifacts: keep commit messages short, imperative, and
  factual.

---

## 7. Tool Files

- `rust-toolchain.toml` pins the stable toolchain with `clippy`, `rustfmt`,
  and `rust-src` components; do not override it ad hoc in CI steps that build
  on native runners.
- `rustfmt.toml` enforces edition 2021 and Unix newlines.
- `.cargo/config.toml` only defines the `xtask` alias.
- `.prettierignore` excludes generated trees (`target`, `binaries`, native generated trees, and `Cargo.lock`) from Prettier runs.
- `.gitattributes` forces LF and marks QML/scripts as vendored for linguist.
