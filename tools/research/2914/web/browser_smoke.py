#!/usr/bin/env python3
"""Compare the Workshop HTML/Wasm replay with native JSON in Chromium.

Usage: browser_smoke.py <wasm-path> <native-fixture-path>
Uses the same pinned Python Playwright environment as CI's web-render job.
"""

from __future__ import annotations

import argparse
import base64
import functools
import http.server
from pathlib import Path
import subprocess
import threading
from urllib.parse import urlencode

from playwright.sync_api import sync_playwright


ROOT = Path(__file__).resolve().parents[4]
WIN = (
    "gather", "gather", "refine", "craft", "deliver", "gather",
    "rest", "gather", "refine", "craft", "deliver", "invalid", "rest",
)
CASES = (
    ("11", WIN),
    ("0", (*("rest" for _ in range(16)), "gather")),
    ("18446744073709551615", ("craft", "refine", "deliver", "state", "gather")),
)


class Handler(http.server.SimpleHTTPRequestHandler):
    """Serve the repository's HTML, modules and Wasm with correct MIME types."""

    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".mjs": "text/javascript",
        ".wasm": "application/wasm",
    }

    def log_message(self, *args: object) -> None:
        """Keep passing checks quiet; a browser error is reported directly."""


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wasm", type=Path)
    parser.add_argument("native", type=Path)
    args = parser.parse_args()
    wasm = args.wasm.resolve(strict=True)
    native = args.native.resolve(strict=True)
    try:
        wasm_url = "/" + wasm.relative_to(ROOT).as_posix()
    except ValueError:
        parser.error("Wasm path must be inside the repository for the HTTP check")

    handler = functools.partial(Handler, directory=str(ROOT))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    host, port = server.server_address[:2]
    errors: list[str] = []
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                args=["--use-angle=swiftshader", "--enable-unsafe-swiftshader"]
            )
            try:
                page = browser.new_page()
                page.on("pageerror", lambda error: errors.append(f"page error: {error}"))
                page.on(
                    "console",
                    lambda message: errors.append(f"console error: {message.text}")
                    if message.type == "error" else None,
                )
                page.on(
                    "response",
                    lambda response: errors.append(f"HTTP {response.status}: {response.url}")
                    if response.status >= 400 and not response.url.endswith("/favicon.ico") else None,
                )
                for seed, actions in CASES:
                    expected = subprocess.run(
                        [str(native), "workshop", seed, ",".join(actions)],
                        cwd=ROOT,
                        check=True,
                        capture_output=True,
                        text=True,
                        timeout=30,
                    ).stdout.strip()
                    query = urlencode({
                        "wasm": wasm_url,
                        "seed": seed,
                        "actions": ",".join(actions),
                        "smoke": "1",
                    })
                    page.goto(
                        f"http://{host}:{port}/tools/research/2914/web/index.html?{query}",
                        wait_until="load",
                    )
                    try:
                        page.wait_for_function(
                            'document.querySelector("#output")?.dataset.replayBase64',
                            timeout=15_000,
                        )
                    except Exception as error:
                        status = page.locator("#status").inner_text()
                        output = page.locator("#output").inner_text()
                        raise AssertionError(
                            f"browser replay did not finish for seed {seed}: {status}; {output}"
                        ) from error
                    encoded = page.locator("#output").get_attribute("data-replay-base64")
                    if encoded is None:
                        raise AssertionError(f"missing replay payload for seed {seed}")
                    actual = base64.b64decode(encoded).decode("utf-8")
                    if actual != expected:
                        raise AssertionError(f"browser/native JSON differs for seed {seed}")
                if errors:
                    raise AssertionError("; ".join(errors))
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()
    print(f"Chromium replay matched native JSON for {len(CASES)} Workshop cases.")


if __name__ == "__main__":
    main()
