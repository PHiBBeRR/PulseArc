#!/usr/bin/env python3
"""
Collate Criterion benchmark samples into a single CSV summary.

The script prefers raw per-iteration samples (sample.json) so we can derive
both p50 and p99 latency in nanoseconds. When samples are missing it falls
back to the point estimate from estimates.json and marks p99 as NA.
"""

import csv
import json
import math
import sys
from pathlib import Path

ROOT = Path("target/criterion")
OUT_PATH = Path("target/criterion-summary.csv")


def percentile(values, frac):
    if not values:
        return None

    if len(values) == 1:
        return values[0]

    position = (len(values) - 1) * frac
    lower_idx = math.floor(position)
    upper_idx = math.ceil(position)

    lower = values[lower_idx]
    upper = values[upper_idx]

    if lower_idx == upper_idx:
        return lower

    weight = position - lower_idx
    return lower + (upper - lower) * weight


def format_duration(ns_value):
    if ns_value is None:
        return "NA"

    if ns_value < 1_000:
        return f"{ns_value:.1f} ns"
    if ns_value < 1_000_000:
        return f"{ns_value / 1_000:.2f} µs"
    if ns_value < 1_000_000_000:
        return f"{ns_value / 1_000_000:.3f} ms"
    return f"{ns_value / 1_000_000_000:.3f} s"


def derive_samples(sample_path):
    if not sample_path.exists():
        return None, None

    data = json.loads(sample_path.read_text())
    times = data.get("times", [])
    iters = data.get("iters", [])

    per_iter = [
        time / itr for time, itr in zip(times, iters) if itr and itr != 0
    ]
    per_iter.sort()

    return percentile(per_iter, 0.50), percentile(per_iter, 0.99)


def derive_estimates(estimates_path):
    if not estimates_path.exists():
        return None, None

    data = json.loads(estimates_path.read_text())
    median = data.get("Median", {})
    return median.get("point_estimate"), None


def collect_rows():
    rows = []

    for estimates_path in ROOT.glob("**/new/estimates.json"):
        bench_dir = estimates_path.parent
        bench_name = bench_dir.parent.name
        group_name = bench_dir.parent.parent.name

        sample_path = bench_dir / "sample.json"
        p50, p99 = derive_samples(sample_path)
        if p50 is None:
            p50, p99 = derive_estimates(estimates_path)

        rows.append(
            (
                group_name,
                bench_name,
                format_duration(p50),
                format_duration(p99),
            )
        )

    rows.sort(key=lambda row: (row[0], row[1]))
    return rows


def main():
    if not ROOT.exists():
        print("target/criterion directory not found", file=sys.stderr)
        sys.exit(1)

    rows = collect_rows()
    if not rows:
        print("No Criterion results found.", file=sys.stderr)
        sys.exit(1)

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUT_PATH.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["group", "bench", "p50", "p99"])
        writer.writerows(rows)

    print(f"Wrote {OUT_PATH}")


if __name__ == "__main__":
    main()
