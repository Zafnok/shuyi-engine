---
id: "2044"
title: "The sound gate's Windows and decoder limits calibrated against more runs"
type: bug
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2044 — The sound gate's Windows and decoder limits, calibrated

## Context

Ticket 2042 set four of `tools/sound-parity/budget.ron`'s performance limits
again from 58 CI runs, by the budget's own rule, after limits from three runs
failed pull requests that changed no sound. Its completion notes say how it was
done. The limits that 3031 added for `windows` and `windows-in-house`, and the
`decoders` limits of 2210, still come from three runs.

2042 then ran the `sound-parity` job 20 times on one revision with no sound
change (CI run 41595469804). Beside the in-house bugs (3088, 3089):

- job 12: `windows perf:cpu_ms` failed at a median ratio of 1.086 (limit 1.05),
  and `windows perf:decode_wav_ms` and `decode_ogg_ms` were inconclusive
  (spreads 0.089 and 0.056);
- job 4: `windows perf:latency_ms` inconclusive, spread 0.177 against 0.15;
- job 7: `linux-native decoder-speed:ogg` inconclusive on all three attempts,
  spreads 0.052 to 0.059 against 0.05 (`decoders`, ratios 0.81 to 0.84).

So one more limit in about seven fails a pull request on its own.

## Nick input

None.

## Scope

**In:**
- The `performance` limits of `windows` and `windows-in-house`, and every
  `decoders` ratio limit, over at least twenty runs of the job: the samples are
  in each run's `sound-parity` and `sound-parity-windows` artifacts (2042 gathered
  58 runs' with a scratch workflow that downloads them on a runner).
- Whether a ratio leans one way (the side measured second against the first),
  which would be a bias in the capture to fix rather than to allow for.
- Each new limit by the budget's rule, with the runs named beside it, and the
  budget's `version` raised.

**Out (do not do):**
- loosening a limit with no new runs, or one the runs never exceed;
- re-measuring a `failed` cell (the README, *Performance*);
- `linux-native` and `browser-chromium`'s `performance` limits, which 2042 set;
- the in-house bugs (3088, 3089).

## Implementation steps

1. Read `tools/sound-parity/README.md`, `budget.ron`, 2042's completion notes and
   `crates/xtask/src/sound_parity.rs` (`judge`, `old_new_cells`) and
   `sound_parity/decoders.rs`.
2. Gather the runs' samples since 3031 merged (12e1256e), including the 20 of
   run 41595469804, and run more on a scratch branch if they are fewer than
   twenty.
3. Set the limits the numbers support, and say why in `budget.ron`.
4. Check that the injected slowdown still fails on every metric where it
   `slows`.
5. Run the job ten times on one revision, and keep the result in the notes.
6. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] The calibration's runs and numbers are in the completion notes, with
      whether a ratio leans one way.
- [ ] `budget.ron` says why each changed limit is what it is, naming the runs.
- [ ] The injected slowdown still fails, on every metric where it `slows`.
- [ ] No cell this ticket owns fails in ten runs on one revision.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Existing: `sound_parity`'s tests.
- CI: the ten runs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
