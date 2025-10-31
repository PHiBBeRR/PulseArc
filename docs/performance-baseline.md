# Legacy Infrastructure Performance Baseline

_Collected on 2025-10-31 using the `infra-baselines` Criterion harness._

## Environment
- Machine: macOS host (local developer workstation)
- Database: SQLCipher (via shimmed legacy `DbManager`)
- HTTP: Legacy retry client hitting local Hyper servers (no proxy)
- Activity Provider: Skipped in automated run (Accessibility permission required)
- Command: `cargo bench -p infra-baselines --offline`
- Test key: `PULSARC_TEST_DB_KEY=test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`

## Results

| Benchmark | p50 | Notes |
| --- | --- | --- |
| `legacy_db_manager/save_snapshot_single` | **≈50.2 µs** | Single-row insert into `activity_snapshots` via pooled SQLCipher connection |
| `legacy_db_manager/time_range_query_day_100_snapshots` | **≈47.3 µs** | 24-hour range scan returning 100 rows |
| `legacy_db_manager/bulk_insert_1000_snapshots` | **≈3.26 ms** | Transactional bulk insert (1,000 rows) using prepared statement reuse |
| `legacy_http_client/single_request` | **≈62.8 µs** | One successful request to local HTTP server |
| `legacy_http_client/request_with_retry` | **≈1.003 s** | Induced 500 → 200 path including retry/backoff sleep |
| `legacy_macos_activity_provider/*` | _Skipped_ | Run manually with Accessibility permission (`System Settings → Privacy & Security → Accessibility`) |

## Running the Benchmarks

```bash
# Ensure SQLCipher uses a deterministic key
export PULSARC_TEST_DB_KEY=test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

# Run benches (no network fetches required)
cargo bench -p infra-baselines --offline
```

### macOS Activity Provider
1. Grant Accessibility permission to the benchmark binary (\`target/release/deps/baseline-*\`).
2. Re-run the suite without `--offline` if Accessibility prompts need to surface interactively.
3. Record p50 latency for:
   - Fetch without enrichment
   - Fetch with enrichment

## Usage Notes
- The shim crate under `benchmarks/infra-baselines/legacy-shim` re-exports the legacy SQLCipher manager, HTTP client, and macOS provider in isolation so the frozen `legacy/api` tree remains untouched.
- HTTP backoff timing (~1 s) reflects the configured retry-after logic; adjust the retry configuration if future comparisons need shorter failure windows.
- These numbers serve as the baseline for Phase 3A migrations. Capture the same metrics for the new infra implementations and document deltas in this file.
