#!/usr/bin/env bash
# The playground's files (ticket 2814): copies Hello, Grid's text files the
# playground edits, from examples/hello-grid/files/, to
# site/static/playground/files/, which the build puts at
# /playground/files/ and git never keeps. The page's script fetches them
# from there, as the game's own text, which Reset puts back. ci.yml and
# pages.yml run it before `zola build`.
#
# The list is the page's (`extra.files` in content/learn/playground.md): a
# file it names that this doesn't copy fails here.
set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
from="$site_dir/../examples/hello-grid/files"
to="$site_dir/static/playground/files"
page="$site_dir/content/learn/playground.md"

files="$(sed -nE 's/^files = \[(.*)\]$/\1/p' "$page" | tr -d '",')"
[ -n "$files" ] || { echo "::error::$page has no files = [...] line" >&2; exit 1; }

rm -rf "$to"
mkdir -p "$to"
for name in $files; do
  [ -f "$from/$name" ] || { echo "::error::examples/hello-grid/files/$name isn't there, and the playground names it" >&2; exit 1; }
  cp "$from/$name" "$to/$name"
done
echo "Copied the playground's files to site/static/playground/files/: $files"
