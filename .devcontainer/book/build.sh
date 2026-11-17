#!/usr/bin/env bash
# Builds the web build of each step named, in `dev`, into the one target
# folder `CARGO_TARGET_DIR` names (ticket 2837, ADR-0126 §1). The steps
# lock the same crates, so the engine is built for the first step and each
# step after it builds only its own crate. The book's dev container
# (`devcontainer.json` beside this) runs it as its `onCreateCommand`, which
# a prebuild runs too; `cargo xtask check-devcontainers` checks it names
# every step of `site/tutorial/`.
#
#   bash build.sh 01-new 02-board ...
set -euo pipefail

cd "$(dirname "$0")/../../site/tutorial"
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
if [ -z "${CARGO_TARGET_DIR:-}" ]; then
  echo "build.sh: CARGO_TARGET_DIR isn't set: the dev container sets it" >&2
  exit 1
fi

all=$SECONDS
for step in "$@"; do
  start=$SECONDS
  (cd "$step" && cargo shuyi web)
  echo "build.sh: $step built in $((SECONDS - start)) s"
done
echo "build.sh: $# steps in $((SECONDS - all)) s; the target folder holds $(du -sh "$CARGO_TARGET_DIR" | cut -f1)"
