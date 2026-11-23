---
id: "2602"
title: "Host a turn-game preview as bounded actions and pixels, built and tested locally: its worker"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: max
status: done
blocked_by: []
nick_input: none
completed: 2026-11-23
---

# 2602 — Host a turn-game preview as actions and pixels: its worker

> **Unparked** by Nick on 2026-11-23 ([`decisions.md`](../../docs/decisions.md#2026-11-23-the-hosted-preview-2602-unparked-built-and-tested-locally-only)): it runs locally and in CI only; no paid hosting until Nick approves its measured cost. The rest of networking stays parked.

## Context

A client-side encrypted pack eventually exposes its decoded content. A hosted preview can keep the executable, source art and structured scene off a visitor's device while still accepting play. Shuyi's headless session can produce a picture, but its CLI/MCP host also exposes snapshots, scenes and logs and accepts scripts; it is not a safe public protocol. See ADR-0006 for the broader networking plan.

**From 3302 (done):** closed builds' copies carry a recipient's code and a signed ledger ([ADR-0118](../../docs/adr/0118-a-closed-builds-recipient-overlay-three-marks-a-signed-ledger-never-proof.md)); a hosted preview's marks are 3083's, after this ticket.

## Nick input

None.

## Scope

**In:**
- A worker per visitor's session: `preview-worker`, a command of the
  game's tool program, one process a session, its options the server's
  (the game's start options with its seed, the most frames an action
  runs, the pictures' scale).
- Only the player's declared actions: those a player can rebind, with the
  developer's debug tools off; anything else, a script or a frame count
  among them, refused.
- One bounded result frame an action, as a PNG with the text it shows,
  without the scene's data, the snapshot's colours or its overlays' files.

**Out (do not do):**
- Exposing the existing MCP/Session API to visitors.
- Real-time video/audio streaming, matchmaking or a hosted Shuyi business.
- A claim that visible gameplay cannot be copied.
- Paid hosting of any kind: no rented server, hosting account or public
  deployment. The service runs on the developer's machine and in CI
  only, until Nick approves the cost this ticket measures.
- The server's action API, deadlines and expiry (2609); its quotas
  (2610); the page, the load test, the accessibility review and the cost
  (2611).

**Estimated size:** about 1,600 lines (code 900, tests 500, docs 200).

**Split:** past 1,000 lines, split off step 3 (the browser client, the
load test and the accessibility review) into a new ticket after this
one, and finish here with the worker, its action API and its quotas.
Done further: the worker here, then 2609, 2610 and 2611 (the notes say
why).

## Implementation steps

1. Wrap a headless game session in an isolated worker; accept only declared player actions.
2. Render one bounded result frame, enforce quotas and expire idle sessions. The frame is here; the rest is 2609's and 2610's.
3. Make a thin browser client and load-test it; check that no game Wasm, pack, source or structured state endpoint is public: 2611.

## Acceptance criteria

- [x] A visitor's game runs in a worker process of its own (`preview-worker`, the game's tool program), from the game's options alone, and ends when its input closes.
- [x] It presses only the player's actions: the developer's Debug key and an unknown action are refused, and a request is an action alone, never a script or a frame count.
- [x] Each action answers one frame, after at most `--frames` frames, as a PNG and the text it shows, and no scene, snapshot, log or file.
- [x] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [x] All gates in the run-gates skill pass.

The criteria this ticket started with moved with its steps: an action API
with no endpoint but its own, and expired sessions (2609); the quotas
(2610); a visitor playing the fixture without downloading it, the p95 and
per-session measurements, the monthly estimate for Nick, and the
keyboard and screen-reader paths (2611).

## Tests required

- Integration: allowed action, rejected unknown action, quotas, expired session and no structured endpoint. Here: the allowed and refused actions, and a request that isn't one, on the toy and on Hello, Grid's real worker process; quotas, expiry and endpoints are 2609's and 2610's.
- Load: bounded concurrent sessions with recorded resource use: 2611.
- Accessibility: keyboard and screen-reader fixture review: 2611.

## Completion notes

**What was done.** `shuyi_cli::preview::worker` is the game's side of the
hosted preview ([ADR-0141](../../docs/adr/0141-the-hosted-preview-a-worker-per-visitor-actions-in-pictures-out.md)):
`preview-worker [--frames N] [--scale N] [the game's options]`, run in a
game's tool program, starts the game's session, makes it a visitor's with
the new `Session::for_visitor` (the actions a player can rebind, the
debug tools off; `Headless` implements it, its default is `None`), and
answers each `{"action": "<name>"}` line on stdin with the frame after it
(`png`, `text`, `actions`, `ended`) after the mark
`shuyi-preview-reply `. After an action it runs frames until the glyphs
and the overlays' places hold still for 30 frames, at most `--frames`
(300). The text is the frame's glyph rows, trimmed. The toy's session now
paints its frame, for the worker's tests. Tested on the toy (actions,
refusals, settling, quitting, the line protocol) and on Hello, Grid's
tool program as a real process through `shuyi preview-worker`.

**Deviations.**
- **Split three more ways.** The worker, the HTTP action API and its
  quotas, written and passing together, measured 2,399 counted lines
  before the ADR and docs, past the 1,500 limit, so only the worker lands
  here. The action API, deadlines and expiry are 2609; the quotas 2610;
  the page, the load test, the accessibility review and the estimate for
  Nick 2611. That code is kept on the branch `draft/2602-preview-server`
  (commit `0ea6a442`), which 2609 and 2610 start from, and 2610 deletes.
- **How "declared player actions" are told** wasn't said: the actions a
  player can rebind, since an action no player can rebind is a
  developer's key, with the debug tools off as well
  (`Session::for_visitor`, ADR-0141 §2). A debug build of a tool program
  turns them on, and the Debug action opens the scene camera and the debug
  view.
- **The frame waits for its glyphs and overlays to hold still**, not the
  whole snapshot: Hello, Grid's grid cursor pulses every frame, so a whole
  snapshot never held still and every action ran its 300 frames (about
  200 ms in a debug build); now an action takes 35 to 75 ms.

**Measured so far** (Hello, Grid's stage, a debug build of its tool
program, Nick's machine, scale 1): 35 to 75 ms from an action to its
picture, about 6 KB a picture (384 × 384); a worker's first start about
2.8 s on the program's first launch, about 0.2 s after. 2611 measures it
properly.

**Follow-up tickets:** 2609, 2610 and 2611, each 2602's work and
unparked with it. 3083 and 2603 now wait for 2611 too, and say what this
gives them.

Book: none, the worker serves the hosted preview's server, which isn't
built yet; no game calls `Session::for_visitor`, and a game whose tools
give a `Headless` has it already. The book's passage comes with the page
a visitor plays (2611).

No break: `Session::for_visitor` is a new method with a default. Nothing
in the game must change: it pins engine `50472bbc`, and `gh search code
--repo Zafnok/visions-of-shuyi` finds no `impl Session for`, no
`for_visitor` and no `preview-worker`.
