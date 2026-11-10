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
│   └── squigit-ocr/      # Rust OCR runner and JSON result types
├── native/
│   ├── qt/              # C++/QML overlay and platform grabbers
│   └── ocr/             # Python OCR CLI and PyInstaller inputs
├── xtask/               # Build, deployment, packaging, and development tasks
├── Cargo.toml           # Workspace manifest
└── README.md
```

The only public application is `squigit`. It owns the tray, settings, about,
and distribution. `squigit-qt` and `squigit-ocr` are private sidecars.

Do not add or run tests unless asked. Do not automate runtime or UI
validation; ask the user to validate interactive flows. Never use sub-agents.
Refactors replace old formats; do not introduce legacy compatibility paths.

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

Use Rust's debug profile and Qt Debug for feature iteration. Release builds and
runtime deployment are reserved for an explicit packaging request. For Qt-only
changes, rebuild only Qt and refresh its executable in the existing package;
do not rebuild Rust or redeploy unchanged Qt libraries.

```bash
cmake -S native/qt -B native/qt/build -DCMAKE_BUILD_TYPE=Debug
cmake --build native/qt/build --config Debug --parallel 2
```

```bash
# Full pipeline: CMake build -> runtime deploy -> cargo release build -> package
cargo xtask build

# Only configure + build the Qt runtime (fast iteration on native code)
cargo xtask build --qt

# Execute the packaged binary; arguments after -- are forwarded to it
cargo xtask run
cargo xtask run -- --traditional
cargo xtask run -- --squiggle --output-dir /tmp/shots

# Workspace checks: cargo check + cmake configure smoke test
cargo xtask doctor

# Format git-changed files only (rustfmt + Prettier + clang-format)
cargo xtask fmt

# Format everything
cargo xtask fmt --all

# Remove target/, native/qt/build, native/qt/_internal, native/ocr/build, native/ocr/dist, binaries/
cargo xtask clean
```

> [!TIP]
> Always run `cargo xtask fmt` before committing and `cargo xtask doctor`
> before completing tasks, so Rust, Prettier, and native diagnostics stay
> clean.

Packaging output layout:

```text
binaries/<host-triple>/
├── squigit            # the Rust binary (or squigit.exe on Windows)
└── _internal/         # private Qt and OCR deployment trees
```

`cargo xtask run` executes `binaries/<host-triple>/squigit` and forwards the
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

OCR uses `OCR_REQUEST:{"id":"...","image":"<path>"}` from Qt and one
newline-delimited JSON response on Qt stdin, with the same `id` and either
`boxes` or `error`. Rust temporarily starts the installed `squigit-ocr` LTS
from PATH for each request; normal builds skip the local OCR fork. The OCR
CLI prints exactly one JSON array of `{text, confidence, box}` on success,
or an `{error}` object on failure. Coordinates are physical pixels relative
to the entire frozen image of the chosen display. Qt caches successful results
and partitions characters by the current crop. `--auto-ocr` starts recognition
on the first press that begins drawing a selection; subsequent OCR button clicks
toggle visibility. Failed and cancelled scans remain retryable.

Qt cancels a pending scan with `OCR_CANCEL:{"id":"..."}`. Rust terminates and
reaps the OCR process tree before replying `{"id":"...","cancelled":true}`.
Termination failures return `cancelled:false` and `error`. Qt ignores racing
scan results while cancellation is pending. Cancelling OCR keeps the overlay
open and does not change the Auto OCR preference. Unix process groups and
Windows Job Objects contain each OCR job. Closing the overlay stops its jobs.
Keep this contract in sync across both Rust crates and Qt.

The first selection press chooses the sole owner of application shortcuts and
starts Auto OCR silently. OCR controls and status stay hidden until selection
finishes. Do not change native activation, keyboard grabs, or window visibility
inside the drawing press. After release, hide the other overlays and transfer
activation and the keyboard grab to the chosen display once. Crop changes do
not repeat that transfer. Drawing canvases must stop on a cancelled press and
must not draw on hover. Escape and copy-all use application shortcuts enabled
only for the chosen overlay; text fields own normal copy handling.

Other capture contracts:

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
  `DBus` on Linux. The QML canvases require `Qt5Compat.GraphicalEffects`.
- `native/qt/backends/` holds the per-OS grabbers behind the `Grabber`
  interface; each platform module exposes `createUnixEngine` /
  `createWindowsEngine`.
- The Linux overlay forces `QT_QPA_PLATFORM=xcb`; Wayland frames come from
  the desktop portal and are re-cropped per screen.
- Deploy flows: `linuxdeployqt` (Linux), `macdeployqt` + optional codesign
  (macOS), `windeployqt` with VC runtime bundling (Windows). These live in
  `xtask/src/native.rs`.
- Keep the runtime's stdout markers identical to §4; do not add prose output
  on stdout.

---

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
- `.prettierignore` excludes generated trees (`target`, `binaries`, native generated trees, the OCR build environment, and `Cargo.lock`) from Prettier runs.
- `.gitattributes` forces LF and marks QML/scripts as vendored for linguist.
