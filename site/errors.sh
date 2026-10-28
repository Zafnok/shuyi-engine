#!/usr/bin/env bash
# The problem codes' pages (ticket 2802): each crates/cli/errors/SHnnnn.md,
# the page `shuyi explain` prints, written into site/content/learn/errors/
# as a page of Learn at /learn/errors/shnnnn/, with the list of them. Run
# it before `zola build`, `zola check` or `zola serve`: ci.yml, pages.yml
# and check-build.sh do. The folder is made each time, never kept
# (.gitignore), so crates/cli/errors/ stays the pages' one home.
set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
errors="$site_dir/../crates/cli/errors"
out="$site_dir/content/learn/errors"

rm -rf "$out"
mkdir -p "$out"
cat >"$out/_index.md" <<'EOF'
+++
title = "Problem codes"
description = "Every error Shuyi Engine's tools give, by its code: what it means and how to fix it."
sort_by = "weight"
template = "errors.html"
page_template = "page.html"
+++

Every error the engine's tools give has a stable code, `SH` and four
digits, and its first digit is its class: `SH0` the command is wrong,
`SH1` the setup, `SH2` a file of the game's, `SH3` the operation. A
program branches on the code, never on the message. Each page here is
the one `cargo shuyi explain <code>` prints.
EOF

count=0
for page in "$errors"/SH[0-9][0-9][0-9][0-9].md; do
  code="$(basename "$page" .md)"
  heading="$(head -n 1 "$page")"
  case "$heading" in
    "# $code: "*) ;;
    *) echo "::error::$page: its first line isn't \`# $code: <what it means>\`" >&2; exit 1 ;;
  esac
  # The title as a TOML string: backslashes and quotes escaped.
  title="$(printf '%s' "${heading#\# }" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g')"
  {
    printf '+++\ntitle = "%s"\nweight = %d\n+++\n' "$title" "$((10#${code#SH}))"
    tail -n +2 "$page"
  } >"$out/$(printf '%s' "$code" | tr '[:upper:]' '[:lower:]').md"
  count=$((count + 1))
done
[ "$count" -gt 0 ] || { echo "::error::no page in $errors" >&2; exit 1; }
echo "Wrote $count problem codes' pages to site/content/learn/errors/."
