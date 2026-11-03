---
id: "2043"
title: "Mutation-test shuyi-sound's adapters, which a run on one package doesn't build"
type: infra
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: in-progress
blocked_by: []
nick_input: none
completed:
---

# 2043 — Mutation-test `shuyi-sound`'s adapters

## Context

Found by 3031. CI's mutation job (`.github/workflows/mutants.yml`) tests a mutant
with `cargo nextest` on its own package, with that package's default features.
`shuyi-sound`'s `wav.rs` and `vorbis.rs` are built only with its `wav` and `vorbis`
features, off by default since 3030 took the dev-dependency that turned them on so
that a run without `std` is one. So every mutant there is built into nothing and
reported as missed, whatever `tests/adapters.rs` says. 3031 added both files to
`exclude_globs` in `.cargo/mutants.toml`, with the reason, so that a PR touching them
isn't failed for it. Their tests still run, with `--all-features`, in CI's
`sound-parity` job: only the mutation testing of them is gone.

## Nick input

None.

## Scope

**In:**

- Mutation testing that reaches `wav.rs` and `vorbis.rs` with their tests built. One
  way: a second invocation in `mutants.yml` limited to those two files with
  `--features wav,vorbis`, in its own shard, without changing the features of every
  other package's run.
- Take both out of `exclude_globs`. Keep the `VorbisSource::next_packet` exclusion if
  it still costs a shard its timeout.
- Kill or explain what it finds.

**Out (do not do):**

- `--all-features` for the whole run: other crates' features aren't meant to combine.
- Changing the `no-std` run's features, which is why the dev-dependency left.

## Implementation steps

1. Add the invocation, reading `cargo mutants --help` for how features are passed.
2. Run it on a PR that touches `wav.rs`, and read the results.
3. Add tests or reasoned exclusions for each mutant it misses.

## Acceptance criteria

- [ ] A mutant in `crates/sound/src/wav.rs` is caught by `crates/sound/tests/adapters.rs`
      in CI, seen in the job's log.
- [ ] `exclude_globs` no longer names the two files.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None of its own: the mutation job's log is the evidence.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
