#!/usr/bin/env python3
"""Opens a web build in headless Chromium and checks that it runs (ticket 2019).

CI's `web-render` job runs it; `crates/cli/web/README.md`, *Checked in CI*,
says how to run it by hand. It serves a folder on 127.0.0.1, opens its
`index.html`, and records every console message, page error and failed
request (a status of 400 or more counts, `/favicon.ico` aside).

    check.py boot <dir> [--seconds N]   N seconds of frames; fails on a
                                        console error, a page error, a failed
                                        request, or a canvas of one colour
    check.py capture <dir>              a capture build (`shuyi-app`'s
                                        `capture` feature): waits for
                                        `shuyi-capture: done`; fails on a
                                        console error (a failed capture is
                                        one), a page error, a failed
                                        request, or no `done`
    check.py fixtures                   the pages in `fixtures/`, each of
                                        which must fail `boot`, naming why

Each run saves the canvas (`page.png`) and the console (`console.txt`) in
`--out` (`target/web-check/<mode>` unless given). The exit code is 0 when the
page passes and 1 when it fails.
"""

from __future__ import annotations

import argparse
import base64
import functools
import http.server
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path

from playwright.sync_api import Browser, ConsoleMessage, Page, Request, Response, sync_playwright

# Console errors that don't fail a page: the exact text, and why. Empty to
# start; a new error is fixed, or added here with its reason.
ALLOWED_CONSOLE_ERRORS: dict[str, str] = {}

# What a capture build logs when its run is over (`shuyi_app::capture::DONE`).
DONE = "shuyi-capture: done"

# How long `capture` waits for `DONE`, in seconds.
CAPTURE_TIMEOUT = 120

# Hello, Grid's window (`WINDOW` in `examples/hello-grid/src/main.rs`).
DEFAULT_SIZE = "784x784"

# The runners have no GPU: WebGL is drawn by SwiftShader, Chromium's software
# renderer, here and on every machine, so the pixels are the same everywhere.
# Chromium has wanted the second flag for software WebGL since version 137.
CHROMIUM_ARGS = ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"]

# The pages `fixtures` opens, each with the text its failure must name.
FIXTURES = {
    "console-error": "console error: fixture: a console error",
    "missing-wasm": "missing.wasm",
    "one-colour": "the canvas is one colour",
}

# Decodes a PNG in a blank page and counts the colours in it, up to two.
COUNT_COLOURS = """async (url) => {
    const image = new Image();
    image.src = url;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext("2d");
    context.drawImage(image, 0, 0);
    const data = context.getImageData(0, 0, image.width, image.height).data;
    for (let i = 4; i < data.length; i += 4) {
        for (let c = 0; c < 4; c++) {
            if (data[i + c] !== data[c]) return 2;
        }
    }
    return data.length > 0 ? 1 : 0;
}"""


class Handler(http.server.SimpleHTTPRequestHandler):
    """The folder's files, `.wasm` as `application/wasm`, with no log."""

    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".ron": "text/plain",
    }

    def log_message(self, *args: object) -> None:
        """Logs nothing: the page's own record is what counts."""


@dataclass
class Record:
    """What the page did: its console, its errors and its failed requests."""

    console: list[tuple[str, str]] = field(default_factory=list)
    page_errors: list[str] = field(default_factory=list)
    failed_requests: list[str] = field(default_factory=list)

    def on_console(self, message: ConsoleMessage) -> None:
        self.console.append((message.type, message.text))

    def on_page_error(self, error: Exception) -> None:
        self.page_errors.append(str(error))

    def on_request_failed(self, request: Request) -> None:
        if not ignored(request.url):
            self.failed_requests.append(f"{request.url}: {request.failure}")

    def on_response(self, response: Response) -> None:
        if response.status >= 400 and not ignored(response.url):
            self.failed_requests.append(f"{response.url}: status {response.status}")

    def has(self, text: str) -> bool:
        """Whether a console message contains `text`."""
        return any(text in line for _, line in self.console)

    def failures(self) -> list[str]:
        """Why the page fails, the canvas aside: one line each."""
        out = [
            f"console error: {text}"
            for kind, text in self.console
            if kind == "error" and text not in ALLOWED_CONSOLE_ERRORS
        ]
        out += [f"page error: {error}" for error in self.page_errors]
        out += [f"failed request: {request}" for request in self.failed_requests]
        return out

    def text(self) -> str:
        """The record as `console.txt` has it."""
        lines = [f"{kind}: {text}" for kind, text in self.console]
        lines += [f"pageerror: {error}" for error in self.page_errors]
        lines += [f"requestfailed: {request}" for request in self.failed_requests]
        return "".join(f"{line}\n" for line in lines)


def ignored(url: str) -> bool:
    """Whether a request's failure doesn't count: the browser's own icon."""
    return url.split("?")[0].endswith("/favicon.ico")


def serve(folder: Path) -> http.server.ThreadingHTTPServer:
    """A server of `folder` on a free port of 127.0.0.1, already running."""
    handler = functools.partial(Handler, directory=str(folder))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def size(text: str) -> tuple[int, int]:
    """`WxH` as a width and a height."""
    width, _, height = text.partition("x")
    try:
        return int(width), int(height)
    except ValueError:
        raise argparse.ArgumentTypeError(f"{text!r} isn't WxH, such as {DEFAULT_SIZE}") from None


def colours(browser: Browser, png: bytes) -> int:
    """How many colours `png` has: 0 when empty, 1, or 2 for more than one."""
    page = browser.new_page()
    try:
        url = "data:image/png;base64," + base64.b64encode(png).decode("ascii")
        return int(page.evaluate(COUNT_COLOURS, url))
    finally:
        page.close()


def check(
    browser: Browser, mode: str, folder: Path, viewport: tuple[int, int], seconds: float, out: Path
) -> tuple[list[str], Record]:
    """Opens `folder`'s page as `mode` says: why it fails, if it does, and its record."""
    record = Record()
    server = serve(folder)
    context = browser.new_context(
        viewport={"width": viewport[0], "height": viewport[1]},
        device_scale_factor=1,
    )
    try:
        page = context.new_page()
        page.on("console", record.on_console)
        page.on("pageerror", record.on_page_error)
        page.on("requestfailed", record.on_request_failed)
        page.on("response", record.on_response)
        host, port = server.server_address[:2]
        page.goto(f"http://{host}:{port}/index.html", wait_until="load")
        failures = run(page, mode, record, seconds)
        failures = record.failures() + failures
        png = save(page, record, out)
        if mode == "boot" and png is not None and colours(browser, png) < 2:
            failures.append("the canvas is one colour")
        if png is None:
            failures.append("the page has no canvas")
        return failures, record
    finally:
        context.close()
        server.shutdown()
        server.server_close()


def run(page: Page, mode: str, record: Record, seconds: float) -> list[str]:
    """Lets the page run as `mode` says, and gives why it fails, if it does."""
    if mode == "boot":
        page.wait_for_timeout(seconds * 1000)
        return []
    deadline = time.monotonic() + CAPTURE_TIMEOUT
    while not record.has(DONE):
        if time.monotonic() > deadline:
            return [f"no {DONE!r} in {CAPTURE_TIMEOUT} seconds"]
        page.wait_for_timeout(250)
    return []


def save(page: Page, record: Record, out: Path) -> bytes | None:
    """Writes `console.txt` and the canvas's `page.png` into `out`, and gives the picture."""
    out.mkdir(parents=True, exist_ok=True)
    (out / "console.txt").write_text(record.text(), encoding="utf-8", newline="\n")
    canvas = page.query_selector("canvas")
    if canvas is None:
        return None
    png = canvas.screenshot()
    (out / "page.png").write_bytes(png)
    return png


def report(name: str, failures: list[str], out: Path) -> bool:
    """Prints whether the page passed and why not, and gives whether it did."""
    if not failures:
        print(f"{name}: passed ({out})")
        return True
    print(f"{name}: FAILED ({out})")
    for failure in failures:
        print(f"  {failure}")
    return False


def fixtures(browser: Browser, out: Path) -> bool:
    """Checks that each page in `fixtures/` fails `boot`, naming why."""
    here = Path(__file__).resolve().parent / "fixtures"
    passed = True
    for name, reason in FIXTURES.items():
        failures, _ = check(browser, "boot", here / name, size(DEFAULT_SIZE), 1, out / name)
        if any(reason in failure for failure in failures):
            print(f"{name}: failed as it should: {reason}")
        else:
            print(f"{name}: should fail naming {reason!r}, and gave {failures}")
            passed = False
    return passed


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("mode", choices=["boot", "capture", "fixtures"])
    parser.add_argument("dir", nargs="?", type=Path, help="the folder of the page (index.html)")
    parser.add_argument("--seconds", type=float, default=5, help="how long boot runs (5 unless given)")
    parser.add_argument("--size", type=size, default=size(DEFAULT_SIZE), help=f"the page's size, WxH ({DEFAULT_SIZE} unless given)")
    parser.add_argument("--out", type=Path, help="where page.png and console.txt go")
    args = parser.parse_args()
    # The capture lines have `×` in them, whatever the terminal's code page.
    sys.stdout.reconfigure(encoding="utf-8")
    if (args.dir is None) != (args.mode == "fixtures"):
        parser.error("boot and capture take a folder; fixtures takes none")
    out = args.out or Path("target/web-check") / args.mode
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(args=CHROMIUM_ARGS)
        print(f"Chromium {browser.version}, {' '.join(CHROMIUM_ARGS)}")
        try:
            if args.mode == "fixtures":
                return 0 if fixtures(browser, out) else 1
            failures, record = check(browser, args.mode, args.dir, args.size, args.seconds, out)
            for _, text in record.console:
                if "shuyi-capture: " in text:
                    print(text)
            return 0 if report(f"{args.mode} {args.dir}", failures, out) else 1
        finally:
            browser.close()


if __name__ == "__main__":
    sys.exit(main())
