# SqlCipherPool API Reference - Critical Differences

**For Phase 0 Segmenter Refactoring**

---

## 🚨 CRITICAL: Use SqlCipherPool, NOT LocalDatabase

As part of the ADR-003 migration, **`LocalDatabase` is being deprecated**. All new repository implementations MUST use `SqlCipherPool` from `pulsearc_common::storage::sqlcipher`.

**Correct Import**:
```rust
use pulsearc_common::storage::sqlcipher::{SqlCipherPool, SqlCipherConnection, StorageResult};
use rusqlite::ToSql;
```

---

## Key API Differences

### 1. SqlCipherPool is Synchronous (No async/await)

**Location**: `crates/common/src/storage/sqlcipher/pool.rs:174`

```rust
impl SqlCipherPool {
    // ✅ SYNCHRONOUS - No async, no .await
    pub fn get_sqlcipher_connection(&self) -> StorageResult<SqlCipherConnection> {
        // Returns immediately with pooled connection
    }
}
```

**Key Point**: Unlike many modern database pools, `SqlCipherPool::get_sqlcipher_connection()` is **synchronous**. Do NOT use `async` or `.await`.

### 2. query_map Returns Vec, Not Iterator

**Location**: `crates/common/src/storage/sqlcipher/connection.rs:116`

```rust
impl<'conn> SqlCipherStatement<'conn> {
    pub fn query_map<T, F>(&mut self, params: &[&dyn ToSql], mut f: F) -> StorageResult<Vec<T>>
    where
        F: FnMut(&Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let rows = self.inner.query_map(params, |row| f(row)).map_err(StorageError::from)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)  // Already collected
    }
}
```

**Key Point**: Unlike standard `rusqlite::Statement::query_map` which returns `Rows<'_>` (an iterator), `SqlCipherStatement::query_map` **immediately collects** the results and returns `StorageResult<Vec<T>>`.

### 3. Parameters Use &[&dyn ToSql]

**Location**: `crates/common/src/storage/sqlcipher/connection.rs:116`

Parameters must be `&[&dyn ToSql]`, not owned values or `&[String]`.

---

## ❌ Common Mistakes

### Mistake 1: Using async/await

```rust
// ❌ WRONG - Compilation error!
impl SegmentRepository for SqlCipherSegmentRepository {
    async fn find_segments(&self) -> Result<Vec<Segment>> {
        let conn = self.pool.get_sqlcipher_connection().await?;  // ❌ ERROR: no .await method
        // ...
    }
}

// Error message:
// error[E0599]: no method named `await` found for struct `StorageResult<SqlCipherConnection>`
```

### Mistake 2: Calling `.collect()` on query_map Result

```rust
// ❌ WRONG - Compilation error!
let segments = stmt
    .query_map(&[&date_str as &dyn ToSql], |row| {
        Ok(ActivitySegment {
            id: row.get(0)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;  // ❌ ERROR: Vec<T> doesn't implement IntoIterator

// Error message:
// error[E0277]: `Vec<ActivitySegment>` is not an iterator
//     |
//     |     .collect::<Result<Vec<_>, _>>()?;
//     |      ^^^^^^^ `Vec<ActivitySegment>` is not an iterator
```

### Mistake 3: Passing Owned Values to query_map

```rust
// ❌ WRONG - Type error!
let date_str = date.to_string();
let segments = stmt
    .query_map(&[date.to_string()], |row| {  // ❌ ERROR: expected &[&dyn ToSql]
        Ok(ActivitySegment { ... })
    })?;

// Error message:
// error[E0308]: mismatched types
//    expected reference `&[&dyn ToSql]`
//    found reference `&[String]`
```

---

## ✅ Correct Usage Patterns

### Pattern 1: Direct Usage (Simplest)

```rust
use std::sync::Arc;
use pulsearc_common::storage::sqlcipher::{SqlCipherPool, StorageResult};
use rusqlite::ToSql;

pub struct SqlCipherSegmentRepository {
    pool: Arc<SqlCipherPool>,
}

impl SegmentRepository for SqlCipherSegmentRepository {
    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>> {
        // ✅ CORRECT: Synchronous (no async, no .await)
        let conn = self.pool.get_sqlcipher_connection()
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let sql = "SELECT id, start_time, end_time FROM segments WHERE date = ?1";
        let mut stmt = conn.prepare(sql)
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let date_str = date.to_string();
        // ✅ CORRECT: query_map returns Vec<T> directly, no .collect()
        // ✅ CORRECT: Use &date_str as &dyn ToSql
        let segments = stmt.query_map(&[&date_str as &dyn ToSql], |row| {
            Ok(ActivitySegment {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
            })
        }).map_err(|e| CommonError::storage(e.to_string()))?;  // Returns Vec

        Ok(segments)
    }
}
```

### Pattern 2: With Multiple Parameters

```rust
impl SegmentRepository for SqlCipherSegmentRepository {
    fn find_by_date_and_user(&self, date: NaiveDate, user_id: i64) -> CommonResult<Vec<ActivitySegment>> {
        let conn = self.pool.get_sqlcipher_connection()
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let sql = "SELECT * FROM segments WHERE date = ?1 AND user_id = ?2";
        let mut stmt = conn.prepare(sql)
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let date_str = date.to_string();
        // ✅ CORRECT: Multiple parameters as &[&dyn ToSql]
        let segments = stmt
            .query_map(
                &[&date_str as &dyn ToSql, &user_id as &dyn ToSql],
                |row| {
                    Ok(ActivitySegment {
                        id: row.get(0)?,
                        start_time: row.get(1)?,
                        end_time: row.get(2)?,
                    })
                }
            )
            .map_err(|e| CommonError::storage(e.to_string()))?;

        Ok(segments)
    }
}
```

### Pattern 3: With Post-Processing

```rust
impl SegmentRepository for SqlCipherSegmentRepository {
    fn find_and_filter(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>> {
        let conn = self.pool.get_sqlcipher_connection()
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let mut stmt = conn.prepare("SELECT * FROM segments WHERE date = ?1")
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let date_str = date.to_string();
        // ✅ CORRECT: Get Vec first, then filter
        let all_segments = stmt.query_map(&[&date_str as &dyn ToSql], |row| {
            Ok(ActivitySegment {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
            })
        }).map_err(|e| CommonError::storage(e.to_string()))?;

        // Now you can use normal Vec methods
        let filtered = all_segments
            .into_iter()
            .filter(|seg| seg.duration() > Duration::minutes(5))
            .collect();

        Ok(filtered)
    }
}
```

### Pattern 4: Using rusqlite::params! Macro (Alternative)

```rust
use rusqlite::params;

impl SegmentRepository for SqlCipherSegmentRepository {
    fn find_by_id(&self, id: &str) -> CommonResult<Option<ActivitySegment>> {
        let conn = self.pool.get_sqlcipher_connection()
            .map_err(|e| CommonError::storage(e.to_string()))?;

        let sql = "SELECT * FROM segments WHERE id = ?1";

        // ✅ ALTERNATIVE: Use params! macro for cleaner syntax
        conn.query_row(sql, params![id], |row| {
            Ok(ActivitySegment {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
            })
        })
        .optional()
        .map_err(|e| CommonError::storage(e.to_string()))
    }
}
```

---

## Comparison Table

| Operation | rusqlite::Statement | SqlCipherStatement |
|-----------|---------------------|-------------------|
| `query_map(...)` return type | `Rows<'_>` (iterator) | `StorageResult<Vec<T>>` (collected) |
| Need to call `.collect()`? | ✅ Yes | ❌ No (already collected) |
| Returns `Result`? | ✅ Yes (`Result<Rows<'_>, Error>`) | ✅ Yes (`StorageResult<Vec<T>>`) |
| Lazy evaluation? | ✅ Yes (iterator) | ❌ No (eager) |
| Can use `.filter()` on result? | ✅ Yes (is iterator) | ❌ No (is Vec, use `.into_iter()` first) |
| Pool API | N/A (direct connection) | **Synchronous** (`get_sqlcipher_connection()`) |
| Use async/await? | N/A | ❌ **NO** (synchronous) |

---

## Migration Checklist

When refactoring code from `LocalDatabase` to `SqlCipherPool`:

- [ ] Replace `LocalDatabase` with `SqlCipherPool`
- [ ] Change repository struct to hold `Arc<SqlCipherPool>`
- [ ] Import from `pulsearc_common::storage::sqlcipher::{SqlCipherPool, StorageResult}`
- [ ] Use `pool.get_sqlcipher_connection()` (synchronous, no `.await`)
- [ ] Change `query_map(...)?.collect()` → `query_map(...)?` (remove `.collect()`)
- [ ] Remove `async` from trait methods and implementations
- [ ] Remove `.await` from all connection/query calls
- [ ] Update parameters to use `&[&dyn ToSql]` (reference to bound variable)
- [ ] Update error types from `rusqlite::Error` to `StorageError`
- [ ] Remove redundant `.map_err()` chains (only map once after `query_map`)
- [ ] Verify no `.into_iter()` or `.collect()` on query_map results
- [ ] Update tests to use `SqlCipherPool` instead of in-memory rusqlite

---

## Example: Before/After Migration

### Before (LocalDatabase with rusqlite)

```rust
use crate::db::local::LocalDatabase;
use rusqlite::{params, Result};

pub struct OldSegmentRepository {
    db: LocalDatabase,
}

impl OldSegmentRepository {
    async fn find_segments(&self, date: &str) -> Result<Vec<ActivitySegment>> {
        let conn = self.db.connection().await;
        let mut stmt = conn.prepare("SELECT * FROM segments WHERE date = ?1")?;

        // rusqlite: query_map returns iterator, need .collect()
        let segments = stmt
            .query_map(params![date], |row| {
                Ok(ActivitySegment {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;  // ✅ Correct for rusqlite

        Ok(segments)
    }
}
```

### After (SqlCipherPool)

```rust
use std::sync::Arc;
use pulsearc_common::storage::sqlcipher::{SqlCipherPool, StorageResult};
use rusqlite::ToSql;

pub struct SqlCipherSegmentRepository {
    pool: Arc<SqlCipherPool>,
}

impl SqlCipherSegmentRepository {
    fn find_segments(&self, date: &str) -> StorageResult<Vec<ActivitySegment>> {
        // ✅ Synchronous (no async/await)
        let conn = self.pool.get_sqlcipher_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM segments WHERE date = ?1")?;

        // ✅ SqlCipher: query_map returns Vec, DON'T call .collect()
        // ✅ Use &date as &dyn ToSql
        let segments = stmt.query_map(&[&date as &dyn ToSql], |row| {
            Ok(ActivitySegment {
                id: row.get(0)?,
                start_time: row.get(1)?,
            })
        })?;  // ✅ Correct for SqlCipherPool - already returns Vec

        Ok(segments)
    }
}
```

---

## Testing Notes

### Unit Tests (Mock Repository)

For unit tests, implement the port trait with in-memory storage:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    struct MockSegmentRepository {
        segments: Arc<RwLock<HashMap<String, Vec<ActivitySegment>>>>,
    }

    impl SegmentRepository for MockSegmentRepository {
        fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>> {
            let segments = self.segments.read().unwrap();
            Ok(segments.get(&date.to_string()).cloned().unwrap_or_default())
        }
    }
}
```

### Integration Tests (Real SqlCipher)

For integration tests, use a temporary database:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use pulsearc_common::storage::sqlcipher::{SqlCipherPool, SqlCipherPoolConfig};
    use tempfile::TempDir;

    #[test]
    fn test_repository_with_real_db() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = Arc::new(
            SqlCipherPool::new(&db_path, "test_key_64_chars_long".to_string(), config).unwrap()
        );
        let repo = SqlCipherSegmentRepository { pool };

        // Test repository operations...
    }
}
```

---

## Quick Reference Card

**Print this out and keep near your keyboard during refactoring!**

```
┌─────────────────────────────────────────────────────────────┐
│  SqlCipherPool Query Pattern                                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  // ✅ Synchronous (no async/await)                         │
│  let conn = pool.get_sqlcipher_connection()?;               │
│  let mut stmt = conn.prepare(sql)?;                         │
│                                                              │
│  let param_str = value.to_string();                         │
│  let results = stmt.query_map(                              │
│      &[&param_str as &dyn ToSql],                           │
│      |row| { ... }                                          │
│  )?;  // Returns Vec<T>, NO .collect()                      │
│                                                              │
│  Remember:                                                   │
│  1. NO async/await                                          │
│  2. query_map ALREADY returns Vec<T>                        │
│  3. Use &var as &dyn ToSql for parameters                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Related Documentation

- [Phase 0 Blockers Tracking](PHASE-0-BLOCKERS-TRACKING.md) - Full refactoring plan
- [SqlCipher Pool Implementation](../../crates/common/src/storage/sqlcipher/pool.rs) - Pool source code
- [SqlCipher Connection Implementation](../../crates/common/src/storage/sqlcipher/connection.rs) - Connection source code
- [ADR-003: Layered Architecture](../adr/ADR-003-layered-architecture.md) - Migration context

---

**Questions?** Check the source at:
- `crates/common/src/storage/sqlcipher/pool.rs:174` (get_sqlcipher_connection)
- `crates/common/src/storage/sqlcipher/connection.rs:116` (query_map)
