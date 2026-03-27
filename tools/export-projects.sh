#!/usr/bin/env bash
# Export all project config files from projects directory into a zip (or tar.gz fallback)
set -euo pipefail

SRC_DIR=${1:-"$(dirname "$0")/../projects"}
OUT_DIR=${2:-"$(pwd)"}

timestamp() {
 date +"%Y%m%d-%H%M%S"
}

if [ ! -d "$SRC_DIR" ]; then
 echo "Projects directory not found: $SRC_DIR" >&2
 exit 1
fi

mkdir -p "$OUT_DIR"

OUT_ZIP="$OUT_DIR/projects-$(timestamp).zip"

# Collect regular, non-hidden files only
mapfile -t files < <(find "$SRC_DIR" -maxdepth 1 -type f ! -name ".*" -printf "%f\n")
if [ ${#files[@]} -eq 0 ]; then
 echo "No project files in $SRC_DIR" >&2
 exit 1
fi

if command -v zip >/dev/null 2>&1; then
 (
 cd "$SRC_DIR"
 zip -q "$OUT_ZIP" "${files[@]}"
 )
 echo "Exported: $OUT_ZIP"
 exit 0
fi

# Fallback to tar.gz if zip is not available
OUT_TGZ="$OUT_DIR/projects-$(timestamp).tar.gz"
(
 cd "$SRC_DIR"
 tar -czf "$OUT_TGZ" --no-recursion "${files[@]}"
)
echo "zip not found; exported tarball: $OUT_TGZ"

