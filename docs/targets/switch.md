# The Nintendo Switch

The Nintendo Switch ([`targets/switch.ron`](../../targets/switch.ron)), a planned target: its ticket, 3014, is parked.

## Budgets

Its row of ADR-0008 §6's table; *prov.* marks a starting guess. What the
budgets are and how `shuyi bench` checks them is in
[`README.md`](README.md).

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `switch` | p99 ≤ 16.7 ms docked and handheld | — (3014) | — (3014) | — | — | — |

## On the device

Not yet measured on a device. Its budgets are 3014's to fill.
