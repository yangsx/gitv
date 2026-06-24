#!/usr/bin/env python3
"""
check_regressions.py
====================
Parses criterion benchmark text output and applies noise-tolerant regression
detection.

Only fails if:
  - >= 2 benchmarks regressed in the same file (unlikely to be noise)
  - OR a single benchmark regressed by > THRESHOLD_PCT (default 15%)

Minor regressions (single benchmark under threshold) are reported as warnings
and do not fail the job.

Usage:
    python3 scripts/check_regressions.py <file1> [file2] ...
"""

import re
import sys
from pathlib import Path

THRESHOLD_PCT = 15.0


def parse_regressions(text: str) -> list[tuple[str, float]]:
    """
    Extract (benchmark_name, pct) pairs from criterion text output.

    Criterion prints "Performance has regressed." after a "change:" line:
        <name>   time:   [...]
                        change: [+lo% +mid% +hi%] (p = ...)
                        Performance has regressed.

    The point estimate is the middle value (mid).
    """
    regressions: list[tuple[str, float]] = []
    lines = text.splitlines()

    for i, line in enumerate(lines):
        if "Performance has regressed" not in line:
            continue

        pct = None
        for j in range(i - 1, max(0, i - 3) - 1, -1):
            m = re.search(r"change:\s*\[([^\]]+)\]", lines[j])
            if m:
                values = re.findall(r"[+-]?\d+\.?\d*%", m.group(1))
                if len(values) >= 2:
                    pct = float(values[1].rstrip("%"))
                break

        name = "<unknown>"
        for j in range(i - 1, max(0, i - 5) - 1, -1):
            stripped = lines[j].strip()
            if "time:" in stripped:
                name_part = stripped.split("time:")[0].strip()
                if name_part:
                    name = name_part
                elif j > 0:
                    prev = lines[j - 1].strip()
                    if prev and "change:" not in prev and "time:" not in prev:
                        name = prev
                break

        regressions.append((name, pct or 0.0))

    return regressions


def check_file(path: Path) -> bool:
    """
    Check a single criterion output file for regressions.
    Returns True if the file indicates a real regression (should fail CI).
    """
    text = path.read_text()
    regressions = parse_regressions(text)

    if not regressions:
        return False

    file_label = path.name

    if len(regressions) >= 2:
        print(f"::error::{len(regressions)} benchmarks regressed in {file_label} "
              f"(multiple = likely real)")
        for name, pct in regressions:
            print(f"  {name}: {pct:+.1f}%")
        return True

    name, pct = regressions[0]
    abs_pct = abs(pct)

    if abs_pct > THRESHOLD_PCT:
        print(f"::error::{name} regressed {pct:+.1f}% in {file_label} "
              f"(exceeds {THRESHOLD_PCT:.0f}% threshold)")
        return True

    print(f"::warning::Ignoring minor regression {name} {pct:+.1f}% "
          f"in {file_label} (under {THRESHOLD_PCT:.0f}% threshold)")
    return False


def main() -> int:
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <criterion-output.txt> [...]",
              file=sys.stderr)
        return 2

    failed = False
    for arg in sys.argv[1:]:
        path = Path(arg)
        if not path.exists():
            print(f"Warning: {arg} not found, skipping", file=sys.stderr)
            continue
        if check_file(path):
            failed = True

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
