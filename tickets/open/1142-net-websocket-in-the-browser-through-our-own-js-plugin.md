---
id: "1142"
title: "Parked: Networking, step 3: WebSocket in the web build through our own miniquad JS plugin"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1141"]
nick_input: none
completed:
---

# 1142 — WebSocket in the browser through our own JS plugin

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §6. macroquad's web loader doesn't use `wasm-bindgen`
(game ADR-0034), so crates like `ewebsock` and matchbox can't run in our web
build. The pattern that works is the one the controller already uses:
`web/gamepad.js`, a small miniquad plugin. The plugin API is described
at https://macroquad.rs/articles/wasm/.

This ticket writes `web/net.js`. It opens a WebSocket, sends bytes, and
queues received bytes for Rust to poll. Then `WsTransport` works on the
web.

## Nick input

None.

## Scope

**In:**
- `web/net.js`: open, send, close, and poll events (open, message,
  close, error). Binary frames only. A limit on queued bytes.
- Its license header and a line in `THIRD_PARTY_ASSETS.md` if it borrows
  from any upstream file. It doesn't need one if it is all ours.
- The `wasm32` side of `WsTransport` calls the plugin through `extern
  "C"`. The one `unsafe` boundary lives in `shuyi-app` (game ADR-0004
  rule 5), in the same way the pad plugin does it.
- `xtask web` / `shuyi-cli web` includes the plugin.
- A test page in the browser pane: two tabs join the same relay room,
  running locally, and play the toy lockstep game.

**Out (do not do):** WebRTC and WebTransport (1147).

## Implementation steps

1. Write the plugin and the Rust side.
2. Run the local relay (1141), open two tabs in the browser pane, and
   play.
3. Write the docs.

## Acceptance criteria

- [ ] Two browser tabs and a native client play one match through a
      local relay. Screenshots of both tabs go in the PR.
- [ ] The WASM build and all gates pass.

## Tests required

- Manual: browser pane, two tabs plus native.
- Unit: the Rust side's message queue.

## Completion notes
