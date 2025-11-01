# Database Backup & Restore

Production-ready backup utilities with integrity verification and automatic retention.

## Quick Start

```bash
# Create backup
./scripts/backup/backup-db.sh

# Restore latest
./scripts/backup/restore-db.sh latest
```

## Features

**[backup-db.sh](backup-db.sh)**
- ✅ Integrity verification (before & after)
- ✅ Automatic retention (keeps 10 backups)
- ✅ SQLCipher encryption support
- ✅ Zero-downtime operation

**[restore-db.sh](restore-db.sh)**
- ✅ Pre-restore integrity check
- ✅ Automatic safety backup
- ✅ Post-restore verification
- ✅ Automatic rollback on failure

## SQLCipher Support

Both scripts auto-detect encrypted databases via `PULSEARC_DB_KEY`:

```bash
export PULSEARC_DB_KEY="your-encryption-key"
./scripts/backup/backup-db.sh
```

**Note:** Requires SQLCipher-enabled `sqlite3`:
```bash
brew install sqlcipher
# Or use: brew install sqlite --with-sqlcipher
```

## Backup Strategy

**When to backup:**
- ✅ Before each phase migration
- ✅ Before risky operations
- ✅ Before schema changes
- ✅ Daily during active development

**Example (Phase 4):**
```bash
# Phase 1: Before expanding AppContext
./scripts/backup/backup-db.sh

# Phase 2: Before first command migration
./scripts/backup/backup-db.sh

# Phase 3: Before frontend changes
./scripts/backup/backup-db.sh

# Phase 4: Before enabling feature flags
./scripts/backup/backup-db.sh
```

## Storage

**Default locations:**
- Backups: `./backups/`
- Database: `~/Library/Application Support/com.pulsearc.app/pulsearc.db`

**Retention policy:**
- Keeps: Last 10 backups
- Auto-deletes: Older backups

## Advanced Usage

### Custom Locations

```bash
# Custom database path
./scripts/backup/backup-db.sh ~/custom/pulsearc.db

# Custom backup directory
./scripts/backup/backup-db.sh "" ~/PulseArcBackups

# Both custom
./scripts/backup/backup-db.sh ~/custom/pulsearc.db ~/PulseArcBackups
```

### Restore Specific Backup

```bash
# List available backups
./scripts/backup/restore-db.sh

# Restore specific backup
./scripts/backup/restore-db.sh ./backups/pulsearc_backup_20250131_120000.db
```

### Manual Integrity Check

```bash
# Standard SQLite
sqlite3 pulsearc.db "PRAGMA integrity_check;"

# SQLCipher (encrypted)
export PULSEARC_DB_KEY="your-key"
sqlite3 pulsearc.db "PRAGMA key='$PULSEARC_DB_KEY'; PRAGMA integrity_check;"
```

## Safety Features

**Backup script:**
1. Checks source database integrity
2. Creates timestamped backup
3. Verifies backup integrity
4. Deletes backup if verification fails

**Restore script:**
1. Verifies backup file integrity
2. Creates safety backup of current DB
3. Restores from backup
4. Verifies restored database
5. Auto-rollback if verification fails

## Troubleshooting

### "sqlite3 not found"
```bash
brew install sqlite3
```

### "file is not a database" (SQLCipher)
```bash
# Set encryption key
export PULSEARC_DB_KEY="your-key-here"

# Install SQLCipher-enabled sqlite3
brew install sqlcipher
```

### Database locked
1. Stop PulseArc application
2. Wait for operations to complete
3. Retry backup

### Permission denied
```bash
# Fix backup directory
mkdir -p ./backups
chmod 755 ./backups
```

## Implementation Details

**Robust file handling:**
- Uses `find` + `sort -z` (not `ls` parsing)
- Handles spaces/special chars in paths
- Escapes single quotes in SQLCipher keys

**SQLCipher detection:**
- Auto-detects missing SQLCipher support
- Graceful degradation (warns & continues)
- Clear error messages

**Retention algorithm:**
```bash
# Keeps 10 most recent, removes oldest
find "$BACKUP_DIR" -name 'pulsearc_backup_*.db' -print0 | \
  sort -z | \
  head -zn $((count - 10)) | \
  xargs -0 rm -f
```

## File Naming

```
Format: pulsearc_backup_YYYYMMDD_HHMMSS.db
Example: pulsearc_backup_20250131_143025.db

Safety backups: pre_restore_safety_YYYYMMDD_HHMMSS.db
```

## Exit Codes

- `0` — Success
- `1` — Error (see output for details)

## Requirements

- Bash 4.0+
- `sqlite3` (standard or SQLCipher-enabled)
- 2x database size free disk space
- Read/write permissions

## Security

⚠️ **Backups contain all data** — store securely

**Best practices:**
```bash
# Use keychain for encryption key
export PULSEARC_DB_KEY=$(security find-generic-password -s "pulsearc-db-key" -w)

# Encrypt backup directory
diskutil apfs enableFileVault /path/to/backups

# Secure deletion
rm -P ./backups/old_backup.db  # macOS
# or
shred -u ./backups/old_backup.db  # Linux
```

## Related Docs

- [Phase 4 Migration](../docs/active-issue/PHASE-4-NEW-CRATE-MIGRATION.md) — When to backup
- [SQLCipher Reference](../docs/issues/completed/SQLCIPHER-API-REFERENCE.md) — Encryption details
