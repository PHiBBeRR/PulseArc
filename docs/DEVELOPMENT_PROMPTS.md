# PulseArc Development Prompt Toolkit

## Using This Toolkit
- Start by priming your AI assistant with the starter context prompt before requesting help on a task.
- Combine the base context with one or more task-specific prompts below to anchor responses in the correct layer.
- Replace placeholders like `{describe the outcome you want}` with concrete goals, inputs, or constraints.
- Ask for tests, verification commands, and architectural guardrails explicitly so responses stay production-ready.

## Repository Context Snapshot
- Tauri 2.x macOS desktop app with a Rust workspace under `crates/` (`common`, `domain`, `core`, `infra`, `api`) implementing a hexagonal architecture.
- React/TypeScript frontend in `frontend/` with feature-first folders (`features/`, `components/`, `shared/`) and the entry point at `frontend/main.tsx`.
- Automation and tooling live in `xtask/`, documentation in `docs/`, and archived experiments in `legacy/` (read-only).
- Key workflows: `make dev`, `make build`, `make build-tauri`, `make test`, `pnpm dev`, `pnpm lint`, `pnpm test`, and `cargo test --workspace --all-features`.

## Starter System Context Prompt
```text
You are an engineering copilot for the PulseArc macOS desktop app built with Tauri 2.x.
System context:
- Rust workspace lives under `crates/` (`common`, `domain`, `core`, `infra`, `api`) with layered hexagonal boundaries.
- Frontend lives in `frontend/` with feature-first modules, shared primitives, and Tauri IPC bridges.
- Automation lives in `xtask/`, docs in `docs/`, and experiments in `legacy/` (read-only).
- Guardrails: deny `unsafe`, avoid `unwrap`/`expect` outside tests, keep modules SRP-aligned, prefer `cargo +nightly fmt --all`, `cargo clippy --workspace --all-targets --all-features`, and `pnpm lint`.
Task: {describe the outcome you want}
Deliverables: {code, tests, docs, plans, etc.}
Expectations:
1. Summarize how the change fits the existing architecture.
2. Provide scoped diffs or code suggestions referencing exact file paths.
3. Enumerate validation steps (e.g., `make test`, `pnpm test`, targeted crate tests) and note any follow-up tasks.
```

## Rust Backend Prompts

### Cross-Crate Feature Slice
```text
Role: PulseArc backend guide for a feature touching multiple crates.
Scenario: {describe cross-cutting feature}
Touchpoints:
- Domain invariants in `crates/domain`.
- Use cases and ports in `crates/core`.
- Adapters in `crates/infra`.
- Tauri command exposure in `crates/api`.
Requirements:
- Maintain flow `common → domain → core → infra → api`; never invert dependencies.
- Keep `core` free of direct I/O; delegate to traits.
- Propagate errors with `CommonResult` or typed error enums; avoid `unwrap`.
Provide:
1. Architecture-aligned plan referencing concrete files.
2. Code or diff snippets per crate observing layering contracts.
3. Testing matrix (unit, integration, command tests) plus commands to run.
```

### Shared Utilities (`crates/common`)
```text
Focus: Extend shared primitives in `crates/common`.
Objective: {describe utility or infrastructure enhancement}
Guidelines:
- Keep logic reusable and side-effect free; side effects belong in `infra`.
- Update module docs and re-export lists when adding modules or public types.
- Prefer resilience helpers (`retry`, `CircuitBreaker`, etc.) when handling transient errors.
- Provide consistent error mapping via `CommonError`/`CommonResult`.
Output:
1. Outline of consumer impacts across other crates.
2. Proposed code changes under `crates/common/src/...` with justification.
3. Unit tests (using `crates/common::testing`) and commands to validate.
```

### Domain Modelling (`crates/domain`)
```text
Focus: Business entities and invariants in `crates/domain`.
Goal: {describe domain change}
Constraints:
- Express domain errors with `thiserror` enums; no logging or I/O.
- Prefer smart constructors and validation helpers over exposing unchecked fields.
- Align vocabulary with time tracking, classification, sync, and ML concepts already present.
Deliver:
1. Updated types/services with doc comments highlighting rules.
2. Companion tests under `crates/domain/src/...`.
3. Notes on downstream impacts for consumers in `crates/core`.
```

### Core Use Cases (`crates/core`)
```text
Focus: Application services and ports in `crates/core`.
Task: {describe use case or service}
Guidelines:
- Depend on workspace domain crates (`pulsearc-common`, `pulsearc-domain`) and vetted third-party libraries needed for pure application logic; never couple `core` to `infra`, `api`, or frontend code.
- Model side effects via traits; implementations land in `crates/infra`.
- Inject dependencies via constructors; keep services testable.
Provide:
1. Proposed trait/service signatures and rationale.
2. Implementation plan referencing `crates/core/src/{module}`.
3. Test strategy using trait mocks and `crates/common::testing`.
```

### Infrastructure Implementations (`crates/infra`)
```text
Focus: Implement adapters in `crates/infra`.
Scenario: {describe integration or platform task}
Constraints:
- Implement ports defined in `crates/core`; keep macOS-specific code in `platform/`.
- Use observability utilities for metrics/logging; respect privacy redaction helpers.
- Handle databases via SQLCipher pools and HTTP integrations via resilient clients.
Deliverables:
1. Adapter implementation outline with file paths (e.g., `crates/infra/src/platform/...`).
2. Error handling strategy using `CommonError` or specific adapter errors.
3. Integration or async tests, plus manual verification steps if needed.
```

### Tauri API Layer (`crates/api`)
```text
Focus: Expose functionality through Tauri commands in `crates/api`.
Objective: {describe UI-facing capability}
Guidelines:
- Wire dependencies via `AppContext` in `crates/api/src/context`.
- Register new commands in the appropriate module and `lib.rs`.
- Serialize payloads with `serde` for TypeScript compatibility; note enums/structs for the frontend.
- Emit events judiciously; document the payload contract.
Provide:
1. Updated command definitions and context wiring steps.
2. Frontend invoke signature or listener expectations.
3. Tests to run (`cargo test -p pulsearc-api`, targeted integration tests) plus manual QA checklist.
```

## Frontend & Tauri Bridge Prompts

### Feature UI Work (`frontend/features`)
```text
Focus: Implement or refine a feature module under `frontend/features`.
Goal: {describe UI change}
Context:
- React + TypeScript with Vite entry in `frontend/main.tsx`.
- State lives in feature-local hooks or shared stores in `frontend/shared`.
- Backend data arrives via typed Tauri invoke wrappers or `@tauri-apps/api`.
Guidelines:
- Keep presentational components reusable; move shared pieces into `frontend/components`.
- Maintain strong typing; avoid `any`, prefer existing DTOs or Zod schemas.
- Update styles in `globals.css` or feature-scoped styles.
Deliver:
1. Component tree/state diagram and affected paths.
2. Implementation notes with file-level references.
3. Testing plan with Vitest + Testing Library and manual checks (e.g., `pnpm dev` smoke test).
```

### Shared Hooks & Utilities (`frontend/shared`)
```text
Focus: Create or update shared hooks/utilities in `frontend/shared`.
Task: {describe hook or helper}
Requirements:
- Encapsulate IPC, storage, or caching concerns here; keep components declarative.
- Provide strict TypeScript signatures and exhaustive error handling.
- Update `frontend/shared/index.ts` exports and add documentation comments.
Produce:
1. API sketch with types and usage example.
2. Implementation plan referencing exact files.
3. Unit tests or usage samples plus lint/format commands (`pnpm lint`, `pnpm format` if needed).
```

### Tauri Command Bridge (Rust ↔︎ TS)
```text
Scenario: Wire a new Tauri command end-to-end.
Inputs:
- Backend command lives in `crates/api`.
- Frontend caller under `frontend/shared/services` (for invoke and IPC helpers) or `frontend/features/{module}`.
Steps:
1. Define or adjust the Rust command signature, serialization, and event emission.
2. Generate or update TypeScript invoke wrappers, ensuring payload parity.
3. Map `CommonError` variants to user-facing responses (toast, modal, retry).
4. Add tests on both sides (Rust command tests, Vitest mocks/integration).
Outputs:
1. Diff outline spanning Rust and TypeScript files.
2. Verification checklist (`cargo test -p pulsearc-api`, `pnpm test`, manual UI checks via `make dev`).
3. Rollback/feature flag notes if applicable.
```

### Reusable Components (`frontend/components`)
```text
Focus: Build or adjust reusable components in `frontend/components`.
Objective: {describe component change}
Constraints:
- Keep components presentational; accept props and delegate business logic to callers.
- Ensure accessibility (ARIA attributes, keyboard navigation, focus management).
- Add Storybook entries or usage docs if the design system relies on them.
Deliver:
1. Prop interface definition and usage guidance.
2. Implementation steps with path references.
3. Snapshot/interaction tests and visual regression considerations.
```

## Testing & QA Prompts

### Rust Tests
```text
Goal: Design test coverage for a Rust workspace change.
Target crate: {crate name}
Guidelines:
- Co-locate unit tests with modules (`mod tests`); integration tests go under `tests/`.
- Use helpers from `crates/common::testing` to mock time, retries, or async components.
- Avoid invoking real macOS APIs; mock traits defined in `crates/core`.
Deliverables:
1. Test scenarios and expected outcomes.
2. Representative test snippets or diff guidance.
3. Commands to run (`cargo test -p {crate name}`, optionally `make test`).
```

### Frontend Tests
```text
Goal: Add or update Vitest/Testing Library tests.
Scope: {component or hook}
Guidelines:
- Co-locate tests (`.test.ts`/`.test.tsx`) next to the implementation.
- Mock Tauri IPC via existing test utilities or manual `vi.fn()` wrappers.
- Cover success, failure, loading, and edge cases that matter for productivity analytics.
Provide:
1. Test plan with file references (e.g., `frontend/features/.../__tests__/`).
2. Representative assertions or test utilities to reuse.
3. Commands to execute (`pnpm test -- {pattern}`) and expected results.
```

### Integration Scenario Validation
```text
Objective: Validate a cross-layer scenario end-to-end.
Scenario: {describe flow}
Components involved: {list crates/modules}
Checklist:
1. Outline environment setup (env vars, `make dev`, seed data).
2. Define logging/metrics to enable via `crates/common::observability`.
3. Provide manual QA steps and automated checks if available.
Outputs:
1. Flow diagram or textual sequence referencing concrete modules.
2. Data fixture or mock plan.
3. Pass/fail criteria and rollback steps.
```

## Build, Tooling & Diagnostics Prompts

### CI Readiness
```text
Goal: Prepare a change for CI.
Context: PulseArc relies on `make ci` (fmt, clippy, tests, lint).
Request:
1. Enumerate exact commands to run locally (Rust + frontend).
2. Note runtime, caching, and environment expectations (e.g., `.env` values).
3. Provide troubleshooting tips for common failures.
Outputs:
1. Ordered command list with rationale.
2. Expected outputs or artifacts to collect.
3. Follow-up actions if any step fails.
```

### Performance Profiling
```text
Goal: Investigate performance characteristics.
Target: {service/hook}
Guidelines:
- For Rust, describe using `cargo bench`, sampling profilers, or tracing instrumentation.
- For frontend, leverage React Profiler, Lighthouse, or Vitest benchmarks.
- Use observability primitives in `crates/common::observability` for timing metrics.
Deliverables:
1. Profiling approach with commands/tools.
2. Instrumentation or code changes required.
3. Interpretation checklist and remediation ideas.
```

### Bug Reproduction & Diagnostics
```text
Objective: Build a reproducible bug investigation plan.
Context: {bug summary}
Checklist:
1. Identify affected crates/modules and user flows.
2. Outline logging/metrics toggles and redaction requirements.
3. Provide minimal reproduction steps, including commands and sample data.
Outputs:
1. Hypothesis list with supporting evidence to gather.
2. Files or code paths to inspect first.
3. Verification steps post-fix (tests + manual validation).
```

## Documentation & Knowledge Sharing Prompts

### Architecture Decision Record (`docs/adr/`)
```text
Goal: Draft an ADR capturing a significant decision.
Topic: {decision}
Expectations:
- Follow existing ADR template (status, context, decision, consequences).
- Reference impacted crates/modules and external integrations.
- Note testing, security, and deployment implications.
Deliverables:
1. Section-by-section outline with bullet points.
2. Suggested filename (`docs/adr/{yyyy-mm-dd}-{slug}.md`).
3. Follow-up tasks or monitoring needs.
```

### Onboarding & Documentation Updates
```text
Objective: Refresh developer docs (e.g., `docs/FILE_MAPPING.md`, `docs/MACOS_ARCHITECTURE.md`).
Scenario: {summarize change}
Guidelines:
- Capture new commands, env vars, or workflows; align with macOS/Tauri specifics.
- Cross-link to ADRs or source files for deeper context.
- Provide reproducible steps that a new developer can follow.
Outputs:
1. Proposed doc updates with headings and bullet points.
2. Exact text to insert or replace.
3. Validation steps to ensure instructions succeed (commands + expected results).
```

## Security, Privacy & Compliance Prompts

### Security & Privacy Review
```text
Goal: Perform a security/privacy assessment for a change.
Scope: {feature}
Considerations:
- Data handling via `crates/common::privacy` and encryption in SQLCipher.
- macOS permissions (Accessibility, idle detection) and keychain usage.
- External integrations (SAP, Calendar) and OAuth scopes.
Deliver:
1. Threat model outline with mitigations.
2. Checklist of security tests or audits (`make audit`, `cargo deny`).
3. Recommendations for logging, redaction, or policy updates.
```

### Compliance & Observability Alignment
```text
Objective: Ensure compliance and monitoring coverage.
Change: {describe}
Guidelines:
- Update audit logging via `crates/common::compliance`.
- Add metrics/traces using `crates/common::observability`.
- Document retention or data residency considerations.
Outputs:
1. Compliance checklist with owners.
2. Instrumentation plan and dashboards to review.
3. Post-deployment verification steps.
```

## Release & Deployment Prompts

### Release Checklist
```text
Goal: Prepare a release build for the macOS app.
Context:
- Desktop bundle generated via `make build-tauri`.
- Versioning managed through workspace `Cargo.toml` and `package.json`.
Deliverables:
1. Step-by-step checklist (version bump, changelog, build, notarization if needed).
2. Tests to rerun post-build (targeted `cargo test`, `pnpm test`, smoke run via `make dev`).
3. Announcement or changelog outline referencing new capabilities.
```
