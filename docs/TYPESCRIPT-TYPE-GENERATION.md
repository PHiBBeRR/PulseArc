# TypeScript Type Generation Guide

## Overview

PulseArc uses [`ts-rs`](https://github.com/Aleph-Alpha/ts-rs) to automatically generate TypeScript type definitions from Rust domain types. This ensures type safety between the Rust backend and TypeScript frontend.

## Architecture

```
┌──────────────────────────────────────┐
│ Rust Domain Types                    │
│ (crates/domain/src/types/*.rs)       │
│                                      │
│ #[cfg_attr(feature = "ts-gen",       │
│   derive(TS))]                       │
│ #[cfg_attr(feature = "ts-gen",       │
│   ts(export))]                       │
│ pub struct DatabaseStats { ... }     │
└──────────┬───────────────────────────┘
           │
           │ cargo test --features ts-gen
           ▼
┌──────────────────────────────────────┐
│ Generated Bindings (Temporary)       │
│ crates/domain/bindings/*.ts          │
│                                      │
│ ❌ NOT committed to git              │
│ ✅ Added to .gitignore               │
└──────────┬───────────────────────────┘
           │
           │ cargo xtask codegen (sync)
           ▼
┌──────────────────────────────────────┐
│ Frontend Types (Source of Truth)     │
│ frontend/shared/types/generated/*.ts │
│                                      │
│ ✅ Committed to git                  │
│ ✅ Imported by frontend code         │
└──────────────────────────────────────┘
```

## Quick Reference

### Generate Types

```bash
# Recommended (via xtask)
cargo xtask codegen

# Alternative methods
make codegen
pnpm run codegen
```

### Verify Types Are Up-to-Date

```bash
make codegen-check
```

### What Gets Generated

- **Source:** Domain types in `crates/domain/src/types/`
- **Temporary Output:** `crates/domain/bindings/` (45 files, gitignored)
- **Final Location:** `frontend/shared/types/generated/` (synced + committed)
- **Index File:** Auto-generated `index.ts` with all exports

## Developer Workflow

### 1. Adding a New Rust Type

1. **Create the type** in `crates/domain/src/types/`:

```rust
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct MyNewType {
    /// Use ts(type = "number") for i64/i32 timestamps
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,

    pub name: String,

    /// Optional fields should use ts(optional)
    #[cfg_attr(feature = "ts-gen", ts(optional))]
    pub description: Option<String>,
}
```

2. **Generate TypeScript types:**

```bash
cargo xtask codegen
```

3. **Verify the output:**

```bash
# Check that the file was created
ls frontend/shared/types/generated/MyNewType.ts

# Verify it's in the index
grep "MyNewType" frontend/shared/types/generated/index.ts
```

4. **Commit the generated files:**

```bash
git add frontend/shared/types/generated/MyNewType.ts
git add frontend/shared/types/generated/index.ts
git commit -m "feat(types): Add MyNewType TypeScript definition"
```

### 2. Modifying an Existing Type

1. **Update the Rust type** in `crates/domain/src/types/`
2. **Regenerate types:** `cargo xtask codegen`
3. **Review changes:**

```bash
git diff frontend/shared/types/generated/
```

4. **Test frontend:** Check if any frontend code breaks due to type changes
5. **Commit changes:**

```bash
git add frontend/shared/types/generated/
git commit -m "feat(types): Update MyType to include new field"
```

### 3. Before Committing Code

Always run codegen before committing if you've modified domain types:

```bash
# Quick check
make codegen-check

# Or as part of full CI
make ci
```

## Frontend Usage

### Importing Types

```typescript
// ✅ Recommended: Import from the index
import { DatabaseStats, ActivitySnapshot, UserProfile } from '@/shared/types/generated';

// ❌ Avoid: Direct imports
import { DatabaseStats } from '@/shared/types/generated/DatabaseStats';
```

### Using Generated Types

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { DatabaseStats } from '@/shared/types/generated';

async function fetchStats(): Promise<DatabaseStats> {
  // Type-safe Tauri command invocation
  const stats = await invoke<DatabaseStats>('get_database_stats');

  // TypeScript knows all fields exist
  console.log(`Snapshots: ${stats.snapshot_count}`);
  console.log(`Segments: ${stats.segment_count}`);

  return stats;
}
```

## Type Annotations Reference

### Common Annotations

| Rust Type | TypeScript Type | Annotation Needed |
|-----------|-----------------|-------------------|
| `String` | `string` | None |
| `i32`, `i64`, `u32`, `u64` | `number` | `#[ts(type = "number")]` for large ints |
| `f32`, `f64` | `number` | None |
| `bool` | `boolean` | None |
| `Option<T>` | `T \| null` | `#[ts(optional)]` for optional fields |
| `Vec<T>` | `Array<T>` | None |
| `HashMap<K, V>` | `{ [key: K]: V }` | None |
| `chrono::DateTime` | `string` (ISO 8601) | `#[ts(type = "number")]` if Unix timestamp |
| `uuid::Uuid` | `string` | None |

### Example: Complex Type

```rust
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ComplexType {
    /// Standard string field
    pub id: String,

    /// Unix timestamp (seconds) → TypeScript number
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,

    /// Optional field
    #[cfg_attr(feature = "ts-gen", ts(optional))]
    pub description: Option<String>,

    /// Nested type (must also derive TS)
    pub metadata: Metadata,

    /// Array of nested types
    pub tags: Vec<Tag>,

    /// Enum (must also derive TS)
    pub status: Status,
}
```

## CI Integration

### Automated Checks

The CI pipeline includes a `typescript-types` job that:

1. Runs `cargo xtask codegen`
2. Compares generated files with committed versions
3. **Fails the build** if types are out of sync
4. Displays a diff of what changed

### When CI Fails

If you see this error:

```
❌ TypeScript types are out of date!
Generated types differ from committed versions.
```

**Fix:**

```bash
# Regenerate types locally
cargo xtask codegen

# Commit the changes
git add frontend/shared/types/generated/
git commit -m "chore: Regenerate TypeScript types"
git push
```

## Troubleshooting

### Problem: Types not generating

**Symptoms:**
- `cargo xtask codegen` succeeds but no files appear
- `crates/domain/bindings/` directory is empty

**Solutions:**

1. **Check that types have `#[ts(export)]`:**

```rust
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]  // ← Must have this!
pub struct MyType { ... }
```

2. **Verify the test runs:**

```bash
cargo test -p pulsearc-domain --features ts-gen --lib
```

3. **Check for compilation errors:**

```bash
cargo check -p pulsearc-domain --features ts-gen
```

---

### Problem: CI fails with "types out of date"

**Symptoms:**
- Local build works fine
- CI fails on `typescript-types` job
- Error shows diff of changed files

**Solutions:**

1. **You forgot to run codegen before committing:**

```bash
cargo xtask codegen
git add frontend/shared/types/generated/
git commit --amend --no-edit
git push --force-with-lease
```

2. **Check git status of generated files:**

```bash
git status frontend/shared/types/generated/
```

3. **Ensure bindings directory is gitignored:**

```bash
git check-ignore crates/domain/bindings/
# Should output: crates/domain/bindings/
```

---

### Problem: Type mismatch errors in frontend

**Symptoms:**
- TypeScript compilation errors
- Fields missing or renamed
- Incompatible types

**Solutions:**

1. **Regenerate types:**

```bash
cargo xtask codegen
```

2. **Check for breaking changes in Rust:**

```bash
git diff crates/domain/src/types/
```

3. **Update frontend code to match new types:**

```typescript
// Before
const stats: DatabaseStats = await invoke('get_stats');
console.log(stats.old_field);  // ← Field was renamed

// After
const stats: DatabaseStats = await invoke('get_stats');
console.log(stats.new_field);  // ← Updated to new field name
```

---

### Problem: "Bindings directory not found" error

**Symptoms:**
```
Error: Bindings directory not found at crates/domain/bindings/.
TypeScript generation may have failed.
```

**Solutions:**

1. **Tests failed during generation:**

```bash
# Run tests manually to see the actual error
cargo test -p pulsearc-domain --features ts-gen --lib
```

2. **Dependencies not installed:**

```bash
cargo fetch
```

3. **Clean and rebuild:**

```bash
cargo clean
cargo xtask codegen
```

---

### Problem: Extra/stale files in frontend/shared/types/generated

**Symptoms:**
- Files in `frontend/shared/types/generated/` that aren't in `crates/domain/bindings/`
- 70 exports in `index.ts` but only 45 types generated

**Explanation:**

The `codegen` command does NOT delete files. This is intentional to avoid accidentally removing manually-maintained types.

**Solutions:**

1. **Identify stale files:**

```bash
comm -13 \
  <(ls crates/domain/bindings/ | sort) \
  <(ls frontend/shared/types/generated/*.ts | xargs -n1 basename | sort)
```

2. **Manually remove if confirmed stale:**

```bash
# CAUTION: Make sure these aren't needed!
rm frontend/shared/types/generated/OldType.ts
cargo xtask codegen  # Regenerate index.ts
```

3. **Or keep them if they're frontend-specific types**

---

## Advanced Topics

### Adding Type Overrides

Sometimes you need to customize how Rust types map to TypeScript:

```rust
// Force a specific TypeScript type
#[cfg_attr(feature = "ts-gen", ts(type = "number"))]
pub timestamp: i64,  // → number instead of bigint

// Rename a field in TypeScript
#[cfg_attr(feature = "ts-gen", ts(rename = "userId"))]
pub user_id: String,  // → userId in TypeScript

// Skip a field entirely
#[cfg_attr(feature = "ts-gen", ts(skip))]
pub internal_field: String,  // Not exported to TypeScript
```

### Inline Types

For types that don't need to be exported as standalone types:

```rust
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export, export_to = "frontend/shared/types/generated/"))]
pub struct ExportedType {
    // This type is inlined, not exported separately
    #[cfg_attr(feature = "ts-gen", derive(TS))]
    pub inline_field: InlineType,
}

// No #[ts(export)] on this type
#[cfg_attr(feature = "ts-gen", derive(TS))]
struct InlineType {
    pub value: String,
}
```

### Pre-commit Hook (Optional)

Add this to `.git/hooks/pre-commit` to automatically check types:

```bash
#!/bin/bash

echo "Checking TypeScript types..."
if ! make codegen-check; then
  echo ""
  echo "❌ TypeScript types are out of date!"
  echo "   Run 'make codegen' to fix."
  echo ""
  exit 1
fi

echo "✓ TypeScript types are up-to-date"
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

## Summary

| Task | Command | When |
|------|---------|------|
| Generate types | `cargo xtask codegen` | After modifying domain types |
| Check if up-to-date | `make codegen-check` | Before committing |
| Run full CI locally | `make ci` | Before opening PR |
| View generated types | `ls frontend/shared/types/generated/` | When debugging |

## See Also

- [ts-rs Documentation](https://github.com/Aleph-Alpha/ts-rs)
- [Domain Crate README](../crates/domain/README.md)
- [Phase 4 Migration Docs](./PHASE-4-NEW-CRATE-MIGRATION.md)
