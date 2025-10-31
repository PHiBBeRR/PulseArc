# Repository Guidelines

## Project Structure & Module Organization
- Rust code is organized as a Cargo workspace under `crates/`, split into `common`, `domain`, `core`, `infra`, and the Tauri-facing `api`. Each crate keeps its tests alongside the `src` tree.
- Frontend code lives in `frontend/` with feature-centric folders (`features/`, `shared/`, `components/`) and the Tauri entry point in `main.tsx`.
- Automation utilities live in `xtask/`, documentation in `docs/`, and archived experiments in `legacy/` (read-only).

## Build, Test, and Development Commands
- Use `make dev` for the full-stack Tauri experience, or `pnpm dev` when iterating purely on the web UI.
- `make build` compiles the Rust workspace and frontend; `make build-tauri` produces the desktop bundle.
- `make test` runs Rust tests; add `make test-frontend` for Vitest suites. Prefer `make ci` before merging to mirror the pipeline.

## Coding Style & Naming Conventions
- Format Rust with `cargo +nightly fmt --all` and keep Clippy clean with `cargo clippy --workspace --all-targets --all-features`. Unsafe code is denied and unwrap/expect calls should stay in tests only.
- Follow idiomatic casing: `snake_case` for Rust items, `PascalCase` for React components, and `camelCase` for hooks/utilities. Avoid `any` in TypeScript; ESLint is configured to flag it.
- Run `pnpm lint` for TypeScript/React checks. Prettier is available via `pnpm format`—use it before committing UI changes.

## Testing Guidelines
- Execute `cargo test --workspace --all-features` for the Rust side. Use `cargo test -p crate_name` while iterating.
- Frontend tests use Vitest and Testing Library. Run `pnpm test` for the suite, `pnpm test --watch` for TDD, and add `.integration.test.ts` for cross-service scenarios.
- Run `pnpm prisma generate` after schema changes to refresh generated types.

## Commit & Pull Request Guidelines
- Follow the Conventional Commits pattern seen in `git log` (e.g., `feat:`, `fix(ci):`, `docs:`). Keep scopes concise and use the imperative mood.
- Before opening a PR, ensure `make ci` passes, link the relevant issue, and include screenshots or terminal output for UI-affecting changes.
- Small, reviewable PRs are preferred. Call out schema or migration impacts explicitly and document new env vars in the PR body.

## Security & Configuration Tips
- Sensitive configuration lives in local `.env` files; never commit credentials. Update onboarding docs in `docs/` when variables change.
- Run `make audit` periodically to execute `cargo audit` and `cargo deny`, and keep dependencies current with `make update` or targeted `pnpm update`.
