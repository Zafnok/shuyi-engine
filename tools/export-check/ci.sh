#!/usr/bin/env bash
# The engine's checks of a package `export.yml` built (ticket 3050), which
# CI's `export` job hands it as its `check` script, run from the
# repository's root: check.sh on the package (ticket 3004), then the same
# input gives the same exposure report (ticket 3081): every hash in it, the
# program's and the archive's too.
#
# Usage: bash tools/export-check/ci.sh <target> <game folder>

set -euo pipefail

target=$1
game=$2
out="$game/dist/export/$target"
here=$(cd "$(dirname "$0")" && pwd)

bash "$here/check.sh" "$target" "$out"

first="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/exposure-report.json"
cp "$out/exposure-report.json" "$first"
(cd "$game" && cargo shuyi export "$target" --release)
diff "$first" "$out/exposure-report.json"
