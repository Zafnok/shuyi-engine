#!/usr/bin/env bash
# The website's build checks (ADR-0048, tickets 2801 and 2802), run by
# ci.yml and pages.yml with `zola` on PATH.
#
#   site/check-build.sh                 checks the book's sources, writes
#                                       the problem codes' pages
#                                       (errors.sh), copies the playground's
#                                       files (copy-playground.sh) and the
#                                       examples' previews
#                                       (copy-previews.sh), builds
#                                       the site twice, with two base URLs,
#                                       and checks both outputs
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

# The book's projects (ADR-0090): each folder of the book whose _index.md
# has `project = true`, its steps' code in site/<its `code`>/<step>/.
projects=()
for index in "$book"/*/_index.md; do
  [ -f "$index" ] && grep -qxF 'project = true' "$index" && projects+=("$(dirname "$index")")
done
[ "${#projects[@]}" -gt 0 ] || fail "the book has no project: no folder of it has an _index.md with project = true"

# Every code block of a project comes from its step's folder, which CI
# builds and tests: only the commands to type are written in a page.
for project in "${projects[@]}"; do
  if grep -rnE '^[[:space:]]*(```|~~~)[[:space:]]*($|[^b[:space:]]|b[^a])' "$project" | grep -vE ':[0-9]+:[[:space:]]*(```|~~~)[[:space:]]*$'; then
    fail "the project $(basename "$project") has a code block of its own: show its step's file with the snippet shortcode"
  fi
done

# Learn links each engine crate's API docs: data/api.toml has a row for
# each `shuyi*` package under crates/, by rustdoc's name for it.
for manifest in "$site_dir"/../crates/*/Cargo.toml; do
  package="$(sed -nE 's/^name = "(shuyi[a-z0-9-]*)"$/\1/p' "$manifest" | head -n 1)"
  [ -n "$package" ] || continue
  grep -qxF "name = \"${package//-/_}\"" "$site_dir/data/api.toml" ||
    fail "site/data/api.toml has no row for $package (its API docs, /api/${package//-/_}/)"
done

bash "$site_dir/errors.sh"
bash "$site_dir/copy-playground.sh"
bash "$site_dir/copy-previews.sh"

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
one="https://one.invalid/first/"
two="https://two.invalid/second/"
(cd "$site_dir" && zola build --base-url "$one" --output-dir "$out/one" --force) 2>&1 | tee "$out/build.log"
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
# Each step of a project has a page, which links its playable build, and
# follows a chapter of the book, so the book's order holds it.
for project in "${projects[@]}"; do
  folder="$(basename "$project")"
  code="$(sed -nE 's/^code = "(.*)"$/\1/p' "$project/_index.md")"
  [ -n "$code" ] && [ -d "$site_dir/$code" ] || fail "learn/book/$folder/_index.md: its code, site/$code/, isn't there"
  for step in "$site_dir/$code"/*/; do
    [ -d "$step" ] || continue
    name="$(basename "$step")"
    page="$out/one/learn/book/$folder/$name/index.html"
    [ -f "$page" ] || fail "the build has no learn/book/$folder/$name/"
    grep -qF "${one}learn/book/$folder/$name/play/" "$page" || fail "learn/book/$folder/$name/ doesn't link its play/"
  done
  for step in "$project"/*.md; do
    [ "$(basename "$step")" = _index.md ] && continue
    chapter="$(sed -nE 's/^follows = "(.*)"$/\1/p' "$step")"
    grep -qxF 'part = "chapters"' "$book/$chapter.md" 2>/dev/null ||
      fail "${step#"$site_dir"/}: follows = \"$chapter\" names no chapter of the book"
  done
done
# Each old address of a page (its `aliases`) sends the reader to the page.
while IFS= read -r source; do
  url="${source#"$site_dir"/content/}"
  url="${url%.md}"
  url="${url%_index}"
  url="${url%/}/"
  for alias in $(sed -nE 's/^aliases = \[(.*)\]$/\1/p' "$source" | tr -d '",'); do
    grep -qF "${one}$url" "$out/one/$alias/index.html" 2>/dev/null ||
      fail "/$alias doesn't redirect to /$url (${source#"$site_dir"/})"
  done
done < <(grep -rlE '^aliases = ' "$site_dir/content")
# The chapters are numbered 1 to the last, none twice and none missing, and
# the appendices are lettered from A the same way.
chapters="$(sed -nE 's/^chapter = ([0-9]+)$/\1/p' "$book"/*.md | sort -n | tr '\n' ' ')"
expected="$(seq 1 "$(echo "$chapters" | wc -w)" | tr '\n' ' ')"
[ "$chapters" = "$expected" ] || fail "the book's chapters are numbered ${chapters}rather than $expected"
appendices="$(sed -nE 's/^appendix = "([A-Z])"$/\1/p' "$book"/*.md | sort | tr -d '\n')"
[ -z "$appendices" ] || [ "$appendices" = "$(echo ABCDEFGHIJKLMNOPQRSTUVWXYZ | cut -c "1-${#appendices}")" ] ||
  fail "the book's appendices are lettered $appendices rather than from A, none twice and none missing"
# Every page of the book loads the reader's filter (ticket 2826), and every
# passage's mark names an audience the filter knows: one in the script's
# AUDIENCES, which the stylesheet folds.
audiences=" $(sed -nE 's/^ *const AUDIENCES = \[(.*)\];$/\1/p' "$site_dir/static/book-filter.js" | tr -d '",') "
[ "$audiences" != "  " ] || fail "site/static/book-filter.js has no AUDIENCES line"
while IFS= read -r page; do
  grep -qE '<script src="[^"]*/book-filter\.js' "$page" || fail "${page#"$out"/one/} doesn't load book-filter.js"
  # Every page of the book but its front page has the book's contents
  # beside it, marking the page itself (ticket 2834).
  if [ "$page" != "$out/one/learn/book/index.html" ]; then
    grep -qE '<script src="[^"]*/book-side\.js' "$page" || fail "${page#"$out"/one/} doesn't load book-side.js"
    grep -qF '<nav class="book-side" id="book-side"' "$page" || fail "${page#"$out"/one/} has no book's contents beside it"
    here="${page#"$out"/one/}"
    grep -qF "href=\"${one}${here%index.html}\" aria-current=\"page\"" "$page" ||
      fail "$here: the book's contents beside it doesn't mark it"
  fi
  # A page with a quick check (ticket 2827) loads the script that answers it.
  if grep -qF 'class="check-choices"' "$page"; then
    grep -qE '<script src="[^"]*/book-check\.js' "$page" || fail "${page#"$out"/one/} has a quick check and doesn't load book-check.js"
  fi
done < <(find "$out/one/learn/book" -name index.html)
for audience in $(grep -rhoE 'data-newcomer="[^"]*"' "$out/one/learn/book" | sort -u | sed -E 's/^data-newcomer="(.*)"$/\1/'); do
  [[ "$audiences" == *" $audience "* ]] || fail "a passage is marked $audience, which book-filter.js's AUDIENCES doesn't have"
  grep -qF "[data-book-fold~=\"$audience\"] .newcomer[data-newcomer=\"$audience\"]" "$site_dir/static/style.css" ||
    fail "a passage is marked $audience, which style.css doesn't fold"
done
grep -qF "${one}examples/hello-grid/" "$out/one/hello-grid/index.html" ||
  fail "/hello-grid/ doesn't redirect to /examples/hello-grid/"
# The playground (ticket 2814): the build holds each file its page names,
# the same as Hello, Grid's, and the page loads its script and frames the
# game.
playground="$out/one/learn/playground/index.html"
if [ -f "$playground" ]; then
  grep -qE '<script [^>]*src="[^"]*/playground\.js' "$playground" || fail "learn/playground/ doesn't load playground.js"
  grep -qF "<iframe src=\"${one}examples/hello-grid/\"" "$playground" || fail "learn/playground/ doesn't frame ${one}examples/hello-grid/"
else
  fail "the build has no learn/playground/"
fi
names="$(sed -nE 's/^files = \[(.*)\]$/\1/p' "$site_dir/content/learn/playground.md" | tr -d '",')"
[ "$(echo "$names" | wc -w)" = 5 ] || fail "the playground names $(echo "$names" | wc -w) files rather than Hello, Grid's five"
for name in $names; do
  cmp -s "$site_dir/../examples/hello-grid/files/$name" "$out/one/playground/files/$name" ||
    fail "the build's playground/files/$name isn't Hello, Grid's files/$name"
  grep -qF "data-file=\"$name\"" "$playground" 2>/dev/null || fail "learn/playground/ has no editor of $name"
done
# Each step's playground (ticket 2820): a step whose page names files has
# them in the build beside its page, the same as its project's own, an
# editor of each, and frames its play/.
for project in "${projects[@]}"; do
  folder="$(basename "$project")"
  code="$(sed -nE 's/^code = "(.*)"$/\1/p' "$project/_index.md")"
  game_files="$(sed -nE 's/^game_files = "(.*)"$/\1/p' "$project/_index.md")"
  for step in "$project"/*.md; do
    [ "$(basename "$step")" = _index.md ] && continue
    names="$(sed -nE 's/^files = \[(.*)\]$/\1/p' "$step" | tr -d '",')"
    [ -n "$names" ] || continue
    name="$(basename "$step" .md)"
    page="$out/one/learn/book/$folder/$name/index.html"
    [ -f "$page" ] || continue
    grep -qE '<script [^>]*src="[^"]*/playground\.js' "$page" || fail "learn/book/$folder/$name/ has a playground and doesn't load playground.js"
    grep -qF "<iframe src=\"${one}learn/book/$folder/$name/play/\"" "$page" ||
      fail "learn/book/$folder/$name/'s playground doesn't frame its play/"
    # Zola writes the `/` of `levels/1.map` as `&#x2F;`.
    decoded="$(sed -e 's/&#x2F;/\//g' "$page")"
    for file in $names; do
      cmp -s "$site_dir/$code/$name/$game_files/$file" "$out/one/learn/book/$folder/$name/files/$file" ||
        fail "the build's learn/book/$folder/$name/files/$file isn't site/$code/$name/$game_files/$file"
      grep -qF "data-file=\"$file\"" <<<"$decoded" || fail "learn/book/$folder/$name/ has no editor of $file"
    done
  done
done

# Each example's preview (ticket 2811): the build holds it as the example
# has it, and the Examples page names it in the example's card.
for preview in "$site_dir"/../examples/*/preview.png; do
  [ -f "$preview" ] || continue
  name="$(basename "$(dirname "$preview")")"
  cmp -s "$preview" "$out/one/previews/$name.png" ||
    fail "the build's previews/$name.png isn't examples/$name/preview.png"
  grep -qF "src=\"${one}previews/$name.png\"" "$out/one/examples/index.html" ||
    fail "the Examples page doesn't show previews/$name.png in $name's card"
done

check_hosts "$out/one" "$one"
check_hosts "$out/two" "$two"

# Code blocks are highlighted (ticket 2821): every language is known, and a
# page with a Rust block has its keywords marked, so highlighting can't be
# turned off by accident. (`fn` and `let` are `z-storage`, so a page is the
# unit: one block may have no `z-keyword` of its own.)
if grep -E 'language .* not found' "$out/build.log"; then
  fail "a code block names a language the build doesn't know: add its grammar to site/syntaxes/"
fi
while IFS= read -r page; do
  grep -qF 'z-keyword' "$page" || fail "$page has a Rust block with no z-keyword: is highlight_code on?"
done < <(grep -rlF 'data-lang="rust"' "$out/one" --include='*.html')

# The home page shows each card's status as its tickets make it (ticket
# 2815), once `cargo xtask site-data` has written them, as CI does first.
made="$site_dir/data/tickets.toml"
if [ -f "$made" ]; then
  shown="$(tr '\n' ' ' <"$out/one/index.html" |
    grep -oE '<h3>[^<]*</h3> *<p>[^<]*</p> *<p class="card-foot"> *<span class="status [^"]*">[^<]*</span>' |
    sed -E 's/^<h3>([^<]*)<\/h3>.*">([^<]*)<\/span>$/\1|\2/')"
  while IFS='|' read -r title status; do
    grep -qxF "$title|$status" <<<"$shown" ||
      fail "the home page doesn't show the card $title as $status, the status its tickets make"
  done < <(sed -nE -e 's/^\["(.*)"\]$/\1/p' -e 's/^status = "(.*)"$/|\1/p' "$made" | paste -d '' - -)
fi

[ "$failed" = 0 ] && echo "The site's build checks passed."
exit "$failed"
