# The web build

the browser build ([`targets/web.ron`](../../targets/web.ron)): a game's `shuyi export web` package, a zip with `index.html` at its root, run in a browser. How the page is made and checked is in [`crates/cli/web/README.md`](../../crates/cli/web/README.md).

## Budgets

Its row of ADR-0008 §6's table; *prov.* marks a starting guess. What the
budgets are and how `shuyi bench` checks them is in
[`README.md`](README.md).

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `web` | p99 ≤ 16.7 ms in Chrome on a mid-range laptop | 20 M (prov.) | 256 MB | 25 MB download (prov.) | 5 000 at 25 Mbit/s | — |

## Hello, Grid's numbers

From CI's `bench` job on 2026-11-05 (ticket 3008's pull request): the
heaviest of Hello, Grid's eight scenarios
([`examples/hello-grid/bench/`](../../examples/hello-grid/bench/)), on an
x86-64 Linux runner. They are proxies, not a device's frame times, and
the budgets above stay as they are until a device is measured.

| Measure | Hello, Grid | Budget |
| ------- | ----------- | ------ |
| Instructions in its heaviest frame | 394,501 (`idle-map-tiles`, the map drawn with its tileset) | 20,000,000 |
| Heap's peak | 158.5 KB (`idle-map-tiles`) | 256 MB |
| Package (the zip, `hello-grid-0.1.0-web.zip`) | 993.4 KB | 25 MB |

The instructions are a native program's: the browser's WebAssembly runs
more of them, which only a measurement in a browser will say.

## On the device

Not yet measured on a device. No device ticket covers the web yet.
