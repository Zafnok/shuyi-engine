#!/usr/bin/env bash
# The oldest macOS a program built with the in-house sound starts on (ticket 3032).
#
#   bash tools/sound-parity/mac-floor.sh [--expect VERSION] PROGRAM...
#
# CPAL's documentation names macOS 14.2, and Shuyi names no floor: a program asks
# for the one rustc writes in it (`minos`), and dyld can refuse to start it, or stop
# it at the first call, on an older system if it imports, not weakly, a function that
# system lacks. So for each slice of each program this reads its `minos` and every
# function it imports from the audio frameworks, and has clang say, from the SDK's
# headers, which of them are newer than that `minos`. It prints the floor each slice
# needs: the newest of `minos` and theirs. A function the headers don't declare fails,
# as its floor is unknown. With `--expect`, a floor other than VERSION fails too, so a
# change in what CPAL imports is seen. Run on macOS, with Xcode's command line tools.

set -euo pipefail

expect=
if [ "${1:-}" = --expect ]; then
  expect=$2
  shift 2
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
status=0
for program in "$@"; do
  for arch in $(lipo -archs "$program"); do
    # `LC_BUILD_VERSION`'s `minos`, or on an older target (rustc's x86_64 default)
    # `LC_VERSION_MIN_MACOSX`'s `version`.
    minos=$(otool -arch "$arch" -l "$program" | awk '
      /cmd LC_BUILD_VERSION/ { build = 1 } /cmd LC_VERSION_MIN_MACOSX/ { old = 1 }
      build && $1 == "minos" { print $2; exit } old && $1 == "version" { print $2; exit }')
    [ -n "$minos" ] || { echo "mac-floor: $program ($arch) records no minos"; exit 1; }
    # `nm -m` names each undefined symbol's library; a weak one may be missing at
    # run time, so only the others can stop the program.
    nm -m -u -arch "$arch" "$program" \
      | grep -E '\(from (CoreAudio|AudioToolbox|AudioUnit|CoreAudioKit)\)' \
      | grep -v 'weak' \
      | awk '{ print $3 }' | sed 's/^_//' | grep -v '^OBJC_' | sort -u > "$work/symbols" || true
    count=$(wc -l < "$work/symbols" | tr -d ' ')
    {
      echo '#import <CoreAudio/CoreAudio.h>'
      echo '#import <CoreAudio/AudioHardwareTapping.h>'
      echo '#import <AudioToolbox/AudioToolbox.h>'
      echo '#import <AudioUnit/AudioUnit.h>'
      echo 'const void *used[] = {'
      sed 's/.*/  (const void *)\&&,/' "$work/symbols"
      echo '  0 };'
    } > "$work/uses.m"
    target=$([ "$arch" = arm64 ] && echo arm64-apple-macos"$minos" || echo x86_64-apple-macos"$minos")
    if ! xcrun clang -c -x objective-c -target "$target" -Wunguarded-availability \
      -Wunguarded-availability-new -o "$work/uses.o" "$work/uses.m" 2> "$work/said"; then
      echo "mac-floor: $program ($arch): a function the SDK's headers don't declare:"
      grep -E 'error' "$work/said" || cat "$work/said"
      status=1
      continue
    fi
    # "'Name' is only available on macOS 14.2 or newer"
    newer=$(sed -n "s/.*'\([A-Za-z0-9_]*\)' is only available on macOS \([0-9.]*\) or newer.*/\2 \1/p" "$work/said" | sort -u)
    floor=$( (echo "$minos"; echo "$newer" | awk 'NF { print $1 }') | sort -V | tail -n 1)
    echo "mac-floor: $program ($arch): minos $minos, $count audio functions; it needs macOS $floor"
    if [ -n "$newer" ]; then
      echo "$newer" | sed 's/^\([^ ]*\) \(.*\)/  \2 needs macOS \1/'
    fi
    if [ -n "$expect" ] && [ "$floor" != "$expect" ]; then
      echo "mac-floor: expected macOS $expect: what CPAL imports changed, so ADR-0139 and 3349 are to be read again"
      status=1
    fi
  done
done
exit "$status"
