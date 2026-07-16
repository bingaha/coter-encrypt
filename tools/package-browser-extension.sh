#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
extension_dir="$repo_root/browser-extension"
output_dir="$repo_root/dist"
zip_path="$output_dir/coter-cookie-bridge.zip"

if [[ ! -d "$extension_dir" ]]; then
  echo "Extension directory not found: $extension_dir" >&2
  exit 1
fi

mkdir -p "$output_dir"
rm -f "$zip_path"

if command -v zip >/dev/null 2>&1; then
  (
    cd "$extension_dir"
    zip -r "$zip_path" . -x '.*' -x '__MACOSX/*'
  )
else
  EXT_DIR="$extension_dir" ZIP_PATH="$zip_path" python3 - <<'PY'
import os
import pathlib
import zipfile

extension = pathlib.Path(os.environ["EXT_DIR"])
out = pathlib.Path(os.environ["ZIP_PATH"])
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in extension.rglob("*"):
        if not path.is_file():
            continue
        rel = path.relative_to(extension)
        if any(part.startswith(".") for part in rel.parts):
            continue
        zf.write(path, rel.as_posix())
print(out)
PY
fi

echo "Browser extension package created:"
echo "$zip_path"
