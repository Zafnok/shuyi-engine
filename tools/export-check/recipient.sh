#!/usr/bin/env bash
# A closed build's copies (ticket 3302, ADR-0118), as CI's `export` job runs
# them after ci.sh's checks: two recipients' copies of the package
# `shuyi export <target> --release` makes, from a throwaway SSH key and the
# reveal kit fixture, then:
#
# - the public package is the same for both (the release's attestation of
#   it covers both copies), and the ledger names it for both;
# - the copies differ (their overlays' digests);
# - the detector names each copy's recipient from all three marks, and says
#   "no mark found" on the public package;
# - each copy passed the `disclosure` gate (its exposure report is ok), and
#   on macOS its `.app` is signed.
#
# Usage: bash tools/export-check/recipient.sh <target> <game folder>

set -euo pipefail

target=$1
game=$2
tmp="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/shuyi-recipient-$target"
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

rm -rf "$tmp" && mkdir -p "$tmp"
ssh-keygen -q -t ed25519 -N "" -C "ci" -f "$tmp/key"
rm -rf "$game/dist/reveal" "$game/dist/recipients" "$game/dist/recipients.ron" "$game/dist/recipients.ron.sig"
mkdir -p "$game/dist"
cp -R "$root/crates/cli/tests/fixtures/reveal/kit" "$game/dist/reveal"

public="dist/export/$target"
for who in press-a press-b; do
  (cd "$game" && cargo shuyi export "$target" --release --recipient "$who" \
    --label "$who, a test" --signing-key "$tmp/key" --json) > "$tmp/$who.json"
done

field() { sed -n "s/^ *\"$2\": \"\(.*\)\",\{0,1\}\r\{0,1\}$/\1/p" "$1" | head -n 1; }
package=$(field "$tmp/press-a.json" package)
for who in press-a press-b; do
  build=$(sed -n 's/^ *"build": "\(.*\)",\{0,1\}\r\{0,1\}$/\1/p' "$tmp/$who.json")
  overlay=$(sed -n 's/^ *"overlay": "\(.*\)",\{0,1\}\r\{0,1\}$/\1/p' "$tmp/$who.json")
  echo "$build" > "$tmp/$who.build"
  echo "$overlay" > "$tmp/$who.overlay"
  grep -q '"ok": true' "$game/dist/recipients/$who/$target/exposure-report.json" || {
    echo "check: $who's copy didn't pass the disclosure gate"; exit 1
  }
done
cmp -s "$tmp/press-a.build" "$tmp/press-b.build" || {
  echo "check: the two copies were made from different public packages"; exit 1
}
if cmp -s "$tmp/press-a.overlay" "$tmp/press-b.overlay"; then
  echo "check: the two recipients' copies are the same"; exit 1
fi
echo "check: both copies of the public package $(cat "$tmp/press-a.build"), each its own"

for who in press-a press-b; do
  code=$(field "$tmp/$who.json" code)
  (cd "$game" && cargo shuyi export --detect-recipient "dist/recipients/$who/$target/$package") > "$tmp/$who.found"
  cat "$tmp/$who.found"
  grep -q "3 of 3 mark(s) name the code $code" "$tmp/$who.found" || {
    echo "check: the detector didn't name $who's code $code from all three marks"; exit 1
  }
  grep -q "was given to $who " "$tmp/$who.found" || {
    echo "check: the ledger didn't name $who"; exit 1
  }
  grep -q "its signature verifies" "$tmp/$who.found" || {
    echo "check: the ledger's signature didn't verify"; exit 1
  }
done
(cd "$game" && cargo shuyi export --detect-recipient "$public/$package") > "$tmp/public.found"
cat "$tmp/public.found"
grep -q ": no mark found" "$tmp/public.found" || {
  echo "check: the public package holds a mark"; exit 1
}
grep -c "circumstantial evidence" "$tmp/public.found" > /dev/null

if [ "$target" = macos ]; then
  app=$(find "$game/dist/recipients/press-a/macos" -name '*.app' -maxdepth 2 | head -n 1)
  codesign --verify --strict "$app"
  echo "check: $app is signed"
fi
echo "check: each recipient's copy names them; the public package names no one"
