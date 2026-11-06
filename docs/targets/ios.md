# iOS

iPhones ([`targets/ios.ron`](../../targets/ios.ron)), a planned target: its ticket, 3015, is parked.

## Budgets

Its row of ADR-0008 §6's table; *prov.* marks a starting guess. What the
budgets are and how `shuyi bench` checks them is in
[`README.md`](README.md).

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `ios` | p99 ≤ 16.7 ms on a four-year-old mid-range phone | 20 M (prov.) | 300 MB | 150 MB | 3 000 cold | — |

## On the device

Not yet measured on a device. Ticket 3015 measures it on a device.
