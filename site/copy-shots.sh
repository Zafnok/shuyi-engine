#!/usr/bin/env bash
# The book's pictures of the examples (ticket 2813): copies each example's
# shots (examples/<name>/shots/*.png, which `cargo shuyi frame-png --shots`
# makes from its shuyi.ron and a test of the example keeps current) to
# site/static/shots/<name>/, which the build puts at /shots/<name>/. The
# `shot` shortcode shows one, and a shot that isn't there fails the build.
# git keeps none of these copies. ci.yml (through check-build.sh) and
# pages.yml run it before `zola build`.
set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
to="$site_dir/static/shots"

rm -rf "$to"
mkdir -p "$to"
copied=()
for shots in "$site_dir"/../examples/*/shots; do
  [ -d "$shots" ] || continue
  name="$(basename "$(dirname "$shots")")"
  mkdir -p "$to/$name"
  for shot in "$shots"/*.png; do
    [ -f "$shot" ] || continue
    cp "$shot" "$to/$name/"
  done
  copied+=("$name")
done
echo "Copied the examples' shots to ${to#"$site_dir"/}/: ${copied[*]:-none}"
