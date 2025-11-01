#!/bin/bash
set -euo pipefail

# PulseArc Database Restore Script
# Restores database from backup with safety checks

# Configuration
DB_PATH="${2:-$HOME/Library/Application Support/com.pulsearc.app/pulsearc.db}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
SQLCIPHER_KEY="${PULSEARC_DB_KEY:-}"  # Optional encryption key from environment

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Show usage
show_usage() {
    echo "Usage: $0 <backup_file> [target_db_path]"
    echo ""
    echo "Examples:"
    echo "  $0 ./backups/pulsearc_backup_20250131_120000.db"
    echo "  $0 latest  # Restore most recent backup"
    echo ""
    echo "Available backups:"
    # Ensure backup directory exists and check for backups
    if [[ -d "$BACKUP_DIR" ]]; then
        BACKUP_LIST=$(find "$BACKUP_DIR" -name 'pulsearc_backup_*.db' -maxdepth 1 2>/dev/null | sort -r | head -n 10)
        if [ -n "$BACKUP_LIST" ]; then
            echo "$BACKUP_LIST" | while IFS= read -r backup; do
                SIZE=$(du -h "$backup" 2>/dev/null | cut -f1)
                MODIFIED=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$backup" 2>/dev/null || date -r "$backup" "+%Y-%m-%d %H:%M" 2>/dev/null || echo "unknown")
                echo "  $(basename "$backup") ($SIZE, $MODIFIED)"
            done
        else
            echo "  No backups found in $BACKUP_DIR"
        fi
    else
        echo "  No backups found in $BACKUP_DIR (directory doesn't exist)"
    fi
    exit 1
}

# Check arguments
if [ $# -lt 1 ]; then
    show_usage
fi

# Determine backup file to restore
BACKUP_FILE="$1"
if [ "$BACKUP_FILE" = "latest" ]; then
    BACKUP_FILE=$(find "$BACKUP_DIR" -name 'pulsearc_backup_*.db' -maxdepth 1 2>/dev/null | sort -r | head -n 1)
    if [ -z "$BACKUP_FILE" ]; then
        echo -e "${RED}Error: No backups found in $BACKUP_DIR${NC}"
        exit 1
    fi
    echo -e "${BLUE}Using latest backup: $(basename "$BACKUP_FILE")${NC}"
fi

# Validate backup file exists
if [ ! -f "$BACKUP_FILE" ]; then
    echo -e "${RED}Error: Backup file not found: $BACKUP_FILE${NC}"
    show_usage
fi

# Verify backup file integrity before restore
echo -e "${YELLOW}Verifying backup file integrity...${NC}"
if command -v sqlite3 &> /dev/null; then
    BACKUP_INTEGRITY=""
    if [ -n "$SQLCIPHER_KEY" ]; then
        # Escape single quotes in key for SQL safety
        ESCAPED_KEY="${SQLCIPHER_KEY//\'/\'\'}"
        BACKUP_INTEGRITY=$(sqlite3 "$BACKUP_FILE" "PRAGMA key='$ESCAPED_KEY'; PRAGMA integrity_check;" 2>&1 || echo "FAILED")

        # Check for SQLCipher compatibility
        if [[ "$BACKUP_INTEGRITY" == *"no such function"* ]] || [[ "$BACKUP_INTEGRITY" == *"file is not a database"* ]]; then
            echo -e "${YELLOW}  Warning: sqlite3 binary may not be linked against SQLCipher${NC}"
            echo -e "${YELLOW}  Install SQLCipher-enabled sqlite3: brew install sqlcipher${NC}"
            echo -e "${YELLOW}  Skipping integrity check for encrypted database${NC}"
            BACKUP_INTEGRITY="SKIPPED"
        fi
    else
        BACKUP_INTEGRITY=$(sqlite3 "$BACKUP_FILE" "PRAGMA integrity_check;" 2>&1 || echo "FAILED")
    fi

    if [[ "$BACKUP_INTEGRITY" == *"ok"* ]]; then
        echo -e "${GREEN}✓ Backup file integrity verified${NC}"
    elif [[ "$BACKUP_INTEGRITY" == "SKIPPED" ]]; then
        echo -e "${YELLOW}  (integrity check skipped)${NC}"
    else
        echo -e "${RED}✗ Backup file integrity check failed${NC}"
        echo "  Result: $BACKUP_INTEGRITY"
        echo -e "${RED}Cannot restore from corrupted backup${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}  Warning: sqlite3 not found, skipping integrity check${NC}"
fi

# Ensure backup directory exists before creating safety backup
mkdir -p "$BACKUP_DIR"

# Create a safety backup of current database before restoring
if [ -f "$DB_PATH" ]; then
    SAFETY_BACKUP="$BACKUP_DIR/pre_restore_safety_$(date +%Y%m%d_%H%M%S).db"
    echo -e "\n${YELLOW}Creating safety backup of current database...${NC}"
    cp "$DB_PATH" "$SAFETY_BACKUP"
    echo -e "${GREEN}✓ Safety backup created: $SAFETY_BACKUP${NC}"
fi

# Confirm restore operation
echo -e "\n${YELLOW}⚠️  WARNING: This will replace the current database${NC}"
echo -e "  From: ${BLUE}$BACKUP_FILE${NC}"
echo -e "  To:   ${BLUE}$DB_PATH${NC}"
echo -e "  Size: $(du -h "$BACKUP_FILE" | cut -f1)"
echo ""
read -p "Continue with restore? (yes/no): " -r
echo

if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo -e "${YELLOW}Restore cancelled${NC}"
    exit 0
fi

# Ensure target database directory exists
mkdir -p "$(dirname "$DB_PATH")"

# Perform restore
echo -e "${YELLOW}Restoring database...${NC}"
cp "$BACKUP_FILE" "$DB_PATH"

if [ ! -f "$DB_PATH" ]; then
    echo -e "${RED}✗ Restore failed - file not created${NC}"
    if [ -n "${SAFETY_BACKUP:-}" ]; then
        echo "  Restoring from safety backup..."
        cp "$SAFETY_BACKUP" "$DB_PATH"
        echo -e "${GREEN}✓ Original database restored${NC}"
    fi
    exit 1
fi

# Verify restored database integrity
echo -e "${YELLOW}Verifying restored database integrity...${NC}"
if command -v sqlite3 &> /dev/null; then
    RESTORE_INTEGRITY=""
    if [ -n "$SQLCIPHER_KEY" ]; then
        # Escape single quotes in key for SQL safety
        ESCAPED_KEY="${SQLCIPHER_KEY//\'/\'\'}"
        RESTORE_INTEGRITY=$(sqlite3 "$DB_PATH" "PRAGMA key='$ESCAPED_KEY'; PRAGMA integrity_check;" 2>&1 || echo "FAILED")

        # Check for SQLCipher compatibility
        if [[ "$RESTORE_INTEGRITY" == *"no such function"* ]] || [[ "$RESTORE_INTEGRITY" == *"file is not a database"* ]]; then
            echo -e "${YELLOW}  Warning: sqlite3 binary may not be linked against SQLCipher${NC}"
            echo -e "${YELLOW}  Skipping integrity check for encrypted database${NC}"
            RESTORE_INTEGRITY="SKIPPED"
        fi
    else
        RESTORE_INTEGRITY=$(sqlite3 "$DB_PATH" "PRAGMA integrity_check;" 2>&1 || echo "FAILED")
    fi

    if [[ "$RESTORE_INTEGRITY" == *"ok"* ]]; then
        echo -e "${GREEN}✓ Restored database integrity verified${NC}"
    elif [[ "$RESTORE_INTEGRITY" == "SKIPPED" ]]; then
        echo -e "${YELLOW}  (integrity check skipped)${NC}"
    else
        echo -e "${RED}✗ Restored database integrity check failed${NC}"
        echo "  Result: $RESTORE_INTEGRITY"
        if [ -n "${SAFETY_BACKUP:-}" ]; then
            echo "  Restoring from safety backup..."
            cp "$SAFETY_BACKUP" "$DB_PATH"
            echo -e "${GREEN}✓ Original database restored${NC}"
        fi
        exit 1
    fi
fi

echo -e "\n${GREEN}✓ Database restored successfully${NC}"
echo ""
echo -e "${GREEN}Restore Summary:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Restored from: $(basename "$BACKUP_FILE")"
echo "  Database size: $(du -h "$DB_PATH" | cut -f1)"
if [ -n "${SAFETY_BACKUP:-}" ]; then
    echo "  Safety backup: $SAFETY_BACKUP"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Test the application to verify the restore"
if [ -n "${SAFETY_BACKUP:-}" ]; then
    echo "  2. If issues occur, restore from: $SAFETY_BACKUP"
    echo "  3. Once verified, you can delete the safety backup"
fi