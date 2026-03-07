#!/usr/bin/env python3
"""
Remove unused simple CSS module selectors based on TSX usage.

Matches TSX usages:
- styles.foo
- styles["foo"]
- styles['foo']

Removes only top-level selectors exactly shaped like:
- .foo { ... }

Complex selectors are intentionally skipped.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path
from typing import Iterable, Iterator


REPO_ROOT = Path(__file__).resolve().parents[2]
IGNORED_DIR_NAMES = {
    ".git",
    ".next",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
}

STYLE_USAGE_PATTERNS = [
    re.compile(r"\bstyles\.([A-Za-z_][A-Za-z0-9_-]*)\b"),
    re.compile(r"""\bstyles\[\s*["']([A-Za-z_][A-Za-z0-9_-]*)["']\s*\]"""),
]
SIMPLE_CLASS_SELECTOR_RE = re.compile(r"\.([A-Za-z_][A-Za-z0-9_-]*)\s*\{")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Remove unused simple top-level CSS module classes from .module.css "
            "files using TSX style usage."
        )
    )
    parser.add_argument(
        "root",
        nargs="?",
        default=".",
        help="Project root directory to scan.",
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Apply edits in-place. Without this flag, runs in dry-run mode.",
    )
    return parser.parse_args()


def resolve_root(root_arg: str) -> Path:
    root = Path(root_arg)
    if not root.is_absolute():
        root = (REPO_ROOT / root).resolve()
    return root


def iter_files(root: Path, suffix: str) -> Iterator[Path]:
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [name for name in dirnames if name not in IGNORED_DIR_NAMES]
        for filename in filenames:
            if filename.endswith(suffix):
                yield Path(dirpath) / filename


def collect_used_classes(root: Path) -> set[str]:
    used: set[str] = set()
    for path in iter_files(root, ".tsx"):
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for pattern in STYLE_USAGE_PATTERNS:
            used.update(pattern.findall(text))
    return used


def find_matching_brace(text: str, open_brace_index: int) -> int:
    if (
        open_brace_index < 0
        or open_brace_index >= len(text)
        or text[open_brace_index] != "{"
    ):
        return -1

    depth = 0
    i = open_brace_index
    in_single = False
    in_double = False
    in_comment = False

    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""

        if in_comment:
            if ch == "*" and nxt == "/":
                in_comment = False
                i += 2
                continue
            i += 1
            continue

        if in_single:
            if ch == "\\" and nxt:
                i += 2
                continue
            if ch == "'":
                in_single = False
            i += 1
            continue

        if in_double:
            if ch == "\\" and nxt:
                i += 2
                continue
            if ch == '"':
                in_double = False
            i += 1
            continue

        if ch == "/" and nxt == "*":
            in_comment = True
            i += 2
            continue
        if ch == "'":
            in_single = True
            i += 1
            continue
        if ch == '"':
            in_double = True
            i += 1
            continue

        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i
            if depth < 0:
                return -1

        i += 1

    return -1


def extract_top_level_css_class_blocks(css: str) -> list[tuple[int, int, str]]:
    blocks: list[tuple[int, int, str]] = []
    i = 0
    depth = 0

    while i < len(css):
        if css.startswith("/*", i):
            end = css.find("*/", i + 2)
            if end == -1:
                break
            i = end + 2
            continue

        ch = css[i]

        if depth == 0:
            match = SIMPLE_CLASS_SELECTOR_RE.match(css, i)
            if match:
                open_brace_index = match.end() - 1
                close_brace_index = find_matching_brace(css, open_brace_index)
                if close_brace_index == -1:
                    i += 1
                    continue
                blocks.append((match.start(), close_brace_index + 1, match.group(1)))
                i = close_brace_index + 1
                continue

        if ch == "{":
            depth += 1
        elif ch == "}":
            depth = max(0, depth - 1)

        i += 1

    return blocks


def _consume_trailing_whitespace_and_one_newline(text: str, i: int) -> int:
    while i < len(text) and text[i] in " \t":
        i += 1
    if i < len(text) and text[i] == "\n":
        i += 1
    return i


def clean_css_file(path: Path, used_classes: set[str], dry_run: bool) -> tuple[int, str]:
    try:
        original = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return 0, f"ERROR reading {path}: {exc}"

    blocks = extract_top_level_css_class_blocks(original)
    if not blocks:
        return 0, f"No simple top-level class blocks found in {path}"

    to_remove = [
        (start, end, class_name)
        for start, end, class_name in blocks
        if class_name not in used_classes
    ]
    if not to_remove:
        return 0, f"No unused classes in {path}"

    new_parts: list[str] = []
    cursor = 0
    removed_names: list[str] = []

    for start, end, class_name in to_remove:
        new_parts.append(original[cursor:start])
        cursor = _consume_trailing_whitespace_and_one_newline(original, end)
        removed_names.append(class_name)

    new_parts.append(original[cursor:])
    cleaned = "".join(new_parts)

    if not dry_run and cleaned != original:
        path.write_text(cleaned, encoding="utf-8")

    return (
        len(to_remove),
        f"{path}: removed {len(to_remove)} -> {', '.join(removed_names)}",
    )


def iter_module_css_files(root: Path) -> Iterable[Path]:
    yield from iter_files(root, ".module.css")


def main() -> int:
    args = parse_args()
    root = resolve_root(args.root)
    dry_run = not args.write

    if not root.exists():
        print(f"error: root path does not exist: {root}", file=sys.stderr)
        return 1

    used_classes = collect_used_classes(root)
    css_files = sorted(iter_module_css_files(root))

    print(f"Scan root: {root}")
    print(f"Found {len(used_classes)} used class name references in TSX files.")
    print(f"Mode: {'DRY RUN' if dry_run else 'WRITE'}")
    print(f"Module CSS files: {len(css_files)}")
    print()

    total_removed = 0
    files_changed = 0
    for css_file in css_files:
        removed_count, message = clean_css_file(css_file, used_classes, dry_run=dry_run)
        total_removed += removed_count
        if removed_count > 0:
            files_changed += 1
        print(message)

    print()
    print(
        f"Summary: files_scanned={len(css_files)} files_changed={files_changed} "
        f"removed_class_blocks={total_removed}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
