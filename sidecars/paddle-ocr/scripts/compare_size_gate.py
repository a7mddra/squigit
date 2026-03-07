#!/usr/bin/env python3
# Copyright 2026 a7mddra
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load_payload(path: str) -> dict:
    p = Path(path).resolve()
    if not p.exists():
        raise SystemExit(f"Size payload not found: {p}")
    return json.loads(p.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare OCR compressed payload size against baseline gate."
    )
    parser.add_argument("--baseline", required=True, help="Baseline JSON payload path")
    parser.add_argument(
        "--candidate", required=True, help="Candidate JSON payload path"
    )
    parser.add_argument(
        "--max-ratio",
        type=float,
        default=0.65,
        help="Maximum allowed candidate/baseline compressed ratio (default: 0.65)",
    )
    args = parser.parse_args()

    baseline = load_payload(args.baseline)
    candidate = load_payload(args.candidate)

    base_c = int(baseline.get("compressed_bytes", 0))
    cand_c = int(candidate.get("compressed_bytes", 0))
    if base_c <= 0:
        raise SystemExit("Invalid baseline compressed_bytes (must be > 0)")

    ratio = cand_c / base_c
    allowed = base_c * args.max_ratio
    print(
        f"Compressed size gate: candidate={cand_c} baseline={base_c} "
        f"ratio={ratio:.4f} max_ratio={args.max_ratio:.4f} allowed={int(allowed)}"
    )

    if ratio > args.max_ratio:
        raise SystemExit(
            "Compressed size gate failed: "
            f"{cand_c} > {int(allowed)} (ratio {ratio:.4f} > {args.max_ratio:.4f})"
        )

    print("Compressed size gate passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
