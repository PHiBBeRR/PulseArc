#!/bin/bash
set -euo pipefail

# PulseArc Database Backup Script
# Creates timestamped backups with automatic retention management

# Configuration
DB_PATH="${1:-$HOME/Library/Application Support/com.pulsearc.app/pulsearc.db}"
BACKUP_DIR="${2:-./backups}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RETENTION_COUNT=10
SQLCIPHER_KEY="${PULSEARC_DB_KEY:-}"  # Optional encryption key from environment

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Ensure backup directory exists
mkdir -p "$BACKUP_DIR"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo -e "${RED}Error: Database not found at: $DB_PATH${NC}"
    echo "Usage: $0 [DB_PATH] [BACKUP_DIR]"
    exit 1
fi

# Verify source database integrity before backup
echo -e "${YELLOW}Verifying source database integrity...${NC}"
if command -v sqlite3 &> /dev/null; then
    INTEGRITY_CHECK=""
    if [ -n "$SQLCIPHER_KEY" ]; then
        # SQLCipher database - use encryption key
        # Note: Requires sqlite3 binary linked against SQLCipher library
        # Escape single quotes in key for SQL safety
        ESCAPED_KEY="${SQLCIPHER_KEY//\'/\'\'}"
        INTEGRITY_CHECK=$(sqlite3 "$DB_PATH" "PRAGMA key='$ESCAPED_KEY'; PRAGMA integrity_check;" 2>&1 || echo "FAILED")

        # Check for "no such function" error indicating sqlite3 isn't SQLCipher-enabled
        if [[ "$INTEGRITY_CHECK" == *"no such function"* ]] || [[ "$INTEGRITY_CHECK" == *"file is not a database"* ]]; then
            echo -e "${YELLOW}  Warning: sqlite3 binary may not be linked against SQLCipher${NC}"
            echo -e "${YELLOW}  Install SQLCipher-enabled sqlite3: brew install sqlcipher${NC}"
            echo -e "${YELLOW}  Skipping integrity check for encrypted database${NC}"
            INTEGRITY_CHECK="SKIPPED"
        fi
    else
        # Standard SQLite database
        INTEGRITY_CHECK=$(sqlite3 "$DB_PATH" "PRAGMA integrity_check;" 2>&1 || echo "FAILED")
    fi

    if [[ "$INTEGRITY_CHECK" == *"ok"* ]]; then
        echo -e "${GREEN}✓ Source database integrity verified${NC}"
    elif [[ "$INTEGRITY_CHECK" == "SKIPPED" ]]; then
        echo -e "${YELLOW}  (integrity check skipped)${NC}"
    else
        echo -e "${RED}✗ Source database integrity check failed${NC}"
        echo "  Result: $INTEGRITY_CHECK"
        echo -e "${YELLOW}  Warning: Proceeding with backup anyway (data may be corrupted)${NC}"
    fi
else
    echo -e "${YELLOW}  Warning: sqlite3 not found, skipping integrity check${NC}"
fi

# Create backup
BACKUP_FILE="$BACKUP_DIR/pulsearc_backup_$TIMESTAMP.db"
echo -e "\n${YELLOW}Creating backup...${NC}"
cp "$DB_PATH" "$BACKUP_FILE"

if [ ! -f "$BACKUP_FILE" ]; then
    echo -e "${RED}✗ Backup failed - file not created${NC}"
    exit 1
fi

# Verify backup integrity
echo -e "${YELLOW}Verifying backup integrity...${NC}"
if command -v sqlite3 &> /dev/null; then
    BACKUP_INTEGRITY=""
    if [ -n "$SQLCIPHER_KEY" ]; then
        # Escape single quotes in key for SQL safety
        ESCAPED_KEY="${SQLCIPHER_KEY//\'/\'\'}"
        BACKUP_INTEGRITY=$(sqlite3 "$BACKUP_FILE" "PRAGMA key='$ESCAPED_KEY'; PRAGMA integrity_check;" 2>&1 || echo "FAILED")

        # Check for SQLCipher compatibility
        if [[ "$BACKUP_INTEGRITY" == *"no such function"* ]] || [[ "$BACKUP_INTEGRITY" == *"file is not a database"* ]]; then
            echo -e "${YELLOW}  Warning: sqlite3 binary may not be linked against SQLCipher${NC}"
            echo -e "${YELLOW}  Skipping integrity check for encrypted database${NC}"
            BACKUP_INTEGRITY="SKIPPED"
        fi
    else
        BACKUP_INTEGRITY=$(sqlite3 "$BACKUP_FILE" "PRAGMA integrity_check;" 2>&1 || echo "FAILED")
    fi

    if [[ "$BACKUP_INTEGRITY" == *"ok"* ]]; then
        echo -e "${GREEN}✓ Backup integrity verified${NC}"
    elif [[ "$BACKUP_INTEGRITY" == "SKIPPED" ]]; then
        echo -e "${YELLOW}  (integrity check skipped)${NC}"
    else
        echo -e "${RED}✗ Backup integrity check failed${NC}"
        echo "  Result: $BACKUP_INTEGRITY"
        rm -f "$BACKUP_FILE"
        exit 1
    fi
fi

DB_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo -e "${GREEN}✓ Backup created successfully${NC}"
echo "  Location: $BACKUP_FILE"
echo "  Size: $DB_SIZE"

# Apply retention policy (keep last 10 backups)
echo -e "\n${YELLOW}Applying retention policy (keeping last $RETENTION_COUNT backups)...${NC}"
BACKUP_COUNT=$(find "$BACKUP_DIR" -name 'pulsearc_backup_*.db' -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')

if [ "$BACKUP_COUNT" -gt "$RETENTION_COUNT" ]; then
    REMOVE_COUNT=$((BACKUP_COUNT - RETENTION_COUNT))
    echo "  Found $BACKUP_COUNT backups, removing $REMOVE_COUNT oldest..."

    # Remove oldest backups (macOS-compatible: using find + sort + tail)
    find "$BACKUP_DIR" -name 'pulsearc_backup_*.db' -maxdepth 1 2>/dev/null | \
        sort | \
        head -n "$REMOVE_COUNT" | \
        while IFS= read -r old_backup; do
            echo "  Removing: $(basename "$old_backup")"
            rm -f "$old_backup"
        done
    echo -e "${GREEN}✓ Retention policy applied${NC}"
else
    echo "  Current backups: $BACKUP_COUNT (under limit)"
fi

# Summary
echo -e "\n${GREEN}Backup Summary:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
find "$BACKUP_DIR" -name 'pulsearc_backup_*.db' -maxdepth 1 2>/dev/null | \
    sort -r | \
    head -n "$RETENTION_COUNT" | \
    while IFS= read -r backup; do
        SIZE=$(du -h "$backup" 2>/dev/null | cut -f1)
        echo "  $(basename "$backup") ($SIZE)"
    done
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}Total backups: $BACKUP_COUNT${NC}"