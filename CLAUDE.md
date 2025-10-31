# CLAUDE.md — PulseArc Rust Workspace Rules (Strict)

These are **non‑negotiable rules** for agents (and humans) working in the PulseArc Rust monorepo.
They assume the workspace is configured as follows: Rust **1.77** (stable), `publish = false`,
workspace dependencies, `tracing` for logs, and strict profiles (debug: unwind; release: abort; overflow checks on).

If a task conflicts with these rules, **stop and request human approval** with a short rationale.

---

## 1) Toolchain & Build
- Use `rustc` **1.77** (pinned by `rust-toolchain.toml`). Do **not** change toolchain/channel.
- Build with the workspace root only. Do **not** inject per-crate profiles.
- Respect profiles:
  - **dev/test**: `panic = unwind`, `debug = 2`, `overflow-checks = true`.
  - **release/bench**: `panic = abort`, ThinLTO, `codegen-units = 1`, `opt-level = 3`, `overflow-checks = true`.
- Never disable `overflow-checks` or `-D warnings` flags.

## 2) Dependencies (Supply‑Chain)
- Prefer existing **`[workspace.dependencies]`**. Add deps there; use `*.workspace = true` in member crates.
- **Forbidden:** wildcards (`"*"`) • unknown git sources • yanked crates • unlicensed/unknown licenses.
- New dependency policy:
  1. Confirm license is **allow‑listed** in `deny.toml`.
  2. Add minimal features only; avoid enabling large `full` feature sets.
  3. Run `cargo deny check` and `cargo audit` locally; include outputs in the PR.
- Do **not** publish crates (`publish = false` stays). External path/git deps require human approval with justification.

## 3) Logging & Observability
- Use **`tracing`** exclusively. No `println!` and no `log::*` macros.
- Structure every log with fields (e.g., `info!(user_id, op = "create", ...)`). Favor spans for request/Task scopes.
- **Never** log secrets, tokens, credentials, or PII. Redact or hash identifiers when possible.
- Production output must be JSON via `tracing-subscriber` with `env-filter` controls.

## 4) Errors & Panics
- Library crates: prefer `thiserror` for typed errors. Application boundaries: use `anyhow` only at the **outermost** layer.
- **Disallowed in non‑test code:** `unwrap()`, `expect()`, `panic!()` (except truly impossible cases with proof).
- Convert `Option`/`Result` explicitly; bubble errors upward; never swallow errors.
- Use `Result<T, E>` returns from public async fns; document error variants and expected recovery paths.

## 5) Async & Concurrency
- Runtime: **Tokio** (multi‑thread). No blocking inside async contexts.
  - Use `tokio::task::spawn_blocking` for CPU‑heavy or blocking IO.
  - Track every spawned task via handles; do not detach fire‑and‑forget work.
  - Use timeouts (`tokio::time::timeout`) and cancellation (`select!`, `CancellationToken`) for all external calls.
- Avoid global mutable state. Prefer passing contexts; guard shared state with `Arc<Mutex/RwLock>` only when necessary.

## 6) Testing
- Must include **unit tests** and, when applicable, **integration tests**.
- Async tests use `#[tokio::test(flavor = "multi_thread")]`.
- Tests must be deterministic (no network, clock, or randomness without seeding/mocking).
- Coverage of error paths is required for new logic (happy path + at least one failure path).

## 7) Lints, Style, and Formatting
- Formatting: `cargo fmt --all -- --check` must pass.
- Lints: `cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery` must pass.
- Do **not** add `#[allow(...)]` except with a **commented justification** and a TODO/issue link.
- Disallow `unsafe` by default. Any `unsafe` must be isolated, documented, and covered by tests.

## 8) API & Crate Boundaries
- Public APIs are minimal and documented (`///` + examples). Avoid `pub use` re‑exports without rationale.
- Keep semver‑safe changes; breaking API changes require a migration note.
- New crate setup:
  - Lives under `crates/<name>/` with `version/edition/rust-version/publish` inherited from the workspace.
  - Dependencies reference workspace entries via `.workspace = true`.

## 9) Configuration & Secrets
- No secrets in code, tests, or logs. Load configuration from env/files; validate at startup with clear errors.
- Use `serde` for config structs and implement a `validate()` step for ranges/URLs/credentials presence.

## 10) CI Gates (PR must pass)
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`
4. `cargo deny check`
5. `cargo audit`
6. (Optional) benchmarks behind explicit flags; results posted as artifacts.

## 11) Performance & Footprint
- Prefer zero‑cost abstractions; avoid heap allocations in hot paths.
- Don’t add background tasks, threads, or timers without clear ownership and shutdown logic.
- Avoid gratuitous logging on hot paths; use TRACE/DEBUG judiciously with sampling where needed.

## 12) Git Hygiene & Reviews
- Conventional Commits in messages (`feat:`, `fix:`, `perf:`, `refactor:`, etc.).
- Small, focused PRs with description, risk assessment, and rollback plan.
- Include "How I tested this" with steps and sample logs.

## 13) Developer Workflow (`xtask`)
- Use **`cargo xtask`** for development automation tasks.
- Available commands:
  - `cargo xtask ci` (or `cargo ci`) — Run **all** CI checks locally before pushing
  - `cargo xtask fmt` — Check code formatting
  - `cargo xtask clippy` — Run Clippy lints
  - `cargo xtask test` — Run all workspace tests
  - `cargo xtask deny` — Check dependencies with cargo-deny
  - `cargo xtask audit` — Audit dependencies for security vulnerabilities
- The `xtask` crate lives at `xtask/` and is a standard Rust binary; not a published crate.
- **Exception:** `xtask` allows `println!`/`eprintln!` as it is a CLI tool for developer-facing output.

---

### Local Compliance Checklist (run before opening a PR)
**Quick method:**
```bash
cargo ci
```

**Manual method (equivalent):**
```bash
cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace && cargo deny check && cargo audit
```

If any rule requires an exception, add a short "Deviation" section in the PR with: *rule*, *reason*, *mitigation*, *owner*, *sunset date*.

---

## Project-Specific Notes

### Formatting
- **Nightly rustfmt** for formatting (`cargo +nightly fmt`), **stable 1.77** for compilation
- CI uses `cargo +nightly fmt --all -- --check`
- Enables better formatting: `group_imports`, `wrap_comments`, `imports_granularity`

### Package Manager
- **pnpm** for frontend (not npm)
- Config: `.npmrc` with `shamefully-hoist=true`, `ignore-scripts=false`
- Lockfile: `pnpm-lock.yaml` (tracked in git)

### Build Locations
- Frontend: builds to `frontend/dist/` (not root `dist/`)
- Tauri config: `frontendDist: "../../frontend/dist"`
- Gitignore: `/frontend/dist`, `.pnpm-store`

### Makefile
- **Use `make` for common tasks** (preferred over raw commands)
- `make help` — Show all available commands
- `make ci` — Run full CI pipeline locally
- `make check` — Quick checks (fmt, lint, test)
- `make build` — Build everything (frontend + backend)
- `make dev` — Run Tauri dev server
- `make audit` — Security audits (cargo-audit + cargo-deny)

### Platform
- **macOS-only** Tauri app (no Linux/Windows builds)
- Linux deps in `Cargo.lock` are **not compiled** for macOS targets
- Security audits ignore Linux-only advisories (`.cargo/audit.toml`)
- `xtask` crate excluded from clippy (`make lint` skips it)

### Common Module Organization (`pulsearc-common`)

The `pulsearc-common` crate provides shared utilities organized in tiers.

**📖 For comprehensive API documentation, usage examples, and migration guides, see [API_GUIDE.md](crates/common/docs/API_GUIDE.md)**

#### Module Tiers

**Foundation Tier** (feature = `foundation`):
- `error` — `CommonError` type with classification and context
- `validation` — Field validators, rule builders, validation framework
- `utils` — Macros, serde helpers
- `collections` — Specialized data structures (bloom filter, bounded queue, LRU, trie, ring buffer)

**Runtime Tier** (feature = `runtime`):
- `cache` — Thread-safe caching with TTL and eviction
- `crypto` — AES-256-GCM encryption primitives
- `privacy` — Data hashing and pattern detection
- `time` — Duration formatting, intervals, timers, cron support
- `resilience` — **Generic** circuit breaker and retry implementations
- `sync` — Domain-specific sync queue with integrated resilience
- `lifecycle` — Component lifecycle management
- `observability` — Metrics, tracing, error reporting

**Platform Tier** (feature = `platform`):
- `auth` — OAuth client, token management, PKCE
- `security` — Key management, keychain provider, RBAC
- `storage` — SQLCipher integration, encrypted storage
- `compliance` — Audit logging, feature flags

#### Key Module Relationships

**Resilience Patterns:**
- `resilience::circuit_breaker` — Generic circuit breaker (library-quality)
- `resilience::retry` — Generic retry with backoff strategies (library-quality)
- `sync::retry` — Domain-specific retry for queue operations (integrated metrics, tracing)
- Use `resilience` for new modules; use `sync::retry` within sync/queue domain

**Encryption:**
- `crypto::encryption` — Low-level AES-256-GCM primitives
- `security::encryption` — High-level key management (caching, rotation, keychain)

**Keychain:**
- `security::encryption::keychain` — Generic platform keychain provider
- `auth::keychain` — OAuth token-specific storage helpers
- `security::keychain` — Convenience re-export

#### Testing Utilities (feature = `test-utils`)
- `testing` — Mock clocks, builders, matchers, temp files, fixtures

---

**📚 Additional Resources:**
- **[Common Crate API Guide](crates/common/docs/API_GUIDE.md)** — Comprehensive documentation with 100+ examples, best practices, and troubleshooting
- **[Common Crate README](crates/common/README.md)** — Feature flags, directory structure, and quick start
- **Module READMEs** — Detailed docs in each module directory (e.g., `crates/common/src/validation/README.md`)
