#!/usr/bin/env bash
# The oldest macOS a program built with the in-house sound runs on (ticket 3032).
#
#   bash tools/sound-parity/mac-floor.sh PROGRAM...
#
# CPAL's documentation names macOS 14.2, and Shuyi names no floor: a program asks
# for the one rustc writes in it (`minos`), and dyld refuses to start it on an older
# system if it imports, not weakly, a function that system lacks. So for each slice
# of each program this reads its `minos` and every function it imports from the
# audio frameworks, then has clang say whether the SDK's headers make any of them
# newer than that `minos`: a use of one is an error under
# `-Werror=unguarded-availability`. Run on macOS, with Xcode's command line tools.

set -euo pipefail

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
    # run time, so only the others can stop the program starting.
    nm -m -u -arch "$arch" "$program" \
      | grep -E '\(from (CoreAudio|AudioToolbox|AudioUnit|CoreAudioKit)\)' \
      | grep -v 'weak' \
      | awk '{ print $3 }' | sed 's/^_//' | grep -v '^OBJC_' | sort -u > "$work/symbols" || true
    count=$(wc -l < "$work/symbols" | tr -d ' ')
    {
      echo '#include <CoreAudio/CoreAudio.h>'
      echo '#include <AudioToolbox/AudioToolbox.h>'
      echo '#include <AudioUnit/AudioUnit.h>'
      echo 'const void *used[] = {'
      sed 's/.*/  (const void *)\&&,/' "$work/symbols"
      echo '  0 };'
    } > "$work/uses.c"
    target=$([ "$arch" = arm64 ] && echo arm64-apple-macos"$minos" || echo x86_64-apple-macos"$minos")
    if xcrun clang -c -x c -target "$target" -Werror=unguarded-availability \
      -Werror=unguarded-availability-new -o "$work/uses.o" "$work/uses.c" 2> "$work/said"; then
      echo "mac-floor: $program ($arch): minos $minos; its $count audio functions are all there"
    else
      echo "mac-floor: $program ($arch): minos $minos; newer than that, or unknown:"
      grep -E 'error' "$work/said" || cat "$work/said"
      status=1
    fi
    sed 's/^/  /' "$work/symbols"
  done
done
exit "$status"
