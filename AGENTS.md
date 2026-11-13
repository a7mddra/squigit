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
scan results while cancellation is pending. Cancelling OCR keeps the overlay
open. Captures default to Traditional with Auto OCR enabled; the former capture preferences
are not read. Configuration contains only OCR models and their selector. Unix process groups and
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
The overlay copies the selected editor through its `copy()` method. Selecting
another text line clears previous selections without focusing an editor.
Tapping Escape clears OCR text selection before dismissing the capture on a
subsequent tap. Holding Escape for 650 ms cancels the active capture directly,
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

- Snapshot keeps only the chosen display's overlay after selection, with a
  movable, resizable crop and a Save button, saving to `Pictures/Squigit`. Ask
  Google replaces the Save button with a floating composer centered at the
  bottom of the chosen display and layered above the crop; sending writes the
  crop to app data `blob_storage`, prints the prompt, and closes the overlay.
  Circle to Search saves automatically, and QR scans automatically; neither has
  buttons.

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
  `QtQuick.Controls`, `QtQuick.Templates`, and `QtQuick.Dialogs` QML modules.
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
`SQUIGIT_CONFIG_DIR` overrides that root. The only current setting is
`[ocr].selected_model`; do not restore the removed capture or demo preferences.

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
