#!/usr/bin/env python3
"""
Reusable bulk find/replace tool.

Examples:
  python3 .codex/tools/replacing.py \
    --map .codex/replacements/token-rename-v1.json \
    --include "ui/src/**/*.css" \
    --include "ui/src/**/*.tsx" \
    --identifier-boundary

  python3 .codex/tools/replacing.py \
    --find ahmed --replace a7md \
    --include "**/*.rs" \
    --apply
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence


REPO_ROOT = Path(__file__).resolve().parents[2]


@dataclass
class Rule:
    find: str
    replace: str
    regex: bool = False
    ignore_case: bool = False


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Batch replacement tool (VS Code style, scriptable).")
    parser.add_argument("--root", default=".", help="Base directory for include/exclude globs.")
    parser.add_argument(
        "--include",
        action="append",
        default=[],
        help='Glob to include. Repeatable. Example: --include "ui/src/**/*.css"',
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        help='Glob to exclude. Repeatable. Example: --exclude "**/*.generated.css"',
    )
    parser.add_argument("--map", dest="map_path", help="JSON file with replacement rules.")
    parser.add_argument("--find", help="Single find expression.")
    parser.add_argument("--replace", help="Single replacement value.")
    parser.add_argument("--regex", action="store_true", help="Treat --find as regex.")
    parser.add_argument("--ignore-case", action="store_true", help="Case-insensitive matching.")
    parser.add_argument(
        "--identifier-boundary",
        action="store_true",
        help="For literal rules, match exact identifier token (no [A-Za-z0-9-] neighbors).",
    )
    parser.add_argument("--apply", action="store_true", help="Write edits. Without this, dry-run only.")
    parser.add_argument("--verbose", action="store_true", help="Print per-file change counts.")
    return parser.parse_args()


def load_rules(args: argparse.Namespace) -> List[Rule]:
    rules: List[Rule] = []

    if args.map_path:
        map_path = Path(args.map_path)
        if not map_path.is_absolute():
            map_path = REPO_ROOT / map_path
        payload = json.loads(map_path.read_text(encoding="utf-8"))
        if not isinstance(payload, list):
            raise ValueError("map file must be a JSON array")
        for idx, item in enumerate(payload):
            if not isinstance(item, dict):
                raise ValueError(f"rule #{idx} must be an object")
            find = item.get("find")
            replace = item.get("replace")
            if not isinstance(find, str) or not isinstance(replace, str):
                raise ValueError(f"rule #{idx} requires string 'find' and 'replace'")
            rules.append(
                Rule(
                    find=find,
                    replace=replace,
                    regex=bool(item.get("regex", False)),
                    ignore_case=bool(item.get("ignore_case", False)),
                )
            )

    if args.find is not None:
        if args.replace is None:
            raise ValueError("--replace is required when --find is used")
        rules.append(
            Rule(
                find=args.find,
                replace=args.replace,
                regex=bool(args.regex),
                ignore_case=bool(args.ignore_case),
            )
        )

    if not rules:
        raise ValueError("no replacement rules provided (use --map or --find/--replace)")

    return rules


def compile_rule(rule: Rule, identifier_boundary: bool) -> re.Pattern[str]:
    flags = re.MULTILINE
    if rule.ignore_case:
        flags |= re.IGNORECASE

    if rule.regex:
        pattern = rule.find
    else:
        escaped = re.escape(rule.find)
        if identifier_boundary:
            pattern = rf"(?<![A-Za-z0-9-]){escaped}(?![A-Za-z0-9-])"
        else:
            pattern = escaped

    return re.compile(pattern, flags=flags)


def discover_files(root: Path, includes: Sequence[str], excludes: Sequence[str]) -> List[Path]:
    include_globs = list(includes) if includes else ["**/*"]
    matched: set[Path] = set()

    for pattern in include_globs:
        for path in root.glob(pattern):
            if path.is_file():
                matched.add(path.resolve())

    excluded: set[Path] = set()
    for pattern in excludes:
        for path in root.glob(pattern):
            if path.is_file():
                excluded.add(path.resolve())

    result = sorted(p for p in matched if p not in excluded)
    return result


def apply_rules_to_text(text: str, rules: Sequence[Rule], identifier_boundary: bool) -> tuple[str, int]:
    total_replacements = 0
    out = text
    for rule in rules:
        pattern = compile_rule(rule, identifier_boundary=identifier_boundary and not rule.regex)
        out, count = pattern.subn(rule.replace, out)
        total_replacements += count
    return out, total_replacements


def main() -> int:
    args = parse_args()

    try:
        rules = load_rules(args)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    root = Path(args.root)
    if not root.is_absolute():
        root = (REPO_ROOT / root).resolve()

    if not root.exists():
        print(f"error: root path does not exist: {root}", file=sys.stderr)
        return 1

    files = discover_files(root=root, includes=args.include, excludes=args.exclude)
    changed_files = 0
    total_replacements = 0

    for path in files:
        try:
            original = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue

        updated, count = apply_rules_to_text(
            original, rules=rules, identifier_boundary=args.identifier_boundary
        )
        if count == 0:
            continue

        changed_files += 1
        total_replacements += count

        if args.apply:
            path.write_text(updated, encoding="utf-8")

        if args.verbose:
            rel = path.relative_to(REPO_ROOT).as_posix()
            print(f"{rel}: replacements={count}")

    mode = "apply" if args.apply else "dry-run"
    print(
        f"[{mode}] files_scanned={len(files)} changed_files={changed_files} "
        f"total_replacements={total_replacements} rules={len(rules)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
