#!/usr/bin/env bash
# The Examples cards' previews (ticket 2811): copies each example's
# preview.png (examples/<name>/preview.png, which `cargo shuyi frame-png
# --preview` makes from its shuyi.ron and a test of the example keeps
# current) to site/static/previews/<name>.png, which the build puts at
# /previews/<name>.png. The Examples card of an example with a preview
# plays it (templates/examples.html); an example without one has none.
# git keeps none of these copies. ci.yml (through check-build.sh) and
# pages.yml run it before `zola build`.
set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
to="$site_dir/static/previews"

rm -rf "$to"
mkdir -p "$to"
copied=()
for preview in "$site_dir"/../examples/*/preview.png; do
  [ -f "$preview" ] || continue
  name="$(basename "$(dirname "$preview")")"
  cp "$preview" "$to/$name.png"
  copied+=("$name")
done
echo "Copied the examples' previews to ${to#"$site_dir"/}/: ${copied[*]:-none}"
