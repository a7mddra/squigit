#!/usr/bin/env python3
"""
Color tokenizer for CSS Modules + TSX style literals.

Modes:
- --report: scan + map + reports only
- --test:   generate .generated preview files (no source writes)
- --apply:  apply source edits + append tokens to variables.css
- --write:  alias of --apply (kept for compatibility)
"""

from __future__ import annotations

import argparse
import bisect
import difflib
import json
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple


REPO_ROOT = Path(__file__).resolve().parents[2]
UI_ROOT = REPO_ROOT / "ui"
SRC_ROOT = UI_ROOT / "src"
VARIABLES_CSS = SRC_ROOT / "styles" / "variables.css"
GENERATED_VARIABLES_CSS = SRC_ROOT / "styles" / "variables.generated.css"
REPORT_DIR = REPO_ROOT / ".codex" / "reports"
REPORT_JSON = REPORT_DIR / "color-map.json"
REPORT_MD = REPORT_DIR / "color-report.md"
REPORT_PATCH = REPORT_DIR / "color-tokenization.patch"

TARGET_SUFFIXES = (".module.css", ".tsx")
EXCLUDED_FILES = {
    "ui/src/styles/globals.css",
    "ui/src/styles/animations.css",
}

SKIP_WORDS = {"transparent", "currentcolor", "inherit", "initial", "unset", "none"}

HEX_PATTERN = r"#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{4}|[0-9a-fA-F]{3})\b"
FUNC_PATTERN = r"\b(?:rgba?|hsla?)\(\s*[^()]*\)"
NAME_PATTERN = r"\b(?:white|black)\b"
COLOR_PATTERN = rf"(?:{HEX_PATTERN}|{FUNC_PATTERN}|{NAME_PATTERN})"

COLOR_LITERAL_RE = re.compile(COLOR_PATTERN, re.IGNORECASE)
PURE_COLOR_RE = re.compile(rf"^\s*{COLOR_PATTERN}\s*$", re.IGNORECASE)
CSS_DECL_RE = re.compile(
    r"(?P<prop>(?:--)?[a-zA-Z0-9-]+)\s*:\s*(?P<value>[^;{{}}]+);"
)
VAR_DECL_RE = re.compile(r"(?P<prop>--[a-zA-Z0-9-]+)\s*:\s*(?P<value>[^;{{}}]+);")

JSX_ATTR_RE = re.compile(
    r"\b(?P<prop>fill|stroke|stopColor|color|backgroundColor|borderColor|outlineColor)"
    r"\s*=\s*(?P<q>[\"'])(?P<value>[^\"']*)(?P=q)"
)
STYLE_PROP_RE = re.compile(
    r"\b(?P<prop>color|backgroundColor|borderColor|outlineColor|fill|stroke|boxShadow|"
    r"textShadow|filter|caretColor)\s*:\s*(?P<q>[\"'])(?P<value>[^\"']*)(?P=q)"
)
CONST_OBJECT_START_RE = re.compile(
    r"\b(?:const|let|var)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b[^=;]*=\s*\{",
    re.MULTILINE,
)
DICT_NAME_RE = re.compile(r"(?:color|colors|palette|theme)", re.IGNORECASE)


@dataclass
class Occurrence:
    rel_path: str
    start: int
    end: int
    raw: str
    normalized: str
    context: str
    line: int = 0
    column: int = 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Tokenize hardcoded colors in CSS modules + TSX.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--report", action="store_true", help="Generate reports only.")
    mode.add_argument(
        "--test",
        action="store_true",
        help="Generate .generated preview files without editing tracked source files.",
    )
    mode.add_argument(
        "--apply",
        action="store_true",
        help="Apply source edits and append new tokens into variables.css.",
    )
    mode.add_argument(
        "--write",
        action="store_true",
        help="Alias of --apply (kept for compatibility).",
    )
    return parser.parse_args()


def mode_name(args: argparse.Namespace) -> str:
    if args.report:
        return "report"
    if args.test:
        return "test"
    if args.apply:
        return "apply"
    return "write"


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def discover_target_files(root: Path) -> List[Path]:
    stack: List[Path] = [root]
    result: List[Path] = []
    while stack:
        current = stack.pop()
        try:
            entries = sorted(current.iterdir(), key=lambda p: p.name, reverse=True)
        except FileNotFoundError:
            continue
        for entry in entries:
            if entry.is_dir():
                stack.append(entry)
                continue
            if not entry.name.endswith(TARGET_SUFFIXES):
                continue
            rel_path = entry.relative_to(REPO_ROOT).as_posix()
            if rel_path in EXCLUDED_FILES:
                continue
            result.append(entry)
    return sorted(result, key=lambda p: p.relative_to(REPO_ROOT).as_posix())


def find_matching_brace(text: str, open_index: int) -> int:
    if open_index < 0 or open_index >= len(text) or text[open_index] != "{":
        return -1
    depth = 0
    i = open_index
    in_single = False
    in_double = False
    in_backtick = False
    in_line_comment = False
    in_block_comment = False

    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
            i += 1
            continue

        if in_block_comment:
            if ch == "*" and nxt == "/":
                in_block_comment = False
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

        if in_backtick:
            if ch == "\\" and nxt:
                i += 2
                continue
            if ch == "`":
                in_backtick = False
            i += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue
        if ch == "/" and nxt == "*":
            in_block_comment = True
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
        if ch == "`":
            in_backtick = True
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


def extract_root_block(text: str) -> str:
    root_match = re.search(r":root\b", text)
    if not root_match:
        return ""
    brace_open = text.find("{", root_match.end())
    if brace_open < 0:
        return ""
    brace_close = find_matching_brace(text, brace_open)
    if brace_close < 0:
        return ""
    return text[brace_open + 1 : brace_close]


def find_selector_block_span(text: str, selector_regex: str) -> Tuple[int, int] | None:
    match = re.search(selector_regex, text, flags=re.MULTILINE)
    if not match:
        return None
    open_brace = text.find("{", match.end())
    if open_brace < 0:
        return None
    close_brace = find_matching_brace(text, open_brace)
    if close_brace < 0:
        return None
    return (open_brace, close_brace)


def build_declared_var_name_set(variables_css_path: Path) -> set[str]:
    if not variables_css_path.exists():
        return set()
    text = variables_css_path.read_text(encoding="utf-8")
    return {match.group("prop").strip() for match in VAR_DECL_RE.finditer(text)}


def normalize_color_literal(raw: str) -> str | None:
    literal = raw.strip().lower()
    if not literal or literal in SKIP_WORDS:
        return None

    if literal == "white":
        return "#ffffff"
    if literal == "black":
        return "#000000"

    if literal.startswith("#"):
        if len(literal) == 4:
            return "#" + "".join(c * 2 for c in literal[1:])
        if len(literal) == 5:
            return "#" + "".join(c * 2 for c in literal[1:])
        if len(literal) in (7, 9):
            return literal
        return None

    fn_match = re.fullmatch(r"(rgba?|hsla?)\((.*)\)", literal, flags=re.IGNORECASE)
    if fn_match:
        fn = fn_match.group(1).lower()
        inner = fn_match.group(2).strip()
        inner = re.sub(r"\s+", " ", inner)
        inner = re.sub(r"\s*,\s*", ", ", inner)
        inner = re.sub(r"\s*/\s*", " / ", inner)
        return f"{fn}({inner})"

    return None


def build_existing_token_index(variables_css_path: Path) -> Dict[str, str]:
    if not variables_css_path.exists():
        return {}
    text = variables_css_path.read_text(encoding="utf-8")
    root_block = extract_root_block(text)
    index: Dict[str, str] = {}
    for match in VAR_DECL_RE.finditer(root_block):
        token = match.group("prop").strip()
        value = match.group("value").strip()
        if "var(--" in value:
            continue
        if not PURE_COLOR_RE.match(value):
            continue
        literal_match = COLOR_LITERAL_RE.search(value)
        if not literal_match:
            continue
        normalized = normalize_color_literal(literal_match.group(0))
        if not normalized:
            continue
        if normalized not in index:
            index[normalized] = token
    return index


def iter_quoted_string_value_spans(text: str, start: int, end: int) -> Iterable[Tuple[int, int]]:
    i = start
    in_line_comment = False
    in_block_comment = False
    while i < end:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < end else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
            i += 1
            continue

        if in_block_comment:
            if ch == "*" and nxt == "/":
                in_block_comment = False
                i += 2
                continue
            i += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue
        if ch == "/" and nxt == "*":
            in_block_comment = True
            i += 2
            continue

        if ch in ("'", '"'):
            quote = ch
            value_start = i + 1
            i += 1
            while i < end:
                c = text[i]
                if c == "\\" and i + 1 < end:
                    i += 2
                    continue
                if c == quote:
                    yield (value_start, i)
                    i += 1
                    break
                i += 1
            continue

        if ch == "`":
            i += 1
            while i < end:
                c = text[i]
                if c == "\\" and i + 1 < end:
                    i += 2
                    continue
                if c == "`":
                    i += 1
                    break
                i += 1
            continue

        i += 1


def annotate_occurrence_positions(text: str, occurrences: Sequence[Occurrence]) -> None:
    newline_indices = [idx for idx, char in enumerate(text) if char == "\n"]
    for occ in occurrences:
        line = bisect.bisect_right(newline_indices, occ.start) + 1
        line_start = newline_indices[line - 2] + 1 if line > 1 else 0
        occ.line = line
        occ.column = occ.start - line_start + 1


def extract_colors_from_segment(
    text: str,
    segment_start: int,
    segment_end: int,
    rel_path: str,
    context: str,
) -> List[Occurrence]:
    segment = text[segment_start:segment_end]
    if "var(--" in segment:
        return []
    hits: List[Occurrence] = []
    for match in COLOR_LITERAL_RE.finditer(segment):
        raw = match.group(0)
        normalized = normalize_color_literal(raw)
        if not normalized:
            continue
        if raw.strip().lower() in SKIP_WORDS:
            continue
        abs_start = segment_start + match.start()
        abs_end = segment_start + match.end()
        hits.append(
            Occurrence(
                rel_path=rel_path,
                start=abs_start,
                end=abs_end,
                raw=raw,
                normalized=normalized,
                context=context,
            )
        )
    return hits


def extract_occurrences_css(text: str, rel_path: str) -> List[Occurrence]:
    occurrences: List[Occurrence] = []
    for match in CSS_DECL_RE.finditer(text):
        prop = match.group("prop").strip()
        value = match.group("value")
        if "var(--" in value:
            continue
        value_start = match.start("value")
        value_end = match.end("value")
        occurrences.extend(
            extract_colors_from_segment(
                text=text,
                segment_start=value_start,
                segment_end=value_end,
                rel_path=rel_path,
                context=f"css:{prop}",
            )
        )
    return occurrences


def extract_occurrences_tsx(text: str, rel_path: str) -> List[Occurrence]:
    occurrences: List[Occurrence] = []

    for match in JSX_ATTR_RE.finditer(text):
        prop = match.group("prop")
        occurrences.extend(
            extract_colors_from_segment(
                text=text,
                segment_start=match.start("value"),
                segment_end=match.end("value"),
                rel_path=rel_path,
                context=f"tsx-jsx-attr:{prop}",
            )
        )

    for match in STYLE_PROP_RE.finditer(text):
        prop = match.group("prop")
        occurrences.extend(
            extract_colors_from_segment(
                text=text,
                segment_start=match.start("value"),
                segment_end=match.end("value"),
                rel_path=rel_path,
                context=f"tsx-style-prop:{prop}",
            )
        )

    for match in CONST_OBJECT_START_RE.finditer(text):
        const_name = match.group("name")
        if not DICT_NAME_RE.search(const_name):
            continue
        open_brace = text.find("{", match.end() - 1)
        if open_brace < 0:
            continue
        close_brace = find_matching_brace(text, open_brace)
        if close_brace < 0:
            continue
        for value_start, value_end in iter_quoted_string_value_spans(
            text, open_brace + 1, close_brace
        ):
            occurrences.extend(
                extract_colors_from_segment(
                    text=text,
                    segment_start=value_start,
                    segment_end=value_end,
                    rel_path=rel_path,
                    context=f"tsx-color-dict:{const_name}",
                )
            )

    return occurrences


def collect_occurrences_for_file(path: Path) -> Tuple[str, List[Occurrence]]:
    text = path.read_text(encoding="utf-8")
    rel_path = path.relative_to(REPO_ROOT).as_posix()

    if path.name.endswith(".module.css"):
        raw_occurrences = extract_occurrences_css(text, rel_path)
    else:
        raw_occurrences = extract_occurrences_tsx(text, rel_path)

    dedup: Dict[Tuple[int, int], Occurrence] = {}
    for occ in raw_occurrences:
        key = (occ.start, occ.end)
        if key not in dedup:
            dedup[key] = occ

    occurrences = sorted(dedup.values(), key=lambda o: (o.start, o.end))
    annotate_occurrence_positions(text, occurrences)
    return text, occurrences


def assign_tokens(
    occurrences: Sequence[Occurrence],
    existing_index: Dict[str, str],
    declared_token_names: set[str],
) -> Tuple[Dict[str, str], Dict[str, str], Counter[str], Dict[str, str]]:
    freq: Counter[str] = Counter(o.normalized for o in occurrences)
    ordered_colors = sorted(freq.keys(), key=lambda color: (-freq[color], color))

    color_to_token: Dict[str, str] = {}
    color_source: Dict[str, str] = {}
    generated: Dict[str, str] = {}

    used_tokens = set(declared_token_names)
    next_id = 0

    for color in ordered_colors:
        if color in existing_index:
            color_to_token[color] = existing_index[color]
            color_source[color] = "existing"
            continue

        while True:
            token = f"--c-raw-{next_id:03d}"
            next_id += 1
            if token not in used_tokens:
                break
        used_tokens.add(token)
        color_to_token[color] = token
        color_source[color] = "generated"
        generated[token] = color

    return color_to_token, color_source, freq, generated


def apply_replacements(text: str, replacements: Sequence[Tuple[int, int, str]]) -> str:
    if not replacements:
        return text
    ordered = sorted(replacements, key=lambda item: (item[0], item[1]))
    out: List[str] = []
    cursor = 0
    for start, end, repl in ordered:
        if start < cursor:
            continue
        out.append(text[cursor:start])
        out.append(repl)
        cursor = end
    out.append(text[cursor:])
    return "".join(out)


def rewrite_contents(
    file_texts: Dict[str, str],
    file_occurrences: Dict[str, List[Occurrence]],
    color_to_token: Dict[str, str],
) -> Dict[str, Tuple[str, str]]:
    changed: Dict[str, Tuple[str, str]] = {}
    for rel_path, original_text in file_texts.items():
        occurrences = file_occurrences[rel_path]
        replacements: List[Tuple[int, int, str]] = []
        for occ in occurrences:
            token = color_to_token[occ.normalized]
            replacements.append((occ.start, occ.end, f"var({token})"))
        rewritten = apply_replacements(original_text, replacements)
        if rewritten != original_text:
            changed[rel_path] = (original_text, rewritten)
    return changed


def build_patch(changed_files: Dict[str, Tuple[str, str]]) -> str:
    lines: List[str] = []
    for rel_path in sorted(changed_files.keys()):
        before, after = changed_files[rel_path]
        diff = difflib.unified_diff(
            before.splitlines(),
            after.splitlines(),
            fromfile=rel_path,
            tofile=rel_path,
            lineterm="",
        )
        chunk = list(diff)
        if not chunk:
            continue
        lines.extend(chunk)
        lines.append("")
    if not lines:
        return "# No source-file changes detected.\n"
    return "\n".join(lines).rstrip() + "\n"


def append_tokens_to_variables_css(text: str, generated_tokens: Dict[str, str]) -> str:
    if not generated_tokens:
        return text

    ordered_pairs = [(token, generated_tokens[token]) for token in sorted(generated_tokens.keys())]

    def append_block(current_text: str, selector_regex: str) -> str:
        span = find_selector_block_span(current_text, selector_regex)
        if not span:
            raise ValueError(f"missing selector block for regex: {selector_regex}")
        open_brace, close_brace = span
        block_body = current_text[open_brace + 1 : close_brace]
        existing = {m.group("prop").strip() for m in VAR_DECL_RE.finditer(block_body)}
        to_add = [(token, value) for token, value in ordered_pairs if token not in existing]
        if not to_add:
            return current_text
        insertion = "\n" + "\n".join(f"  {token}: {value};" for token, value in to_add) + "\n"
        return current_text[:close_brace] + insertion + current_text[close_brace:]

    out = text
    out = append_block(out, r":root\b")
    out = append_block(out, r"body\.light-mode\b")
    return out


def build_generated_header(comment: str) -> str:
    return "\n".join(
        [
            "/**",
            " * @generated by .codex/tools/color_tokenizer.py",
            f" * {comment}",
            " */",
            "",
        ]
    )


def build_json_report(
    mode: str,
    files_scanned: int,
    occurrences: Sequence[Occurrence],
    color_to_token: Dict[str, str],
    color_source: Dict[str, str],
    freq: Counter[str],
) -> Dict[str, object]:
    mapping_rows = []
    for normalized in sorted(freq.keys(), key=lambda c: (-freq[c], c)):
        mapping_rows.append(
            {
                "normalized": normalized,
                "token": color_to_token[normalized],
                "source": color_source[normalized],
                "occurrences": freq[normalized],
            }
        )

    occurrence_rows = []
    for occ in sorted(occurrences, key=lambda o: (o.rel_path, o.line, o.column, o.start)):
        occurrence_rows.append(
            {
                "file": occ.rel_path,
                "line": occ.line,
                "column": occ.column,
                "raw": occ.raw,
                "normalized": occ.normalized,
                "context": occ.context,
                "token": color_to_token[occ.normalized],
                "source": color_source[occ.normalized],
            }
        )

    return {
        "meta": {
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "mode": mode,
            "scope": {
                "root": "ui/src",
                "extensions": [".module.css", ".tsx"],
                "excluded": sorted(EXCLUDED_FILES),
            },
            "files_scanned": files_scanned,
            "matches": len(occurrences),
            "unique_colors": len(freq),
            "reused_tokens": sum(1 for s in color_source.values() if s == "existing"),
            "new_tokens": sum(1 for s in color_source.values() if s == "generated"),
        },
        "mappings": mapping_rows,
        "occurrences": occurrence_rows,
    }


def build_markdown_report(
    mode: str,
    files_scanned: int,
    occurrences: Sequence[Occurrence],
    color_to_token: Dict[str, str],
    color_source: Dict[str, str],
    freq: Counter[str],
) -> str:
    unique_files_with_matches = len({o.rel_path for o in occurrences})
    raw_variants: Dict[str, set[str]] = defaultdict(set)
    raw_to_normalized: Dict[str, str] = {}
    for occ in occurrences:
        raw_to_normalized[occ.raw] = occ.normalized
        raw_variants[occ.normalized].add(occ.raw)

    lines = [
        "# Color Tokenization Report",
        "",
        "## Summary",
        f"- Mode: `{mode}`",
        f"- Files scanned: `{files_scanned}`",
        f"- Files with matches: `{unique_files_with_matches}`",
        f"- Total color matches: `{len(occurrences)}`",
        f"- Unique normalized colors: `{len(freq)}`",
        f"- Reused existing tokens: `{sum(1 for s in color_source.values() if s == 'existing')}`",
        f"- New generated tokens: `{sum(1 for s in color_source.values() if s == 'generated')}`",
        "",
        "## Top Colors",
        "",
        "| Normalized | Occurrences | Replacement | Source |",
        "|---|---:|---|---|",
    ]

    for normalized in sorted(freq.keys(), key=lambda c: (-freq[c], c)):
        lines.append(
            f"| `{normalized}` | {freq[normalized]} | "
            f"`var({color_to_token[normalized]})` | {color_source[normalized]} |"
        )

    lines.extend(
        [
            "",
            "## Literal Mapping",
            "",
        ]
    )

    for raw in sorted(raw_to_normalized.keys(), key=lambda r: (raw_to_normalized[r], r.lower(), r)):
        normalized = raw_to_normalized[raw]
        token = color_to_token[normalized]
        lines.append(f"- `{raw}` -> `var({token})`")

    lines.extend(
        [
            "",
            "## Normalized Mapping",
            "",
            "| Normalized | Raw Samples | Replacement | Source |",
            "|---|---|---|---|",
        ]
    )
    for normalized in sorted(freq.keys(), key=lambda c: (-freq[c], c)):
        sample = ", ".join(f"`{s}`" for s in sorted(raw_variants[normalized], key=str.lower))
        lines.append(
            f"| `{normalized}` | {sample} | `var({color_to_token[normalized]})` | "
            f"{color_source[normalized]} |"
        )

    lines.append("")
    return "\n".join(lines)


def write_json(path: Path, payload: Dict[str, object]) -> None:
    ensure_parent(path)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    ensure_parent(path)
    path.write_text(text, encoding="utf-8")


def main() -> int:
    args = parse_args()
    mode = mode_name(args)

    if not SRC_ROOT.exists():
        print(f"error: missing source root at {SRC_ROOT}", file=sys.stderr)
        return 1
    if not VARIABLES_CSS.exists():
        print(f"error: missing variables.css at {VARIABLES_CSS}", file=sys.stderr)
        return 1

    targets = discover_target_files(SRC_ROOT)
    file_texts: Dict[str, str] = {}
    file_occurrences: Dict[str, List[Occurrence]] = {}
    all_occurrences: List[Occurrence] = []

    for path in targets:
        rel_path = path.relative_to(REPO_ROOT).as_posix()
        text, occurrences = collect_occurrences_for_file(path)
        file_texts[rel_path] = text
        file_occurrences[rel_path] = occurrences
        all_occurrences.extend(occurrences)

    existing_index = build_existing_token_index(VARIABLES_CSS)
    declared_token_names = build_declared_var_name_set(VARIABLES_CSS)
    color_to_token, color_source, freq, generated_tokens = assign_tokens(
        all_occurrences,
        existing_index,
        declared_token_names,
    )
    changed_files = rewrite_contents(file_texts, file_occurrences, color_to_token)

    variables_rel = VARIABLES_CSS.relative_to(REPO_ROOT).as_posix()
    if generated_tokens:
        variables_before = VARIABLES_CSS.read_text(encoding="utf-8")
        variables_after = append_tokens_to_variables_css(variables_before, generated_tokens)
        if variables_before != variables_after:
            changed_files[variables_rel] = (variables_before, variables_after)

    report_payload = build_json_report(
        mode=mode,
        files_scanned=len(targets),
        occurrences=all_occurrences,
        color_to_token=color_to_token,
        color_source=color_source,
        freq=freq,
    )
    markdown_report = build_markdown_report(
        mode=mode,
        files_scanned=len(targets),
        occurrences=all_occurrences,
        color_to_token=color_to_token,
        color_source=color_source,
        freq=freq,
    )

    write_json(REPORT_JSON, report_payload)
    write_text(REPORT_MD, markdown_report)

    if mode in {"test", "apply", "write"}:
        write_text(REPORT_PATCH, build_patch(changed_files))

    if mode == "test":
        variables_before = VARIABLES_CSS.read_text(encoding="utf-8")
        variables_after = append_tokens_to_variables_css(variables_before, generated_tokens)
        write_text(
            GENERATED_VARIABLES_CSS,
            build_generated_header("Preview of variables.css after token append (test mode).")
            + variables_after,
        )

    if mode in {"apply", "write"}:
        for rel_path in sorted(changed_files.keys()):
            _, after = changed_files[rel_path]
            write_text(REPO_ROOT / rel_path, after)

    print(
        f"[{mode}] scanned={len(targets)} matches={len(all_occurrences)} "
        f"unique={len(freq)} reused={sum(1 for s in color_source.values() if s == 'existing')} "
        f"new={sum(1 for s in color_source.values() if s == 'generated')}"
    )
    print(f"report_json={REPORT_JSON.relative_to(REPO_ROOT).as_posix()}")
    print(f"report_md={REPORT_MD.relative_to(REPO_ROOT).as_posix()}")
    if mode in {"test", "apply", "write"}:
        print(f"patch_preview={REPORT_PATCH.relative_to(REPO_ROOT).as_posix()}")
    if mode == "test":
        print(
            f"generated_variables={GENERATED_VARIABLES_CSS.relative_to(REPO_ROOT).as_posix()}"
        )
        print("source_writes=0 (test mode)")
    if mode in {"apply", "write"}:
        print("append_targets=ui/src/styles/variables.css")
        print(f"source_writes={len(changed_files)}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
