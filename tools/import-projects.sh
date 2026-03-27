#!/usr/bin/env bash
# Import projects from a zip (or tar.gz) archive into the projects directory (overwrite on conflicts)
set -euo pipefail

ARCHIVE=${1:-}
DEST_DIR=${2:-"$(dirname "$0")/../projects"}

if [ -z "$ARCHIVE" ]; then
 echo "Usage: $0 <archive(.zip|.tar.gz|.tgz)> [dest_dir]" >&2
 exit 1
fi

mkdir -p "$DEST_DIR"

case "$ARCHIVE" in
 *.zip)
 if ! command -v unzip >/dev/null 2>&1; then
 echo "unzip command not found" >&2
 exit 1
 fi
 # extract files into DEST_DIR, overwrite
 unzip -o "$ARCHIVE" -d "$DEST_DIR" >/dev/null
 ;;
 *.tar.gz|*.tgz)
 tar -xzf "$ARCHIVE" -C "$DEST_DIR"
 ;;
 *)
 echo "Unsupported archive format: $ARCHIVE" >&2
 exit 1
 ;;
esac

echo "Imported into: $DEST_DIR"

