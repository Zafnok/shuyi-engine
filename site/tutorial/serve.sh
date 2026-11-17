#!/usr/bin/env bash
# Serves one step's web build (`<step>/dist/web/`) on port 4000, in the
# book's Codespace (ticket 2837, ADR-0126 §1), which forwards the port,
# private to whoever made the Codespace, and opens it in a preview.
#
#   bash serve.sh            the step this folder is, or the last one served
#   bash serve.sh 03-walk    that step, from now on
#
# The step served is kept in `~/.book-step` (02-board until another is
# named), so each new window of the Codespace serves it again. A server
# started before is stopped first. After an edit, `cargo shuyi web` in the
# step's folder builds it again into the same folder: reload the game's
# tab.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
choice="$HOME/.book-step"
port=4000

if [ $# -gt 0 ]; then
  step="${1%/}"
elif [ "$(dirname "$PWD")" = "$here" ] && [ -f "$PWD/shuyi.ron" ]; then
  step="$(basename "$PWD")"
else
  step="$(cat "$choice" 2> /dev/null || echo 02-board)"
fi

if [ ! -f "$here/$step/shuyi.ron" ]; then
  echo "serve.sh: there is no step \`$step\`; the steps are:" $(cd "$here" && ls -d [0-9][0-9]-*/ | tr -d /) >&2
  exit 1
fi
dist="$here/$step/dist/web"
if [ ! -f "$dist/index.html" ]; then
  echo "serve.sh: $step has no web build yet: run \`cargo shuyi web\` in its folder, then this again" >&2
  exit 1
fi
printf '%s\n' "$step" > "$choice"

pid_file="$HOME/.book-serve.pid"
if [ -f "$pid_file" ]; then
  kill "$(cat "$pid_file")" 2> /dev/null || true
  sleep 0.5
fi
# Python's own server, the image has it, with no caching: a build replaces
# the files, and a reload must fetch them again.
server='
import functools, http.server, sys
class NoCache(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()
handler = functools.partial(NoCache, directory=sys.argv[2])
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), handler).serve_forever()
'
log="$HOME/.book-serve.log"
setsid nohup python3 -c "$server" "$port" "$dist" > "$log" 2>&1 < /dev/null &
echo $! > "$pid_file"
sleep 1
if ! kill -0 "$(cat "$pid_file")" 2> /dev/null; then
  echo "serve.sh: the server stopped as it started:" >&2
  cat "$log" >&2
  exit 1
fi

echo "Serving $step's game on port $port: the Ports tab opens it."
echo "After an edit: \`cargo shuyi web\` in $step/, then reload the game's tab."
echo "Another step: \`bash serve.sh <step>\` here, or \`bash ../serve.sh\` in the step's folder."
