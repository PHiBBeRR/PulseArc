# Legacy Infrastructure Performance Baseline

_Collected on 2025-10-31 using the `infra-baselines` Criterion harness._

## Environment
- Machine: macOS host (local developer workstation)
- Database: SQLCipher (via shimmed legacy `DbManager`)
- HTTP: Legacy retry client hitting local Hyper servers (no proxy)
- Activity Provider: macOS AX capture (requires Accessibility permission)
- MDM: Local HTTPS server backed by `.mdm-certs` self-signed CA
- Command: `PULSARC_TEST_DB_KEY=… PULSARC_ENABLE_MAC_BENCH=1 cargo bench -p infra-baselines --offline`
- Test key: `PULSARC_TEST_DB_KEY=test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`

## Results

### Database (legacy)

| Scenario | p50 | p99 | Notes |
| --- | ---:| ---:| --- |
| Single insert | 55.0 µs | 66.7 µs | Insert one `activity_snapshot` via pooled SQLCipher |
| 1-day range query | 48.9 µs | 55.4 µs | `[start, end)` query returning 100 rows |
| Bulk insert (1 000) | 3.58 ms | 4.19 ms | Transactional batch with prepared statement reuse |

### HTTP (legacy)

| Scenario | p50 | p99 | Notes |
| --- | ---:| ---:| --- |
| Single request | 63.9 µs | 90.8 µs | Warm connection against local Hyper server |
| Retry path (transient 5xx) | 1.002 s | 1.003 s | Includes backoff + retry-after sleep |

### MDM Client (legacy shim)

| Scenario | Mode | p50 | p99 | Δ vs warm (p50) | Δ vs warm (p99) | Notes |
| --- | --- | ---:| ---:| ---:| ---:| --- |
| `fetch_config` | Warm | 62.5 µs | 66.2 µs | baseline | baseline | TLS session reused (loopback) |
| `fetch_and_merge` | Warm | 63.3 µs | 68.6 µs | — | — | Remote fetch merged into baseline config |
| `fetch_config` | Cold TLS | 3.03 ms | 3.17 ms | +2.97 ms | +3.11 ms | Fresh client per iteration (`no_pool` + `fresh_tls_config`) |

### macOS Activity Provider (legacy)

| Scenario | AX-off p50 | AX-on p50 | Δ p50 | AX-off p99 | AX-on p99 | Δ p99 | Notes |
| --- | ---:| ---:| ---:| ---:| ---:| ---:| --- |
| Activity fetch | 0.11 µs | 956 µs | +955 µs | 0.14 µs | 1.18 ms | +1.18 ms | Window/title capture with Accessibility permission |
| Activity fetch + enrichment | — | 952 µs | — | — | 1.24 ms | — | Adds synchronous enrichment attempt; skips when AX is denied |
| Activity fetch (forced denied) | 0.11 µs | — | baseline | 0.14 µs | — | baseline | Harness enforces `PULSARC_FORCE_AX_DENIED=1`; fallback returns immediately |

## Running the Benchmarks

```bash
# Optional: override the deterministic SQLCipher key used by the shim
export PULSARC_TEST_DB_KEY=test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

# Warm benches (DB/HTTP/MDM warm paths + macOS AX-off)
make bench

# macOS Accessibility-on variants (requires prior Accessibility grant)
make mac-bench
```

- `make mac-bench-prep` builds the bench binary (no run) and opens System Settings so you can grant Accessibility once.
- `make bench-save` captures a Criterion baseline (defaults to the current git SHA); `make bench-diff BASELINE=20251031` compares against a saved run.
- `make bench-csv` runs `scripts/bench/criterion_to_csv.py` and emits `target/criterion-summary.csv` with p50/p99 snapshots for every group.
- The harness auto-detects `.mdm-certs/`; override with `PULSARC_MDM_CERT_DIR` if you keep certs elsewhere. All commands run without a network connection.

### macOS Activity Provider
1. Run `make mac-bench-prep` once per machine. It builds the bench binary (no execution) and launches System Settings → Privacy & Security → Accessibility so you can grant access.
2. `make mac-bench` sets `PULSARC_ENABLE_MAC_BENCH=1` for you and only registers the AX-on group when Accessibility is already granted. Otherwise it logs a single skip hint (`[macos benches] AX-on skipped…`) and continues.
3. The harness always executes a fallback run with `PULSARC_FORCE_AX_DENIED=1`, so the AX-denied profile is captured even if permission is granted.
4. On managed CI hardware, provision Accessibility via your real MDM profile and set `PULSARC_ENABLE_MAC_BENCH=1` in the job so the AX-on group runs unattended.

### MDM Client
1. Generate self-signed certs via `scripts/mdm/generate-test-certs.sh` (creates `.mdm-certs/`).
2. Trust the `ca-cert.pem` locally if you want to hit the endpoint outside the benches (optional); the benches pass it directly.
3. The harness spins up a TLS server using `server-fullchain.pem` / `server-key.pem` and exercises `MdmClient::with_ca_cert` for the warm path.
4. The cold TLS path constructs a fresh client per iteration with `MdmClient::builder().no_pool().fresh_tls_config()`, disabling connection pooling and TLS session resumption to capture handshake cost.

## Usage Notes
- The shim crate under `benchmarks/infra-baselines/legacy-shim` re-exports the legacy SQLCipher manager, HTTP client, macOS provider, and MDM client/config structures so the frozen `legacy/api` tree remains untouched.
- HTTP backoff timing (~1 s) reflects the configured retry-after logic; adjust the retry configuration if future comparisons need shorter failure windows.
- `target/criterion-summary.csv` captures p50/p99 snapshots for every group (generated by `make bench-csv`) and can be attached to CI artifacts for diffing.
- These numbers serve as the baseline for Phase 3A migrations. Capture the same metrics for the new infra implementations (including the forthcoming MDM adapter) and document deltas in this file.
