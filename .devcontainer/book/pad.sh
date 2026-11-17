#!/usr/bin/env bash
# Starts the book's server, `cargo xtask book-pad` (ticket 2840,
# crates/xtask/src/book_pad.rs), in the background, once: it builds the
# site at the Codespace's forwarded port 4000 and serves it there, each
# step's game from the step's own build, built again when its files change.
# Its log is `~/.book-pad.log`. The book's dev container (`devcontainer.json`
# beside this) runs it as its `onCreateCommand` ends, once step 2 is built,
# and as its `postAttachCommand`. A second start finds the port taken, and
# says the book is already served.
#
#   bash pad.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
log="$HOME/.book-pad.log"

start=$SECONDS
# Built here, not in the background: into the target folder the steps
# share (`CARGO_TARGET_DIR`), a build the reader starts meanwhile waits for.
cargo build -q --bin xtask
echo "pad.sh: xtask built in $((SECONDS - start)) s"

setsid nohup cargo xtask book-pad > "$log" 2>&1 < /dev/null &
pid=$!
sleep 2
if ! kill -0 "$pid" 2> /dev/null; then
  # Stopped already: the port was taken (the book is served), or it failed.
  cat "$log"
  wait "$pid"
  exit
fi
echo "pad.sh: the book is being built, then served on port 4000: the preview opens it, or the Ports tab."
echo "pad.sh: a step is built again when its files change; the log is $log."
