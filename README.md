# Squigit

Native screenshot tool. Qt 6 provides the on-screen selection overlays and
screen grabbing; Rust is the entrypoint, session guardian, and packager. The
built binary saves the cutout into a folder and prints the absolute path of
the saved image.

## Features

- `--squiggle` (default): freehand circle-style selection with smooth strokes
  and a soft glow, à la "Circle to Search".
- `--traditional`: rectangular selection with rounded corners and a gradient
  stroke outline.
- Multi-monitor aware: one fullscreen frozen frame per display.
- Wayland capture via the desktop portal (with permission preflight), native
  grabs on X11, macOS (CoreGraphics), and Windows (GDI+).
- Single-instance lock, display hotplug watchdog, and shutter-sound
  suppression on macOS and Wayland.
- Output: `<output-dir>/squigit-<timestamp>.png`, the path is printed on
  stdout (the only stdout line on success). Exit codes: `0` success, `2`
  cancelled, `1` failure.

## Building and running

```bash
cargo xtask build         # cmake build + runtime deploy + rust release + package
cargo xtask build --qt    # only the Qt runtime steps
cargo xtask run           # execute the packaged binary
cargo xtask run -- --traditional --output-dir /tmp/shots
```

The packaged tree lives at `binaries/<host-triple>/`:

```text
binaries/x86_64-unknown-linux-gnu/
├── squigit                     # the Rust entrypoint binary
└── _internal/                  # deployed Qt runtime (bundler output)
```

On macOS the runtime is `Squigit.app`, on Windows it is a windeployqt-spread
tree next to `squigit-qt.exe`.

## Usage

```text
Usage:
  squigit [MODE] [--output-dir <DIR>]
  squigit --authorize
  squigit --preflight
  squigit --help
  squigit --version

Capture modes:
  --squiggle     Capture a freeform squiggle selection (default)
  --traditional  Capture a rectangular selection

Options:
  --output-dir <DIR>  Save the captured image into DIR (default: Pictures/squigit)
  --authorize         Request screenshot access without opening the selection overlay
  --preflight         Check screenshot access without requesting permission
  -h, --help          Print this help
  -V, --version       Print the squigit version
```

## Repository layout

```text
squigit/
├── .cargo/config.toml      # cargo xtask alias
├── .github/                # build workflow + composite action
├── docker/                 # linux builder images (ubuntu, fedora)
├── native/                 # Qt 6 selection engine (CMake, QML, grabber backends)
├── src/                    # Rust sources of the squigit binary
└── xtask/                  # build / run / fmt / doctor / clean automation
```

## Development

- `cargo xtask fmt [--all]` — rustfmt + prettier + clang-format over changed
  files (or the whole tree).
- `cargo xtask doctor` — `cargo check --workspace` plus a cmake reconfigure.
- `cargo xtask clean` — drop the cargo target, native build tree, and

  packaged binaries.

See [`AGENTS.md`](AGENTS.md) for architecture boundaries and the marker
protocol between the Rust process and the Qt runtime.
