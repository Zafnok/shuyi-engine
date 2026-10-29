#!/usr/bin/env bash
# The website's build checks (ADR-0048, tickets 2801 and 2802), run by
# ci.yml and pages.yml with `zola` on PATH.
#
#   site/check-build.sh                 checks the book's sources, writes
#                                       the problem codes' pages
#                                       (errors.sh), builds the site twice,
#                                       with two base URLs, and checks both
#                                       outputs
#   site/check-build.sh hosts DIR BASE  checks a built site in DIR, served
#                                       at BASE, loads nothing from another
#                                       host (pages.yml, after the examples
#                                       are copied in)
set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
failed=0

fail() {
  echo "::error::$*"
  failed=1
}

# Every address a page or a stylesheet in $1 loads (a `src`, the `href` of
# a `link`, a CSS `url()`), one per line, `&#x2F;` read as `/`.
loaded_addresses() {
  local dir="$1"
  {
    find "$dir" -name '*.html' -print0 | xargs -0 -r grep -ohiE '<(script|img|iframe|source|audio|video|embed|track|input)\b[^>]*\bsrc="[^"]*"' |
      grep -oiE 'src="[^"]*"' | sed -E 's/^src="(.*)"$/\1/I' || true
    find "$dir" -name '*.html' -print0 | xargs -0 -r grep -ohiE '<link\b[^>]*\bhref="[^"]*"' |
      grep -oiE 'href="[^"]*"' | sed -E 's/^href="(.*)"$/\1/I' || true
    find "$dir" -name '*.css' -print0 | xargs -0 -r grep -ohiE 'url\([^)]*\)' |
      sed -E "s/^url\(['\"]?([^'\")]*)['\"]?\)$/\1/I" || true
  } | sed -e 's/&#x2F;/\//g' -e 's/&#47;/\//g'
}

# Fails for each address in $1 that names a host other than $2's.
check_hosts() {
  local dir="$1" base="$2" address
  while IFS= read -r address; do
    case "$address" in
      "$base"*) ;;
      http:* | https:* | //*) fail "$dir loads $address from another host" ;;
    esac
  done < <(loaded_addresses "$dir")
}

if [ "${1:-}" = hosts ]; then
  check_hosts "$2" "$3"
  exit "$failed"
fi

# No source file names the Pages host: links come from the base URL.
if grep -rniF "zafnok.github.io" "$site_dir" --exclude=check-build.sh --exclude-dir=public; then
  fail "site/ names zafnok.github.io: build links from the base URL (get_url, @/ links)"
fi

# Every code block of the book comes from site/snippets/, which CI builds
# and tests, through the `snippet` shortcode: no fenced Rust of its own.
book="$site_dir/content/learn/book"
if grep -rnE '^[[:space:]]*(```|~~~)[[:space:]]*(rust|rs)([^[:alnum:]_-]|$)' "$book"; then
  fail "the book has a fenced Rust block: write it in site/snippets/ and show it with the snippet shortcode"
fi

# Learn links each engine crate's API docs: data/api.toml has a row for
# each `shuyi*` package under crates/, by rustdoc's name for it.
for manifest in "$site_dir"/../crates/*/Cargo.toml; do
  package="$(sed -nE 's/^name = "(shuyi[a-z0-9-]*)"$/\1/p' "$manifest" | head -n 1)"
  [ -n "$package" ] || continue
  grep -qxF "name = \"${package//-/_}\"" "$site_dir/data/api.toml" ||
    fail "site/data/api.toml has no row for $package (its API docs, /api/${package//-/_}/)"
done

bash "$site_dir/errors.sh"

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
one="https://one.invalid/first/"
two="https://two.invalid/second/"
(cd "$site_dir" && zola build --base-url "$one" --output-dir "$out/one" --force)
(cd "$site_dir" && zola build --base-url "$two" --output-dir "$out/two" --force)

# Changing the base URL alone moves every link: neither output names the
# other's base, nor the base in config.toml.
default_host="$(sed -nE 's/^base_url = "https?:\/\/([^/"]*).*"$/\1/p' "$site_dir/config.toml")"
for pair in "one:two.invalid" "two:one.invalid" "one:$default_host" "two:$default_host"; do
  dir="$out/${pair%%:*}"
  host="${pair#*:}"
  if grep -rlF "$host" "$dir"; then
    fail "the build at ${pair%%:*}.invalid still names $host"
  fi
done

# Every built page is there.
for page in index.html learn/index.html news/index.html news/atom.xml examples/index.html \
  examples/hello-grid/index.html hello-grid/index.html community/index.html sponsor/index.html \
  learn/book/index.html learn/errors/index.html learn/errors/sh0001/index.html; do
  [ -f "$out/one/$page" ] || fail "the build has no $page"
done
for chapter in "$book"/*.md; do
  name="$(basename "$chapter" .md)"
  [ "$name" = _index ] && continue
  [ -f "$out/one/learn/book/$name/index.html" ] || fail "the build has no learn/book/$name/"
done
# The chapters are numbered 1 to the last, none twice and none missing.
chapters="$(sed -nE 's/^chapter = ([0-9]+)$/\1/p' "$book"/*.md | sort -n | tr '\n' ' ')"
expected="$(seq 1 "$(echo "$chapters" | wc -w)" | tr '\n' ' ')"
[ "$chapters" = "$expected" ] || fail "the book's chapters are numbered ${chapters}rather than $expected"
grep -qF "${one}examples/hello-grid/" "$out/one/hello-grid/index.html" ||
  fail "/hello-grid/ doesn't redirect to /examples/hello-grid/"

check_hosts "$out/one" "$one"
check_hosts "$out/two" "$two"

[ "$failed" = 0 ] && echo "The site's build checks passed."
exit "$failed"
