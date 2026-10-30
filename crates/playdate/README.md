# `shuyi-playdate`

Shuyi Engine's Playdate backend: `shuyi-platform`'s `Platform` on Panic's
SDK, without `std`. A game names it from its `playdate` feature and says how
it starts with `shuyi_playdate::main!`; `shuyi export playdate` builds and
packs it. How to make a game for it is in
[`docs/targets/playdate.md`](../../docs/targets/playdate.md), and the choices
behind it in ADR-0062.

| Path | What |
| ---- | ---- |
| `shim/shim.c` | The SDK's entry point and one small C wrapper per SDK call, compiled against the installed SDK's `pd_api.h`, whose text the exporter writes beside its objects |
| `src/sdk.rs` | `Sdk`, the SDK's calls as a small trait: the tests' fake (`src/fake.rs`) and the device's real calls implement it |
| `src/backend.rs` | `Playdate`, the `Platform` |
| `src/driver.rs` | `Driver`, the update callback's body around `App::frame`; its tests run the whole loop on the fake |
| `src/input.rs` | The buttons and the crank; the only file that names buttons |
| `src/display.rs` | The frame buffer and its changed rows, the card and the errors |
| `src/storage.rs` | The game's saves on the SDK's files, which pass the power-cut suite over the fake |
| `src/device/` | Only with `--cfg shuyi_playdate_sdk`: `Sdk` on the real SDK, the allocator, the panic handler. The only `unsafe` |

Without that cfg the crate builds on any machine, so the workspace's host
builds and tests never need the SDK. It is a cfg and no cargo feature: a build
with `--all-features` has `std`, whose panic handler the SDK's would clash
with.
