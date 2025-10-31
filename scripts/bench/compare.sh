#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <baseline-name> [-- <extra cargo bench args>]" >&2
  exit 1
fi

BASELINE=$1
shift || true

CARGO_ARGS=(cargo bench -p infra-baselines --bench baseline -- --baseline "$BASELINE")
if [[ $# -gt 0 ]]; then
  CARGO_ARGS+=("$@")
fi

"${CARGO_ARGS[@]}"
