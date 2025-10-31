## Summary

- [ ] Ready for review
- [ ] Linked the relevant issue(s)
- [ ] Added screenshots or logs if the UI changes

## Testing

- [ ] `cargo test --workspace --all-features`
- [ ] `pnpm test`
- [ ] Additional validation (describe below)

## Phase 3 Feature Flags

- [ ] Ran `cargo xtask test-features` (or confirmed the CI feature matrix covers these changes)
- [ ] Verified new/updated code is correctly gated with `#[cfg(feature = "...")]`
- [ ] Added or updated regression tests for feature-flagged functionality
- [ ] Double-checked for the anti-patterns listed in `docs/issues/PHASE-3-PRE-MIGRATION-FIXES.md`
