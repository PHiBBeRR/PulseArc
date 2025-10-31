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

| Area | Scenario | p50 | p99 | Notes |
| --- | --- | ---:| ---:| --- |
| **Database (legacy)** | Single insert | 56.3 µs | 66.7 µs | Insert one `activity_snapshot` via pooled SQLCipher |
|  | 1-day range query | 49.8 µs | 51.7 µs | `[start, end)` query returning 100 rows |
|  | Bulk insert (1 000) | 3.49 ms | 4.11 ms | Transactional batch with prepared statement reuse |
| **HTTP (legacy)** | Single request | 64.1 µs | 73.1 µs | Warm connection against local Hyper server |
|  | Retry path (transient 5xx) | 1.003 s | 1.003 s | Includes backoff + retry-after sleep |
| **MDM (new)** | `MdmClient::fetch_config` (warm) | 61.6 µs | 66.0 µs | TLS session reused (loopback) |
|  | `MdmClient::fetch_and_merge` (warm) | 62.2 µs | 65.4 µs | Remote fetch merged into baseline config |
|  | `MdmClient::fetch_config` (cold TLS) | 3.88 ms | 4.23 ms | Fresh client per call (handshake cost) |
| **macOS Provider (new)** | Fetch (AX granted) | 0.97 ms | 1.22 ms | Window/title capture with Accessibility permission |
|  | Fetch + enrichment (AX granted) | 0.99 ms | 1.19 ms | Adds synchronous enrichment attempt |
|  | Fetch (AX forced off) | 0.99 ms | 1.10 ms | `PULSARC_FORCE_AX_DENIED=1` fallback path |

## Running the Benchmarks

```bash
# Ensure SQLCipher uses a deterministic key and enable macOS activity benchmarks
export PULSARC_TEST_DB_KEY=test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
export PULSARC_ENABLE_MAC_BENCH=1

# Optional: point at your MDM cert bundle (defaults to workspace/.mdm-certs)
# export PULSARC_MDM_CERT_DIR=/path/to/.mdm-certs

cargo bench -p infra-baselines --offline

# Compare against a saved Criterion baseline (e.g. nightly CI snapshot)
scripts/bench/compare.sh 20251031
```

### macOS Activity Provider
1. Grant Accessibility permission to the benchmark binary (`target/release/deps/baseline-*`).
2. Ensure `PULSARC_ENABLE_MAC_BENCH=1` is set so the macOS suite runs.
3. The harness also executes a fallback run with `PULSARC_FORCE_AX_DENIED=1` to capture the “AX denied” latency profile automatically.

### MDM Client
1. Generate self-signed certs via `scripts/mdm/generate-test-certs.sh` (creates `.mdm-certs/`).
2. Trust the `ca-cert.pem` locally if you want to hit the endpoint outside the benches.
3. The harness spins up a TLS server using `server-fullchain.pem` / `server-key.pem` and exercises `MdmClient::with_ca_cert`.
4. Two scenarios are measured: warm session reuse and a cold TLS handshake (new client per iteration).

## Usage Notes
- The shim crate under `benchmarks/infra-baselines/legacy-shim` re-exports the legacy SQLCipher manager, HTTP client, macOS provider, and MDM client/config structures so the frozen `legacy/api` tree remains untouched.
- HTTP backoff timing (~1 s) reflects the configured retry-after logic; adjust the retry configuration if future comparisons need shorter failure windows.
- These numbers serve as the baseline for Phase 3A migrations. Capture the same metrics for the new infra implementations (including the forthcoming MDM adapter) and document deltas in this file.
