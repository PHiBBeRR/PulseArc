#!/usr/bin/env python3
import json
import math
import pathlib
import sys

if len(sys.argv) != 2:
    print("usage: export_samples.py <output.csv>", file=sys.stderr)
    sys.exit(1)

out_path = pathlib.Path(sys.argv[1])
root = pathlib.Path('target/criterion')
rows = [("group","bench","p50_ns","p99_ns")]

if not root.exists():
    print("target/criterion directory not found", file=sys.stderr)
    sys.exit(1)

for group_dir in sorted(root.iterdir()):
    if not group_dir.is_dir():
        continue
    group = group_dir.name
    for bench_dir in sorted(group_dir.iterdir()):
        sample = bench_dir / 'new' / 'sample.json'
        if not sample.exists():
            continue
        data = json.load(sample.open())
        per_iter = [t / i for t, i in zip(data['times'], data['iters']) if i]
        if not per_iter:
            continue
        per_iter.sort()
        n = len(per_iter)
        def percentile(p: float) -> float:
            return per_iter[int((n - 1) * p)]
        rows.append((group, bench_dir.name, f"{percentile(0.5):.6f}", f"{percentile(0.99):.6f}"))

out_path.parent.mkdir(parents=True, exist_ok=True)
with out_path.open('w') as f:
    for row in rows:
        f.write(','.join(row) + '\n')
