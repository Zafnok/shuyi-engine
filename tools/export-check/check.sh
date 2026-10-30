#!/usr/bin/env bash
# Checks a package `shuyi export <target> --release` wrote (ticket 3004),
# as CI's `export` job runs it: its files against the list in
# `<target>.txt` beside this script (`{base}` is the package's name
# without its extension), then by target:
#
# - windows: the program imports no MinGW runtime DLL, has no `.debug_*`
#   section, and its `.debug` file beside the package has them at the
#   same `.text` address;
# - linux: the same sections, and no `GLIBC_` version above the floor;
# - macos: the `.app` passes `codesign --verify --strict`, is signed
#   ad hoc, and its program has an arm64 and an x86_64 slice.
#
# The exporter itself already refused any path of the build machine.
#
# Usage: bash tools/export-check/check.sh <target> <export folder>

set -euo pipefail

target=$1
out=$2
here=$(cd "$(dirname "$0")" && pwd)
py=$(command -v python3 || command -v python || true)
report="$out/export-report.json"
# A string field of the report, which serde_json writes one to a line;
# no Python needed (the sniper image's is not counted on).
field() { sed -n "s/^  \"$1\": \"\(.*\)\",\{0,1\}\r\{0,1\}$/\1/p" "$report" | head -n 1; }
version=$(field version)
folder=$(field folder)
package=$(field package)
signing=$(field signing)
# The program, from the export folder: the second size the report lists.
program="$out/$(sed -n 's/^      "file": "\(.*\)",\{0,1\}\r\{0,1\}$/\1/p' "$report" | sed -n 2p)"
echo "check: $target $package (version $version, signing $signing)"

# The package's files, by name.
case "$package" in
  *.zip) listed=$("$py" -c "import zipfile,sys; print('\n'.join(zipfile.ZipFile(sys.argv[1]).namelist()))" "$out/$package") ;;
  *.tar.gz) listed=$(tar tzf "$out/$package") ;;
  *) listed=$(cd "$out" && find "$package" -type f | sort) ;;
esac
listed=$(printf '%s\n' "$listed" | tr -d '\r')
expected=$(sed "s/{base}/$folder/" "$here/$target.txt")
if [ "$(printf '%s\n' "$listed" | sort)" != "$(printf '%s\n' "$expected" | sort)" ]; then
  echo "check: the package's files differ from $target.txt:"
  diff <(printf '%s\n' "$expected" | sort) <(printf '%s\n' "$listed" | sort) || true
  exit 1
fi

# The program's sections, and its debug file's.
sections() {
  local program="$1" debug="$out/$folder.debug"
  # `.debug_gdb_scripts` is loaded data rustc writes on ELF (the name of
  # gdb's pretty-printer script, no path): `objcopy --strip-debug` keeps
  # it, as it keeps every section the program loads.
  local left
  left=$(objdump -h "$program" | awk '$2 ~ /^\.debug_/ && $2 != ".debug_gdb_scripts" { print $2 }')
  if [ -n "$left" ]; then
    echo "check: $program has debug sections: $(echo "$left" | tr '\n' ' ')"; exit 1
  fi
  objdump -h "$debug" | grep -q ' \.debug_line' || { echo "check: $debug has no .debug_line"; exit 1; }
  local a b
  a=$(objdump -h "$program" | awk '$2 == ".text" { print $4 }')
  b=$(objdump -h "$debug" | awk '$2 == ".text" { print $4 }')
  [ -n "$a" ] && [ "$a" = "$b" ] || { echo "check: .text at $a, in the debug file at $b"; exit 1; }
  echo "check: no debug sections; $folder.debug has them, .text at $a in both"
}

case "$target" in
  windows)
    sections "$program"
    dlls=$(objdump -p "$program" | awk '/DLL Name:/ { print $3 }' | sort -fu)
    echo "check: imports $(echo "$dlls" | tr '\n' ' ')"
    if echo "$dlls" | grep -qiE '^(libgcc|libwinpthread|libstdc\+\+)'; then
      echo "check: the program needs a MinGW runtime DLL"; exit 1
    fi
    ;;
  linux)
    sections "$program"
    readelf -d "$program" | grep NEEDED
    floor=2.31
    newest=$(objdump -T "$program" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -Vu | tail -n 1)
    echo "check: the newest glibc version it asks for is $newest (the floor: $floor)"
    if [ "$(printf '%s\n%s\n' "$newest" "$floor" | sort -V | tail -n 1)" != "$floor" ]; then
      echo "check: GLIBC_$newest is above the floor"; exit 1
    fi
    ;;
  macos)
    app=$(cd "$out/$folder" && ls -d ./*.app)
    codesign --verify --strict --verbose=2 "$out/$folder/$app"
    codesign -dv "$out/$folder/$app" 2>&1 | grep -q 'Signature=adhoc' || { echo "check: not signed ad hoc"; exit 1; }
    [ "$signing" = "ad-hoc" ] || { echo "check: the report says $signing"; exit 1; }
    archs=$(lipo -archs "$out/$folder/$app/Contents/MacOS/"*)
    echo "check: slices $archs"
    case " $archs " in *" arm64 "*) ;; *) echo "check: no arm64 slice"; exit 1 ;; esac
    case " $archs " in *" x86_64 "*) ;; *) echo "check: no x86_64 slice"; exit 1 ;; esac
    ;;
esac
grep -q 'compiled in by gilrs' "$out/$folder/NOTICES.txt" || [ "$target" = web ] || {
  echo "check: NOTICES.txt lacks the controller database"; exit 1
}
echo "check: $target OK"
