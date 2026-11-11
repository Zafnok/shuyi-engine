#!/usr/bin/env bash
# The playgrounds' files (tickets 2814 and 2820): copies the text files each
# playground edits to where its page fetches them, as the game's own text,
# which Reset puts back. The build puts site/static/ at the site's root, and
# git keeps none of these copies. ci.yml and pages.yml run it before
# `zola build`.
#
# - Hello, Grid's playground (content/learn/playground.md): from
#   examples/hello-grid/files/ to site/static/playground/files/
#   (/playground/files/).
# - Each step of a project of the book (a folder of content/learn/book/
#   whose _index.md has `project = true`) whose page has a `files = [...]`
#   line: from site/<the project's `code`>/<step>/<its `game_files`>/ to
#   site/static/learn/book/<project>/<step>/files/, beside its page and its
#   `play/`.
#
# Each list is its page's (`extra.files`): a file it names that this doesn't
# copy fails here.
set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# The names on the `files = [...]` line of the page $1, or nothing.
listed() {
  sed -nE 's/^files = \[(.*)\]$/\1/p' "$1" | tr -d '",'
}

# Copies each of the names $3 from the folder $1 to the folder $2, which is
# made again from nothing, keeping their folders.
copy() {
  local from="$1" to="$2" names="$3" name
  rm -rf "$to"
  mkdir -p "$to"
  for name in $names; do
    [ -f "$from/$name" ] || { echo "::error::${from#"$site_dir"/}/$name isn't there, and its playground names it" >&2; exit 1; }
    mkdir -p "$(dirname "$to/$name")"
    cp "$from/$name" "$to/$name"
  done
  echo "Copied a playground's files to ${to#"$site_dir"/}/: $names"
}

page="$site_dir/content/learn/playground.md"
files="$(listed "$page")"
[ -n "$files" ] || { echo "::error::$page has no files = [...] line" >&2; exit 1; }
copy "$site_dir/../examples/hello-grid/files" "$site_dir/static/playground/files" "$files"

rm -rf "$site_dir/static/learn"
for index in "$site_dir"/content/learn/book/*/_index.md; do
  grep -qxF 'project = true' "$index" || continue
  project="$(dirname "$index")"
  folder="$(basename "$project")"
  code="$(sed -nE 's/^code = "(.*)"$/\1/p' "$index")"
  game_files="$(sed -nE 's/^game_files = "(.*)"$/\1/p' "$index")"
  for step in "$project"/*.md; do
    [ "$step" = "$index" ] && continue
    files="$(listed "$step")"
    [ -n "$files" ] || continue
    [ -n "$code" ] && [ -n "$game_files" ] ||
      { echo "::error::$index needs code = and game_files = lines for its steps' playgrounds" >&2; exit 1; }
    name="$(basename "$step" .md)"
    copy "$site_dir/$code/$name/$game_files" "$site_dir/static/learn/book/$folder/$name/files" "$files"
  done
done
