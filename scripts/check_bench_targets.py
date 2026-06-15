#!/usr/bin/env python3
"""
check_bench_targets.py
======================
Parses criterion benchmark JSON results and checks them against the absolute
performance targets defined in AGENTS.md / requirements.md (Req 27.6).

Usage:
    python3 scripts/check_bench_targets.py <criterion-dir>

criterion-dir:  path that contains <bench-name>/*/new/estimates.json files
                (typically crates/gitv-git-core/target/criterion)

Exit codes:
    0  all targets met
    1  one or more targets exceeded — details printed and written to
       $GITHUB_STEP_SUMMARY (when running in GitHub Actions)
"""

import json
import os
import sys
from pathlib import Path
from typing import NamedTuple


# ---------------------------------------------------------------------------
# Absolute performance targets (from AGENTS.md)
# ---------------------------------------------------------------------------
# Each entry maps a (bench_name_glob, test_name_glob) tuple to a budget.
# The budget is in the SAME unit that criterion stores:
#   - time benchmarks  → nanoseconds  (criterion mean point estimate)
#   - memory benchmarks → bytes       (dhat peak_bytes, parsed from stdout)
#
# Criterion stores time in nanoseconds in estimates.json.
#
# Targets:
#   Search (indexed, 100k commits)   < 100 ms   → 100_000_000 ns
#   Graph layout (10k commits)       < 5 s total load budget, layout alone
#                                      budgeted at 2 s → 2_000_000_000 ns
#   Index build (100k)               < 5 s      → 5_000_000_000 ns
# ---------------------------------------------------------------------------

NS_PER_MS = 1_000_000
NS_PER_S  = 1_000_000_000

class Target(NamedTuple):
    bench: str          # substring that must appear in benchmark group name
    test:  str          # substring that must appear in function/test name
    budget_ns: float    # maximum allowed nanoseconds (mean point estimate)
    label: str          # human-readable description shown in the report


TARGETS: list[Target] = [
    Target(
        bench="search_query_100k",
        test="text_exact",
        budget_ns=100 * NS_PER_MS,
        label="Search (text, 100k commits) < 100 ms",
    ),
    Target(
        bench="search_query_100k",
        test="text_regex",
        budget_ns=100 * NS_PER_MS,
        label="Search (regex, 100k commits) < 100 ms",
    ),
    Target(
        bench="search_query_100k",
        test="author",
        budget_ns=100 * NS_PER_MS,
        label="Search (author, 100k commits) < 100 ms",
    ),
    Target(
        bench="search_index_build",
        test="build/100000",
        budget_ns=5 * NS_PER_S,
        label="Search index build (100k commits) < 5 s",
    ),
    Target(
        bench="graph_layout",
        test="linear/10000",
        budget_ns=2 * NS_PER_S,
        label="Graph layout linear (10k commits) < 2 s",
    ),
    Target(
        bench="graph_layout_branchy",
        test="branchy/10000",
        budget_ns=2 * NS_PER_S,
        label="Graph layout branchy (10k commits) < 2 s",
    ),
]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def find_estimates(criterion_dir: Path) -> dict[tuple[str, str], float]:
    """
    Walk criterion_dir and collect (group, test_path) → mean_ns mappings.

    Criterion stores results at:
        <criterion_dir>/<group>/<function>[/<param>]/new/estimates.json

    The test_path captures everything between group and new/estimates.json,
    so parameterized benchmarks like linear/10000 and build/100000 get
    distinct keys.
    """
    results: dict[tuple[str, str], float] = {}
    for estimates_path in criterion_dir.rglob("new/estimates.json"):
        try:
            data = json.loads(estimates_path.read_text())
            mean_ns: float = data["mean"]["point_estimate"]
            parts = estimates_path.relative_to(criterion_dir).parts
            if len(parts) >= 3:
                group = parts[0]
                test_path = "/".join(parts[1:-2])
                results[(group, test_path)] = mean_ns
        except (KeyError, json.JSONDecodeError, ValueError):
            pass
    return results


def format_duration(ns: float) -> str:
    if ns >= NS_PER_S:
        return f"{ns / NS_PER_S:.3f} s"
    if ns >= NS_PER_MS:
        return f"{ns / NS_PER_MS:.3f} ms"
    return f"{ns / 1000:.3f} µs"


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <criterion-dir>", file=sys.stderr)
        return 2

    criterion_dir = Path(sys.argv[1])
    if not criterion_dir.is_dir():
        print(f"Error: {criterion_dir} is not a directory", file=sys.stderr)
        return 2

    results = find_estimates(criterion_dir)

    rows_pass: list[str] = []
    rows_fail: list[str] = []
    missing:   list[str] = []

    for target in TARGETS:
        # Find a matching (group, test_path) pair.
        match: tuple[str, str] | None = None
        for (group, test_path) in results:
            if target.bench in group and target.test in test_path:
                match = (group, test_path)
                break

        if match is None:
            missing.append(
                f"| ⚠️  MISSING | {target.label} | — | {format_duration(target.budget_ns)} | — |"
            )
            continue

        mean_ns = results[match]
        ratio   = mean_ns / target.budget_ns
        pct     = (ratio - 1.0) * 100.0

        row = (
            f"| {{status}} | {target.label} "
            f"| {format_duration(mean_ns)} "
            f"| {format_duration(target.budget_ns)} "
            f"| {{diff}} |"
        )

        if mean_ns <= target.budget_ns:
            rows_pass.append(row.format(
                status="✅ PASS",
                diff=f"{abs(pct):.1f}% under budget",
            ))
        else:
            rows_fail.append(row.format(
                status="❌ FAIL",
                diff=f"{pct:.1f}% OVER budget",
            ))

    # ------------------------------------------------------------------
    # Build the Markdown report
    # ------------------------------------------------------------------
    header = (
        "## Performance Target Report\n\n"
        "| Status | Benchmark | Measured | Budget | vs Budget |\n"
        "|--------|-----------|----------|--------|-----------|\n"
    )
    table_rows = rows_pass + rows_fail + missing
    report = header + "\n".join(table_rows) + "\n"

    if rows_fail:
        report += (
            f"\n> **{len(rows_fail)} target(s) exceeded budget.**  "
            "See details above.\n"
        )
    else:
        report += "\n> **All targets met ✅**\n"

    if missing:
        report += (
            f"\n> **{len(missing)} benchmark(s) not found** in criterion output. "
            "They may not have run yet.\n"
        )

    # Print to stdout (always visible in CI logs).
    print(report)

    # Write to GitHub Actions job summary when running in CI.
    summary_file = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_file:
        with open(summary_file, "a") as f:
            f.write(report)

    return 1 if rows_fail else 0


if __name__ == "__main__":
    sys.exit(main())
