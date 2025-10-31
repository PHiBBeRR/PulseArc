#!/usr/bin/env bash
set -euo pipefail

if [[ $(uname -s) != "Darwin" ]]; then
  echo "This helper is intended for macOS hosts." >&2
  exit 1
fi

echo "▶ Building benchmark binary so macOS can register its signature (no benches run)…"
PULSARC_ENABLE_MAC_BENCH=1 cargo bench -p infra-baselines --bench baseline --no-run >/dev/null

echo "▶ Opening System Settings › Privacy & Security › Accessibility"
open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"

cat <<'MSG'

Next steps:
  1. In the Accessibility list, enable the `baseline` benchmark binary that just appeared.
  2. Re-run the benches when ready:
       PULSARC_ENABLE_MAC_BENCH=1 cargo bench -p infra-baselines --bench baseline

The harness always records an AX-denied trace by temporarily applying
PULSARC_FORCE_AX_DENIED=1 around that group, so you'll capture both paths.
MSG
