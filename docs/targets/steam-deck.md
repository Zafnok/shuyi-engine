# The Steam Deck

Valve's Steam Deck ([`targets/steam-deck.ron`](../../targets/steam-deck.ron)), a Linux PC: a game's `shuyi export linux` package runs there.

## Budgets

Its row of ADR-0008 §6's table; *prov.* marks a starting guess. What the
budgets are and how `shuyi bench` checks them is in
[`README.md`](README.md).

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `steam-deck` | p99 ≤ 16.7 ms, no frame over 33 ms | 20 M (prov.) | 1 GB | — | 3 000 | ≤ 8 W device draw on an idle battle map (about 5 hours on the 40 Wh LCD model; prov.) |

## On the device

Not yet measured on a device. Ticket 3013 measures it on the device.
