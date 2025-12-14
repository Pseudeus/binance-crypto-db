#!/bin/bash

# ==============================================================================
# Database Dump & Backup Script
# Optimized for Rust Integration (Structured Logs, Explicit Exit Codes)
# ==============================================================================

# -e: Exit immediately on error
# -u: Treat unset variables as error
# -o pipefail: Fail pipeline if any command fails
set -euo pipefail

# --- Configuration: Exit Codes ---
# Rust can check process.exit_code() against these values
E_SUCCESS=0
E_USAGE=1
E_DEPENDENCY=2
E_DIR_NOT_FOUND=3
E_FILE_NOT_FOUND=4
E_DUMP_FAILED=5
E_COMPRESS_FAILED=6
E_UPLOAD_FAILED=7
E_MOVE_FAILED=8

# --- Helper: Logging ---
# format: [YYYY-MM-DD HH:MM:SS] [LEVEL] Message
log() {
    local level="$1"
    shift
    local msg="$@"
    local timestamp
    timestamp=$(date +'%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $msg"
}

log_info() {
    log "INFO" "$@"
}

log_warn() {
    log "WARN" "$@"
}

log_error() {
    log "ERROR" "$@" >&2
}

# --- Helper: Cleanup ---
# Runs on exit to ensure temporary files are removed
cleanup() {
    local exit_code=$?
    if [ -n "${OUTPUT_SQL:-}" ] && [ -f "$OUTPUT_SQL" ]; then
        log_info "Cleaning up temporary SQL file: $OUTPUT_SQL"
        rm -f "$OUTPUT_SQL"
    fi

    if [ $exit_code -eq $E_SUCCESS ]; then
        log_info "Operation finished successfully."
    else
        log_error "Operation failed with exit code $exit_code."
    fi
}
trap cleanup EXIT

# --- Step 1: Input Validation ---
if [ "$#" -ne 2 ]; then
    log_error "Usage: $0 <working_directory> <database_file>"
    exit $E_USAGE
fi

WORKING_DIR="${1%/}" # Remove trailing slash if present
DB_FILE="$2"

# --- Step 2: Dependency Check ---
log_info "Checking system dependencies..."
for cmd in sqlite3 zstd rclone; do
    if ! command -v "$cmd" &> /dev/null; then
        log_error "Missing dependency: '$cmd' is not installed."
        exit $E_DEPENDENCY
    fi
    # Log version for debug traces
    version=$($cmd --version | head -n 1)
    log_info "Dependency '$cmd' found: $version"
done

# --- Step 3: Path Validation ---
CURRENT_DB_DIR="current"
ARCHIVED_DB_DIR="archived"
BACKUP_DIR=".backup"

DB_PATH="$WORKING_DIR/$CURRENT_DB_DIR/$DB_FILE"

if [ ! -d "$WORKING_DIR" ]; then
    log_error "Working directory not found: '$WORKING_DIR'"
    exit $E_DIR_NOT_FOUND
fi

if [ ! -f "$DB_PATH" ]; then
    log_error "Source database file not found: '$DB_PATH'"
    exit $E_FILE_NOT_FOUND
fi

DB_FILE_NAME=$(basename "$DB_FILE" .db)

# Ensure output directories exist
mkdir -p "$WORKING_DIR/$ARCHIVED_DB_DIR"
mkdir -p "$WORKING_DIR/$BACKUP_DIR"

OUTPUT_SQL="$WORKING_DIR/$ARCHIVED_DB_DIR/$DB_FILE_NAME.sql"
FINAL_ARCHIVE="$WORKING_DIR/$ARCHIVED_DB_DIR/$DB_FILE_NAME.sql.zst"

# --- Step 4: Database Dump ---
log_info "Dumping database '$DB_FILE'..."
log_info "Source: $DB_PATH"
log_info "Target: $OUTPUT_SQL"

if ! sqlite3 "$DB_PATH" .dump > "$OUTPUT_SQL"; then
    log_error "sqlite3 dump failed."
    exit $E_DUMP_FAILED
fi

dump_size=$(du -h "$OUTPUT_SQL" | cut -f1)
log_info "Dump created successfully. Size: $dump_size"

# --- Step 5: Compression ---
log_info "Compressing dump with zstd..."

# -f: overwrite output
# --rm: remove source file (SQL) after successful compression
# -T0: use all available cores
# -12: compression level 12 (balanced)
if ! zstd -f --rm -T0 -12 -o "$FINAL_ARCHIVE" "$OUTPUT_SQL" > /dev/null 2>&1; then
    log_error "zstd compression failed."
    exit $E_COMPRESS_FAILED
fi

if [ ! -f "$FINAL_ARCHIVE" ]; then
    log_error "Compression finished but output file '$FINAL_ARCHIVE' is missing."
    exit $E_COMPRESS_FAILED
fi

archive_size=$(du -h "$FINAL_ARCHIVE" | cut -f1)
log_info "Compression complete. Archive: $FINAL_ARCHIVE (Size: $archive_size)"

# --- Step 6: Upload ---
REMOTE_DEST="my_drive:orange_pi_db_backups"
log_info "Uploading to remote storage: $REMOTE_DEST"

# rclone output is captured to avoid cluttering logs unless it fails,
# or we can let it pass through if VERBOSE is needed.
# Here we use -v and let it write to stderr (rclone default for logs) so Rust can capture it.
if ! rclone copy --update -v "$FINAL_ARCHIVE" "$REMOTE_DEST"; then
    log_error "rclone upload failed."
    exit $E_UPLOAD_FAILED
fi
log_info "Upload successful."

# --- Step 7: Archiving Original ---
log_info "Moving processed database to backup folder..."
if ! mv "$DB_PATH" "$WORKING_DIR/$BACKUP_DIR/"; then
    log_error "Failed to move original DB file to '$WORKING_DIR/$BACKUP_DIR/'"
    exit $E_MOVE_FAILED
fi

log_info "Database moved to backup."
exit $E_SUCCESS
