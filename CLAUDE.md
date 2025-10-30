## General Rules
- **NEVER** commit or push with errors or failing tests
- **NEVER** bypass the pre-commit hook
- **NEVER** make up facts or make assumptions
- **NEVER** indicate completion if you did not meet all acceptance requirements
- **NEVER** create documentation unless specifically requested
- **NEVER** take short cuts - always ensure code follows best practices
- Always update tickets following implementation of a step or phase
- Always ask for confirmation if you are unsure or have conflicting decisions

## Testing
- Put unit tests in the same modu#le file under src/ using #[cfg(test)] mod tests { ... }.
- Unit tests may exercise private items; keep them small and focused on the module's behavior.
- Put integration tests in the top-level tests/ directory; each *.rs there is a separate crate.
- Integration tests should import your crate like a user would and test public API only.
- Share helpers for integration tests in tests/common/mod.rs (and tests/data/ for fixtures).
- Keep file/fixture paths stable by resolving from CARGO_MANIFEST_DIR.
- Place benchmarks in benches/ (e.g., with Criterion) — separate from tests.
- Put runnable examples in examples/; they double as documentation.
- Write doc tests in /// or //! comments; they run with cargo test.
- For binaries, keep src/main.rs thin; put real logic in src/lib.rs so tests can use it.
- For multiple binaries, use src/bin/*.rs; still test via the library crate.
- Use [dev-dependencies] for test-only crates and gate test modules with #[cfg(test)].


## Enforcement Levels

### 🚫 DENY - Blocks CI/Builds
- `unimplemented!()` - No incomplete code in production
- `unwrap_in_result()` - Never unwrap inside Result functions
- `panic_in_result_fn` - Results must propagate errors
- `correctness` - Bug detection (always fails build)
- `unused_must_use` - Never ignore Result/Option returns

### ⚠️ WARN - Fix Before Merge
**Error Handling:**
- `.unwrap()`, `.expect()` → Use `?` operator
- `panic!()` → Return `Result<T, E>`
- `todo!()` → Track and complete
- `arr[idx]` → Use `.get(idx)?` for safety
- `.get().unwrap()` → Defeats the purpose of `.get()`

**Code Quality:**
- `dbg!()` → Remove debug code
- `println!()` / `eprintln!()` → Use `tracing`
- `exit()` / `abort()` → Graceful shutdown
- Complex functions → Refactor (complexity > 15)
- Too many params → Use config structs (> 5 params)
- Large functions → Break down (> 100 lines)

**Performance:**
- Large stack arrays → Box them (> 500KB)
- Large Vec types → Box variants (> 4KB)
- Clone on Arc/Rc → Cheap, avoid `&Arc`

**Style:**
- Wildcard imports → Be explicit
- String concatenation → Use `format!()` or `.push_str()`

### ✅ ALLOWED - Development Flexibility
- Private item docs (encourage but don't require)
- Similar variable names (x1, x2 acceptable in context)
- Some false-positive pedantic lints

## Complexity Thresholds

| Metric | Limit | Rationale |
|--------|-------|-----------|
| Cognitive complexity | 15 | Maintainable functions |
| Type complexity | 100 | Clean API design |
| Function parameters | 5 | Use structs beyond this |
| Boolean parameters | 3 | Use config structs |
| Function lines | 100 | Focused, testable code |

## Disallowed Patterns

```rust
// ❌ Don't use - prevents graceful shutdown
std::process::exit()
std::process::abort()

// ⚠️ Allowed in dev, but be careful - not thread-safe
std::env::set_var()    // Use sparingly, consider config management for production
std::env::remove_var()
```