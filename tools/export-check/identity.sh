#!/usr/bin/env bash
# The game's identity in each desktop package (ticket 3049), as CI's
# `export` job checks it after ci.sh's checks of the example: the fixture
# game Lune Café (`lune-cafe/`, an `app` part and a 1024-pixel icon)
# exported for the target with `--release`, then read back:
#
# - windows: the program has a `.rsrc` section, its version resource and
#   its strings in UTF-16LE (the `#[ignore]`d test
#   `exported::the_exported_program_says_who_the_game_is`);
# - linux: the desktop entry and the hicolor icons are in the `tar.gz`, and
#   `desktop-file-validate` passes on the entry;
# - macos: the `.app` is named after the game, its Info.plist passes
#   `plutil -lint` and has the id, `iconutil` reads its `.icns`, and the
#   signature covers both.
#
# Usage: bash tools/export-check/identity.sh <target>, from the repository's
# root.

set -euo pipefail

target=$1
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
game="$here/lune-cafe"
case "$target" in
  windows|linux|macos) ;;
  *) echo "identity: no identity to check in a $target package"; exit 0 ;;
esac

# Its own workspace: the engine's tools by the root's manifest.
(cd "$game" && cargo run -q --manifest-path "$root/Cargo.toml" -p shuyi-cli -- export "$target" --release)
out="$game/dist/export/$target"
folder="lune-cafe-1.2.3-$target"
id=io.github.zafnok.shuyi-engine.lune-cafe

case "$target" in
  windows)
    exe="$out/$folder/lune-cafe.exe"
    if command -v cygpath > /dev/null; then exe=$(cygpath -w "$exe"); fi
    SHUYI_EXPORTED_EXE="$exe" cargo test -q -p shuyi-cli --test it -- \
      --ignored --exact exported::the_exported_program_says_who_the_game_is
    ;;
  linux)
    listed=$(tar tzf "$out/$folder.tar.gz")
    wanted="share/applications/$id.desktop"
    for side in 16 32 48 64 128 256 512; do
      wanted="$wanted share/icons/hicolor/${side}x${side}/apps/$id.png"
    done
    for path in $wanted; do
      printf '%s\n' "$listed" | grep -qxF "$folder/$path" || {
        echo "identity: $folder.tar.gz lacks $path"; exit 1
      }
    done
    if ! command -v desktop-file-validate > /dev/null; then
      as_root=""
      if [ "$(id -u)" != 0 ]; then as_root=sudo; fi
      $as_root apt-get update -qq
      $as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends desktop-file-utils
    fi
    desktop-file-validate "$out/$folder/share/applications/$id.desktop"
    echo "identity: the desktop entry is valid; the icons are at their hicolor paths"
    ;;
  macos)
    app="$out/$folder/Lune Café.app"
    [ -d "$app" ] || { echo "identity: no $app"; ls "$out/$folder"; exit 1; }
    plist="$app/Contents/Info.plist"
    plutil -lint "$plist"
    found=$(plutil -extract CFBundleIdentifier raw "$plist")
    [ "$found" = "$id" ] || { echo "identity: CFBundleIdentifier is $found"; exit 1; }
    iconset="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/AppIcon.iconset"
    rm -rf "$iconset"
    iconutil -c iconset -o "$iconset" "$app/Contents/Resources/AppIcon.icns"
    ls "$iconset"
    codesign --verify --strict "$app"
    echo "identity: Info.plist and AppIcon.icns read back, under the signature"
    ;;
esac
echo "identity: $target OK"
