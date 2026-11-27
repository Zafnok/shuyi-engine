---
id: "3542"
title: "The preview server's deadline test gives its worker time to start on a loaded machine"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3542 — The preview server's deadline test gives its worker time to start on a loaded machine

## Context

`preview::server::tests::a_worker_that_doesnt_answer_in_time_is_ended_with_its_session`
(`crates/cli/src/preview/server/tests.rs`, about line 197) sets
`Limits::deadline` to 200 ms. Its first worker (`stalled(true)`) must
start within that deadline: `start` (about line 87) asserts a 200 for
`POST /session`, and the worker's thread renders a first frame
(`worker::serve` on `ToyTools`) before it answers. Under a full
`cargo xtask gates` on Nick's machine (24 cores, every test binary at
once), on 2026-11-26 during 3541, that start took longer:

```text
preview: a session didn't start (the game stopped: the game took longer than 200ms): 0 actions, 235 ms of the game's time, 0 KB sent, its memory unread
panicked at crates\cli\src\preview\server\tests.rs:91:5:
assertion `left == right` failed: {"error":"the game stopped: the game took longer than 200ms"}
  left: 500
 right: 200
```

Run alone it passed three times out of three (about 0.4 s each). The
deadline is the one thing the test checks, and the first frame shares it,
so on a loaded machine the test fails for a reason it isn't testing.

## Nick input

None.

## Scope

**In:**
- The test's deadline is wide enough for a loaded machine's first frame
  (2 s, say), with the error text it expects taken from that deadline.
  The stalled worker never answers an action, so the press still
  reaches the deadline and the test checks the same thing; it takes
  about the deadline longer.
- Any other test in `crates/cli/src/preview/server/tests.rs` whose
  first frame shares a deadline under 1 s, the same way.

**Out (do not do):**
- Changing `Limits::default()` or the server's code.
- Retrying a failed test, or marking it `#[ignore]`.

## Implementation steps

1. Read the test and `stalled`, and grep the file for `deadline:`.
2. Widen the deadline(s) in the test only, and build the expected error
   text from the `Duration`, so a later change keeps the two together.
3. Run `cargo test -p shuyi-cli --lib preview::server` and the full
   gates.

## Acceptance criteria

- [ ] The test's first worker starts within its deadline on a machine
      running every test binary at once (a full `cargo xtask gates`).
- [ ] It still fails if a stalled press isn't ended at the deadline.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the changed test itself; nothing new.

## Completion notes
