# The web build

the browser build ([`targets/web.ron`](../../targets/web.ron)): a game's `shuyi export web` package, a zip with `index.html` at its root, run in a browser. How the page is made and checked is in [`crates/cli/web/README.md`](../../crates/cli/web/README.md).

## Budgets

Its row of ADR-0008 §6's table; *prov.* marks a starting guess. What the
budgets are and how `shuyi bench` checks them is in
[`README.md`](README.md).

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `web` | p99 ≤ 16.7 ms in Chrome on a mid-range laptop | 20 M (prov.) | 256 MB | 25 MB download (prov.) | 5 000 at 25 Mbit/s | — |

## On the device

Not yet measured on a device. No device ticket covers the web yet.
