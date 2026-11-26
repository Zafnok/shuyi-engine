---
id: "2791"
title: "Measure the trails server at ten full rooms: CPU, memory and bytes a player, the numbers for the free host"
type: feature
milestone: "Engine: examples"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2789"]
nick_input: none
completed:
---

# 2791 — Measure the trails server at ten full rooms

## Context

Nick asked (decision of 2026-11-25, `docs/decisions.md`) for a free
host that holds about **10 rooms at any time**, players joining until
each is full. His rule of 2026-11-23 is to measure a server's cost
before hosting it. 2618 hosts trails' server on Oracle Cloud's Always
Free tier, on an A1 if Oracle has capacity and otherwise on an
E2.1.Micro: 1/8 OCPU (burstable), 1 GB of memory, "up to 50 Mbps", and
10 TB of outbound data a month.

The plan's estimates, which this ticket proves or corrects:

- **Bytes:** a `Step` frame is about 12 bytes of payload (2612's byte
  form, 2614), about 112 bytes on the wire with WebSocket, TLS 1.3 and
  TCP/IP overhead, so about 1.7 KB/s a client at 15 steps a second.
  Eighty humans around the clock are about 350 GB a month, 3.6% of
  10 TB; at 10% humans, about 35 GB. Bots cost no traffic.
- **Memory:** under 256 MB for the whole server, well inside the
  Micro's 1 GB.
- **CPU:** integer simulation of 10 rooms at 15 steps a second, plus 80
  TLS sockets, inside 1/8 OCPU.

2611 (open) plans a load test of the hosted preview; this one is
separate and shares nothing it needs. The repository's memory note on
CPU on Windows: process CPU time misreads a capped program, so on
Windows use `QueryProcessCycleTime`.

## Nick input

None.

## Scope

**In:**
- **A load client, `src/bin/trails-load.rs`** in `examples/trails`,
  `required-features = ["load"]` (`load = ["server"]`), so `xtask`
  gains no async runtime:
  - opens N WebSocket clients (default 80: 10 rooms × 8) on one
    single-threaded runtime, each sending `Hello` and `Play`, then turns
    at human rates (a seeded random turn every 0.5–2 s) and `Alive` as
    2614 says;
  - then 20 more clients that must each get `Busy`;
  - counts, per client, the payload bytes received and sent and the
    gap between `Step` frames.
- **`cargo xtask load-trails [--seconds N] [--clients N]`** (default
  120 s, 80): builds `trails-server` and `trails-load` with `--release`,
  starts the server on `127.0.0.1` with a config of `rooms_max: 10`,
  runs the load, and reports:
  - bytes a client a second down and up, as payload and as an
    estimated wire size (payload plus 2 bytes of WebSocket header, 22
    of TLS 1.3 record and 52 of TCP/IPv4 with timestamps, per message;
    the constants named in the code);
  - the server's peak resident memory;
  - the server's CPU seconds per wall second (`getrusage` of the child
    on Linux; `QueryProcessCycleTime` over the CPU's rated frequency on
    Windows), against 0.125;
  - the step lag, p50 and p99: from 2616's `/stats` if it reports one,
    else the gap between `Step` frames at the clients less the step
    period;
  - the refusals seen.
- **Budgets**, asserted (the command fails on any):
  - payload down ≤ 400 bytes a second a client;
  - peak resident memory ≤ 256 MB;
  - p99 step lag under one step (66 ms);
  - all 20 extra clients refused with `Busy`.
- **A workflow, `.github/workflows/load-trails.yml`**, on Linux: weekly
  on a schedule, on `workflow_dispatch`, and on pull requests that touch
  `crates/server`, `crates/net` or `examples/trails`. It runs `cargo
  xtask load-trails`, posts the report to the step summary and keeps it
  as an artifact. `permissions: contents: read`, pinned actions,
  zizmor-clean.
- **A table in `examples/trails/README.md`**, from the CI run, dated:
  the measured numbers; the month's egress at 10% and 100% humans
  against 10 TB; whether 1/8 OCPU suffices; the memory against 1 GB.
- **Never in the local gates.** `cargo xtask gates` doesn't run it, and
  it never points at a server other than the one it starts on
  127.0.0.1.

**Out (do not do):**
- Load-testing the live server: never point this at production.
- Tuning the server beyond fixing a broken budget. A real fix found
  here is its own ticket (the `write-ticket` skill).
- Sharing code with 2611's load test.
- Mutation testing (CI's alone).

**Estimated size:** about 500 lines (code 260, tests 140, CI and docs
100).

## Implementation steps

1. Read 2616's config, `/stats` and limits, 2614's messages, and
   2789's binary and features.
2. `trails-load`: the client pool, the turns, the counters.
3. `cargo xtask load-trails`: build, start, run, measure, report,
   budgets. The report's math in pure functions.
4. The workflow.
5. Run it in CI; write the README table from the run.

## Acceptance criteria

- [ ] `load-trails.yml` passes on a pull request touching
      `examples/trails`, printing every number above, and the README's
      table holds that run's numbers with its date.
- [ ] Each budget, set lower by a test flag, fails the command with a
      message naming the budget and the measure.
- [ ] The 20 extra clients are each refused with `Busy`.
- [ ] `cargo xtask gates` doesn't run the load.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the report's math (wire estimate from payload and message
  counts, percentiles, monthly egress, CPU per wall second) with hand
  computed cases.
- Integration: a 5-second smoke run (`--seconds 5 --clients 16`) in the
  normal test suite on Linux only, with the memory and lag budgets
  relaxed, skipped elsewhere with a reason.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
