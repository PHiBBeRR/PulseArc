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
- Include “How I tested this” with steps and sample logs.

---

### Local Compliance Checklist (run before opening a PR)
```bash
cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace && cargo deny check && cargo audit
```
If any rule requires an exception, add a short “Deviation” section in the PR with: *rule*, *reason*, *mitigation*, *owner*, *sunset date*.
