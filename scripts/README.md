# Scripts Directory

Development and operational scripts for PulseArc.

## Available Scripts

### [backup/](backup/)
Database backup and restore utilities with integrity verification.

```bash
# Create backup
./scripts/backup/backup-db.sh

# Restore latest backup
./scripts/backup/restore-db.sh latest
```

See [backup/BACKUP-README.md](backup/BACKUP-README.md) for full documentation.

### [test-features.sh](test-features.sh)
Feature flag testing script.

## Directory Structure

```
scripts/
├── README.md              # This file
├── backup/               # Database backup utilities
│   ├── BACKUP-README.md  # Detailed backup documentation
│   ├── backup-db.sh      # Create database backups
│   └── restore-db.sh     # Restore from backups
├── bench/                # Benchmark scripts
├── mac/                  # macOS-specific scripts
├── mdm/                  # MDM-related scripts
└── test-features.sh      # Feature flag testing
```
