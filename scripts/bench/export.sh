#!/usr/bin/env bash
set -euo pipefail

DIR="target/criterion/latest"
CSV="target/criterion/export.csv"
mkdir -p "$DIR"

# Run the full suite and save as "latest" baseline
CARGO_TARGET_DIR=target \
PULSARC_TEST_DB_KEY=${PULSARC_TEST_DB_KEY:-test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa} \
PULSARC_ENABLE_MAC_BENCH=${PULSARC_ENABLE_MAC_BENCH:-} \
cargo bench -p infra-baselines --bench baseline -- --save-baseline latest "$@"

# Collate all sample.json files into a CSV summary
python3 scripts/bench/export_samples.py "$CSV"

echo "Exported Criterion results to $CSV"
