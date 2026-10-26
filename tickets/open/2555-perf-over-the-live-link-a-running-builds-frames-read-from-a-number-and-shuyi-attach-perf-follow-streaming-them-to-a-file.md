---
id: "2555"
title: "Perf over the live link: a running debug build's frames read from a number, shuyi attach --perf-follow streaming them to a file, and the MCP perf tool on an attached session"
type: feature
milestone: MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2521", "2553"]
nick_input: none
completed:
---

# 2555 — Perf over the live link

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md),
"perf traces agents can read"):

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

He put the traces in the MVP, and moved the live link (2521) into the
MVP with them, so an agent reads a running windowed build's trace too.

- **2521** builds the link: JSON lines over TCP on loopback, debug
  builds only, a token, one client at a time. `shuyi_platform::link`
  turns request lines into answer lines over an `App<G>`: `Request`
  (`Hello`, `Status`, `Frame`, `Screenshot`, `Play`, `Log { since }`),
  `Server::handle`, `Handled` (`Answer`, `Later`, `Screenshot`,
  `Close`), and `link::VERSION`. `Log { since }` answers the log's lines
  numbered `since` or more, `next`, and how many of those the ring no
  longer keeps. `shuyi attach` is the contract's operation over it, and
  MCP's `start` with a `link` argument attaches a session.
- **2553** gives the trace: `PerfLine` JSON lines, `PerfRead` (records,
  `next`, `dropped`), `PerfSummary` and `summary(records)`, MCP's `perf`
  tool (`since`, `max`), and `SHUYI_PERF=<file>` in a running desktop
  build. 3063's `App::perf()` is a `PerfRing` numbered like the log.
- **3064** puts heap counts in each record, so they ride along.

This ticket reads the trace over the link the way 2521 reads the log,
and adds a stream: `shuyi attach --perf-follow` writes each new frame to
a file as the build runs, and an agent reads the file as it grows.

## Nick input

None.

## Scope

**In:**
- **The link** (`shuyi_platform::link`):
  - `Request::Perf { since: u64, max: u32 }` (`max` 1 to 600, default
    120), answered at once from `App::perf()`: the records numbered
    `since` or more, the first `max` of them; `next`; and `dropped`,
    counted as 2553's `PerfRead` counts it. The answer carries 3063's
    and 3064's serde types as they are;
  - `link::VERSION` raised by one, so a `shuyi` and a build that differ
    say so (2521's error).
- **`shuyi attach`:**
  - `--perf-since <N>`: the answer (`AttachOutput`) gains `perf` (the
    records), `perf_next` and `perf_summary` (2553's summary over them),
    and the limit 2553 writes when records were dropped;
  - `--perf-follow <file.jsonl>` with `--for <seconds>` (default 10, 1
    to 3,600): after the steps, it asks for `Perf` every 100 ms of wall
    time, from `--perf-since` (or, without it, from the build's `next`
    at that moment), and appends each new record to the file as a
    `PerfLine::Frame` line, flushed after each answer. It stops after
    `--for` seconds or when the build closes the link (it quit), and
    answers with the summary over every record it wrote, the file's
    path and the count of lines. Records dropped between two reads (a
    build faster than the reads) are counted, and a limit says how
    many: "N frame(s) were dropped between reads". A relative path is
    from the folder `shuyi` runs in;
  - `--perf-follow` prints nothing until it answers, so an agent runs it
    in the background and reads the file;
  - `attach.input.json` and `attach.output.json` regenerated, and the
    contract's version raised by one with its line in `versions.json`.
- **MCP:** a session attached over the link (`start` with `link`,
  which 2521's size rule moved to 2560) answers 2553's `perf` tool
  through `Request::Perf`, with the same input and answer as a headless
  session. **From 2560 (done):** the attached session is
  `crates/cli/src/mcp/attached.rs` (its `call`, beside `step` and
  `frame_png`); add `perf` there, with its criterion and a test in
  `mcp/attached/tests.rs` on the test host
  (`shuyi_cli::toy::link::LinkHost`).
- **Tests,** as 2521's are: the server over the toy game on the
  headless backend with 3063's fake clock (`Perf` from 0 gives every
  kept record; `next` gives none until a frame runs; `dropped` past a
  full ring); `shuyi attach --perf-since` and `--perf-follow --for 1`
  against 2521's test host, the follow's file read back as `PerfLine`s.
- **Docs** (the `document` skill): `docs/contract.md`'s *Attaching to a
  running build* (2521's section) and *Perf traces* (2553's): reading a
  running build's trace over the link, and following it into a file.
- **The skill pack:** 2553's `shuyi-perf` skill gains a section, "A
  running build": when to read the window's build rather than a
  headless `run` (a stutter only the real renderer shows, a person
  playing while the agent reads); starting it with the link (`--features
  link`, `SHUYI_LINK=0`, the token from its log); `shuyi attach
  --perf-follow perf.jsonl --for 30` in the background, then reading the
  file as it grows; and the limit on frames dropped between reads. The
  skill's line on `SHUYI_PERF` stays, for a build no agent attaches to.
  Its `metadata.version` is raised. Its second worked example,
  `example-follow.ron`, needs a window, so it is marked manual
  (`manual: Some("needs a window")`), listed and not run, as 2521's
  `example-attach.ron` in `shuyi-test` is.
- **Either order with 2554:** if 2554 is done, `Request::Perf` gains
  `snapshot: bool`: the backend takes a snapshot after the frame, as it
  takes the screenshot after `present`, and answers it with the records;
  `shuyi attach` gains `--perf-snapshot`, which the skill's section
  names; and MCP's `perf` takes
  snapshots and diffs over the link, the session keeping them on the
  `shuyi` side. If not, 2554 adds them (its own clause says so).

**Out (do not do):**
- the records, the trace's lines and the summary (3063, 2553), and the
  heap counts (3064);
- the web build and a device on the LAN: 2522 (after the MVP) carries
  2521's JSON lines unchanged, these among them;
- pushing frames to the client unasked: the link answers requests in
  order (2521's ADR), so following is reading again and again;
- a trace file written by the build itself (2553's `SHUYI_PERF`);
- Chrome trace-event JSON (2325, parked).

## Implementation steps

1. Read 2521's ADR, `shuyi_platform::link`, its `Log { since }`, and
   `shuyi attach`; 2553's `PerfRead`, `PerfSummary`, MCP `perf` tool and
   *Perf traces* docs.
2. Add `Request::Perf`, its answer and its server tests, and raise
   `link::VERSION`.
3. Add `--perf-since` and `--perf-follow` to `shuyi attach`, with tests
   against the test host.
4. Answer MCP's `perf` over the link, with a step in 1133's scripted
   client test that attaches.
5. Regenerate the schemas, raise the version, write the docs and the
   skill's section, run the gates. Follow Hello, Grid for 10 seconds on your desktop with
   `SHUYI_LINK=0` and put three of the file's lines in the PR.

## Acceptance criteria

- [ ] Over the server on the toy game, `Perf { since: 0 }` answers every
      kept record, and `Perf { since: next }` answers none until a frame
      runs, then that frame.
- [ ] `shuyi attach --perf-follow perf.jsonl --for 1` against the test
      host writes lines that parse as `PerfLine`s, one per frame the host
      ran, and answers their summary.
- [ ] A test host that runs more than 600 frames between two reads makes
      the answer's limit count the dropped frames.
- [ ] MCP's `perf` on a session attached to the test host answers as it
      does on a headless session.
- [ ] The schemas are regenerated, `versions.json` has the new version,
      and `link::VERSION` is raised.
- [ ] Hello, Grid followed by hand on a desktop wrote a file while it
      ran (three lines in the PR).
- [ ] `shuyi-perf` has its section on a running build, `cargo shuyi
      check-skills skills` passes, and `cargo xtask skill-examples`
      lists `example-follow.ron` as manual.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Request::Perf` and its answer round trip as JSON lines; `max`
  and `since` past `next`; the follow loop's stop rules (the time, a
  closed link) over a fake connection.
- Integration: the server over the toy with the fake clock; `attach
  --perf-since` and `--perf-follow` against the test host; MCP's `perf`
  attached.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
